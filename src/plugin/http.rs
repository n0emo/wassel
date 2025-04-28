use std::ops::DerefMut as _;

use bindings::exports::wassel::plugin::http_plugin;
use hyper::{Request, Response, body::Incoming};
use tokio::sync::{Mutex, MutexGuard};
use wasmtime::{
    Engine, Store,
    component::{Component, Instance, InstancePre, Linker},
};
use wasmtime_wasi_http::{
    WasiHttpView as _, bindings::http::types::Scheme, body::HyperOutgoingBody,
};

use super::{PluginHandleError, bindings, state::State};

pub struct HttpPluginImage {
    _component: Component,
    pre: InstancePre<State>,
}

impl HttpPluginImage {
    pub fn new(component: Component, pre: InstancePre<State>) -> Self {
        Self {
            _component: component,
            pre,
        }
    }

    pub fn load(bytes: &[u8], engine: &Engine, linker: &mut Linker<State>) -> anyhow::Result<Self> {
        let component = Component::new(engine, bytes)?;

        let export = "wassel:plugin/http-plugin";
        if component.get_export(None, export).is_none() {
            anyhow::bail!("There is no '{export}' export");
        }

        let pre = linker.instantiate_pre(&component)?;

        Ok(Self::new(component, pre))
    }

    pub async fn instantiate(&self, engine: &Engine) -> anyhow::Result<HttpPlugin> {
        let mut store = wasmtime::Store::new(engine, State::default());
        let instance = self.pre.instantiate_async(&mut store).await?;
        let bindings = bindings::Exports::new(&mut store, &instance)?;

        let descriptor = bindings
            .wassel_plugin_http_plugin()
            .call_instantiate(&mut store)
            .await?;

        let mut handler_map = matchit::Router::new();
        for endpoint in &descriptor.endpoints {
            handler_map.insert(&endpoint.path, endpoint.handler)?;
        }

        Ok(HttpPlugin {
            _instance: instance,
            store: Mutex::new(store),
            descriptor,
            proxy: bindings,
            handler_map,
        })
    }
}

pub struct HttpPlugin {
    _instance: Instance,
    store: Mutex<Store<State>>,
    proxy: bindings::Exports,
    descriptor: http_plugin::Plugin,
    handler_map: matchit::Router<http_plugin::Handler>,
}

impl HttpPlugin {
    pub async fn handle(
        &self,
        req: Request<Incoming>,
    ) -> Result<Response<HyperOutgoingBody>, PluginHandleError> {
        let (sender, reciever) = tokio::sync::oneshot::channel();
        let route = req.uri().path();

        let mut store_guard = self.store.lock().await;
        let mut store = MutexGuard::deref_mut(&mut store_guard);
        let handler = *self
            .handler_map
            .at(route)
            .map_err(|_| PluginHandleError::EndpointNotFound(route.to_owned()))?
            .value;

        let req = store
            .data_mut()
            .new_incoming_request(Scheme::Http, req)
            .map_err(PluginHandleError::CreateResource)?;

        let out = store
            .data_mut()
            .new_response_outparam(sender)
            .map_err(PluginHandleError::CreateResource)?;

        self.proxy
            .wassel_plugin_http_plugin()
            .handler()
            .call_handle(&mut store, handler, req, out)
            .await
            .map_err(PluginHandleError::CallingHandleMethod)?;

        let response = reciever.await??;

        Ok(response)
    }

    pub fn endpoints(&self) -> impl Iterator<Item = &str> {
        self.descriptor.endpoints.iter().map(|e| e.path.as_str())
    }

    pub fn name(&self) -> &str {
        &self.descriptor.name
    }
}
