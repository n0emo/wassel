use std::ops::DerefMut as _;

use bindings::exports::wassel::plugin::http_plugin;
use hyper::{body::Incoming, Request, Response};
use tokio::sync::{Mutex, MutexGuard};
use wasmtime::{
    Engine, Store,
    component::{Component, Instance, Linker},
};

mod bindings;
mod pool;
mod state;

use state::State;
pub use pool::{PluginPool, PoolConfig};
use wasmtime_wasi_http::{bindings::http::types::Scheme, body::HyperOutgoingBody, WasiHttpView};

#[allow(unused)]
pub struct HttpPlugin {
    instance: Instance,
    component: Component,
    store: Mutex<Store<State>>,
    bindings: bindings::Plugin,
    descriptor: http_plugin::Plugin,
    handler_map: matchit::Router<http_plugin::Handler>,
}

impl HttpPlugin {
    pub async fn load(
        bytes: &[u8],
        engine: &Engine,
        linker: &mut Linker<State>,
    ) -> anyhow::Result<Self> {
        let component = Component::new(engine, bytes)?;

        let export = "wassel:plugin/http-plugin";
        if component.get_export(None, export).is_none() {
            anyhow::bail!("There is no '{export}' export");
        }

        let mut store = wasmtime::Store::new(engine, State::default());
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let bindings = bindings::Plugin::new(&mut store, &instance)?;

        let descriptor = bindings
            .wassel_plugin_http_plugin()
            .call_instantiate(&mut store)
            .await?;

        let mut handler_map = matchit::Router::new();
        for endpoint in &descriptor.endpoints {
            handler_map.insert(&endpoint.path, endpoint.handler)?;
        }

        Ok(Self {
            instance,
            component,
            store: Mutex::new(store),
            descriptor,
            bindings,
            handler_map,
        })
    }

    pub async fn handle(&self, req: Request<Incoming>) -> Response<HyperOutgoingBody> {
        let (sender, reciever) = tokio::sync::oneshot::channel();
        let route = req.uri().path();


        let mut store_guard = self.store.lock().await;
        let mut store = MutexGuard::deref_mut(&mut store_guard);
        let handler = *self
            .handler_map
            .at(route)
            .unwrap_or_else(|_| panic!("No handler for route {route}"))
            .value;

        let req = store.data_mut().new_incoming_request(Scheme::Http, req).unwrap();
        #[allow(unreachable_code)]
        let out = store.data_mut().new_response_outparam(sender).unwrap();

        self.bindings
            .wassel_plugin_http_plugin()
            .handler()
            .call_handle(&mut store, handler, req, out)
            .await
            .unwrap();

        reciever.await.unwrap().unwrap()
    }

    pub fn endpoints(&self) -> impl Iterator<Item = &str> {
        self.descriptor.endpoints.iter().map(|e| e.path.as_str())
    }

    pub fn name(&self) -> &str {
        &self.descriptor.name
    }
}
