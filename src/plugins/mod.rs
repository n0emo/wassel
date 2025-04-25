use std::ops::DerefMut as _;

use bindings::exports::wassel::plugin::http_plugin;
use tokio::sync::{Mutex, MutexGuard};
use wasmtime::{component::{Component, Instance, Linker}, Engine, Store};

use state::State;

mod bindings;
mod pool;
mod state;

pub use pool::{PluginPool, PoolConfig};

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
    pub async fn load(bytes: &[u8], engine: &Engine, linker: &mut Linker<State>) -> anyhow::Result<Self> {
        let component = Component::new(engine, bytes)?;

        let export = "wassel:plugin/http-plugin";
        if component.get_export(None, export).is_none() {
            anyhow::bail!("There is no '{export}' export");
        }

        let mut store = wasmtime::Store::new(engine, State::default());
        let instance = linker.instantiate_async(&mut store, &component).await?;
        let bindings = bindings::Plugin::new(&mut store, &instance)?;

        let descriptor = bindings.wassel_plugin_http_plugin().call_instantiate(&mut store).await?;

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

    pub async fn handle(&self, route: &str) -> String {
        let mut store_guard = self.store.lock().await;
        let mut store = MutexGuard::deref_mut(&mut store_guard);
        let handler = *self.handler_map.at(route)
            .unwrap_or_else(|_| panic!("No handler for route {route}"))
            .value;

        self.bindings
            .wassel_plugin_http_plugin()
            .handler()
            .call_handle(&mut store, handler)
            .await
            .unwrap()
    }

    pub fn endpoints(&self) -> impl Iterator<Item = &str> {
        self.descriptor.endpoints.iter()
            .map(|e| e.path.as_str())
    }

    pub fn name(&self) -> &str {
        &self.descriptor.name
    }
}
