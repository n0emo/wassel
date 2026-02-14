use std::{collections::HashMap, fs, ops::DerefMut as _, path::Path};

use anyhow::Context;
use hyper::{Request, Response, body::Incoming};
use serde::Deserialize;
use tokio::sync::{Mutex, MutexGuard};
use wasmtime::{
    Engine, Store,
    component::{Component, HasSelf, Instance, InstancePre},
};
use wasmtime_wasi_config::{WasiConfig, WasiConfigVariables};
use wasmtime_wasi_http::{
    WasiHttpView as _, bindings::http::types::Scheme, body::HyperOutgoingBody,
};

use crate::{
    config::Config,
    plugin::bindings::wassel::foundation::http_client::{self},
};

use super::{PluginHandleError, bindings, state::State};

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
            .unwrap_or_else(|| HashMap::from_iter([("base_url".to_owned(), "/".to_owned())]));

        let mut linker = wasmtime::component::Linker::<State>::new(engine);

        http_client::add_to_linker::<_, HasSelf<State>>(&mut linker, |s| s)
            .context("Could not add wassel:foundation/http-client to linker")?;

        wasmtime_wasi::p2::add_to_linker_async(&mut linker)
            .context("Adding WASIp2 exports to linker")?;
        wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)
            .context("Adding WASI HTTP tp linker")?;
        wasmtime_wasi_config::add_to_linker(&mut linker, |c| WasiConfig::from(&c.config_vars))
            .context("Adding WASI config to linker")?;

        let export = "wassel:foundation/http-handler";
        if component.get_export(None, export).is_none() {
            anyhow::bail!("There is no '{export}' export");
        }

        let pre = linker
            .instantiate_pre(&component)
            .context("Pre-instantiating plugin")?;

        let image = Self {
            _component: component,
            pre,
            meta,
            config: plugin_config,
        };

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
        })
    }

    pub fn id(&self) -> &str {
        &self.meta.id
    }

    pub fn config(&self) -> &HashMap<String, String> {
        &self.config
    }
}

pub struct HttpPlugin {
    instance: Instance,
    store: Mutex<Store<State>>,
}

impl HttpPlugin {
    pub async fn handle(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<HyperOutgoingBody>, PluginHandleError> {
        let (sender, reciever) = tokio::sync::oneshot::channel();

        let mut store_guard = self.store.lock().await;
        let mut store = MutexGuard::deref_mut(&mut store_guard);

        let req = store
            .data_mut()
            .new_incoming_request(Scheme::Http, req)
            .map_err(PluginHandleError::CreateResource)?;

        let out = store
            .data_mut()
            .new_response_outparam(sender)
            .map_err(PluginHandleError::CreateResource)?;

        let proxy = bindings::HttpPlugin::new(&mut store, &self.instance)
            .map_err(PluginHandleError::Guest)?;

        proxy
            .wassel_foundation_http_handler()
            .call_handle_request(&mut store, req, out)
            .await
            .map_err(PluginHandleError::CallingHandleMethod)?;

        let response = reciever.await??;

        Ok(response)
    }
}
