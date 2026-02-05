use std::{collections::HashMap, fs, ops::DerefMut as _, path::Path, sync::Arc};

use anyhow::Context;
use hyper::{Request, Response, body::Incoming};
use matchit::Router;
use serde::Deserialize;
use tokio::sync::{Mutex, MutexGuard};
use tracing::error;
use wasmtime::{
    Engine, Store,
    component::{Component, Instance, InstancePre},
};
use wasmtime_wasi_config::{WasiConfig, WasiConfigVariables};
use wasmtime_wasi_http::{
    WasiHttpView as _, bindings::http::types::Scheme, body::HyperOutgoingBody,
};

use crate::config::Config;

use super::{
    PluginHandleError,
    bindings::{self, exports::wassel::plugin::http_plugin},
    state::State,
};

#[derive(Debug, Clone, Deserialize)]
pub struct HttpPluginMeta {
    id: String,
    #[allow(unused)]
    name: String,
    #[allow(unused)]
    version: String,
    #[allow(unused)]
    description: Option<String>,
}

pub struct HttpPluginImage {
    _component: Component,
    pre: InstancePre<State>,
    #[allow(unused)]
    meta: HttpPluginMeta,
    paths: Vec<String>,
    router: Arc<Router<String>>,
    config: HashMap<String, String>,
}

impl HttpPluginImage {
    pub async fn load(directory: &Path, engine: &Engine, config: &Config) -> anyhow::Result<Self> {
        let meta = fs::read_to_string(directory.join("plugin.toml"))
            .context("Joining directory path with `plugin.toml`")?;
        let meta: HttpPluginMeta = toml::from_str(&meta).context("Parsing plugin.toml")?;

        let bytes = fs::read(directory.join("plugin.wasm"))
            .context("Joining directory path with `plugin.wasm`")?;

        let component = Component::new(engine, bytes).context("Creating WASM component")?;

        let plugin_config = config
            .plugins
            .get(&meta.id)
            .cloned()
            .unwrap_or_else(|| HashMap::from_iter([("base_url".to_owned(), "".to_owned())]));

        let mut linker = wasmtime::component::Linker::<State>::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)
            .context("Adding WASIp2 exports to linker")?;
        wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)
            .context("Adding WASI HTTP tp linker")?;
        wasmtime_wasi_config::add_to_linker(&mut linker, |c| WasiConfig::from(&c.config_vars))
            .context("Adding WASI config to linker")?;

        let export = "wassel:plugin/http-plugin";
        if component.get_export(None, export).is_none() {
            anyhow::bail!("There is no '{export}' export");
        }

        let pre = linker
            .instantiate_pre(&component)
            .context("Pre-instantiating plugin")?;
        let mut image = Self {
            _component: component,
            pre,
            meta,
            paths: Vec::new(),
            router: Arc::default(),
            config: plugin_config,
        };
        let instance = image
            .instantiate(&engine)
            .await
            .context("Instantiating plugin")?;

        let proxy =
            bindings::Exports::new(instance.store.lock().await.deref_mut(), &instance.instance)?;
        let endpoints = proxy
            .wassel_plugin_http_plugin()
            .call_get_endpoints(instance.store.lock().await.deref_mut())
            .await
            .context("Getting plugin endpoints")?;

        let base_url = &image.config["base_url"];
        let mut router = Router::new();
        for endpoint in endpoints {
            let http_plugin::Endpoint { path, handler } = endpoint;
            if !path.starts_with("/") {
                error!(
                    "Error loading plugin: incorrect endpoint path `{path}`: paths must start with /"
                );
                continue;
            }

            let path = format!("{base_url}{path}");
            router
                .insert(path.clone(), handler)
                .context("Inserting plugin path into router")?;
            image.paths.push(path);
        }

        image.router = Arc::new(router);

        Ok(image)
    }

    pub async fn instantiate(&self, engine: &Engine) -> anyhow::Result<HttpPlugin> {
        let mut store = wasmtime::Store::new(
            engine,
            State {
                config_vars: WasiConfigVariables::from_iter(self.config.iter()),
                ..Default::default()
            },
        );

        let instance = self.pre.instantiate_async(&mut store).await?;

        Ok(HttpPlugin {
            instance,
            store: Mutex::new(store),
            router: self.router.clone(),
        })
    }

    pub fn id(&self) -> &str {
        &self.meta.id
    }

    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.paths.iter().map(|s| s.as_str())
    }
}

pub struct HttpPlugin {
    instance: Instance,
    store: Mutex<Store<State>>,
    router: Arc<Router<String>>,
}

impl HttpPlugin {
    pub async fn handle(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<HyperOutgoingBody>, PluginHandleError> {
        let (sender, reciever) = tokio::sync::oneshot::channel();
        let route = req.uri().path().to_owned();

        let mut store_guard = self.store.lock().await;
        let mut store = MutexGuard::deref_mut(&mut store_guard);
        let handler = self
            .router
            .at(&route)
            .map_err(|_| PluginHandleError::EndpointNotFound(route.clone()))?
            .value;

        let req = store
            .data_mut()
            .new_incoming_request(Scheme::Http, req)
            .map_err(PluginHandleError::CreateResource)?;

        let out = store
            .data_mut()
            .new_response_outparam(sender)
            .map_err(PluginHandleError::CreateResource)?;

        let handler = self
            .instance
            .get_typed_func::<(
                wasmtime::component::Resource<wasmtime_wasi_http::types::HostIncomingRequest>,
                wasmtime::component::Resource<wasmtime_wasi_http::types::HostResponseOutparam>,
            ), ()>(&mut store, handler)
            .map_err(|e| PluginHandleError::GettingHandlerExport {
                path: route.to_owned(),
                handler: handler.clone(),
                error: e,
            })?;

        handler
            .call_async(&mut store, (req, out))
            .await
            .map_err(|e| PluginHandleError::CallingHandleMethod(e))?;
        handler
            .post_return_async(&mut store)
            .await
            .map_err(|e| PluginHandleError::CallingHandleMethod(e))?;

        let response = reciever.await??;

        Ok(response)
    }
}
