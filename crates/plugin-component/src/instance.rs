use std::ops::DerefMut as _;

use hyper::{Request, Response, body::Incoming};
use tokio::sync::{Mutex, MutexGuard};
use wasmtime::{Store, component::Instance};
use wasmtime_wasi_http::{
    WasiHttpView as _, bindings::http::types::Scheme, body::HyperOutgoingBody,
};

use crate::{errors::PluginHandleError, state::PluginState};

pub struct PluginInstance {
    instance: Instance,
    store: Mutex<Store<PluginState>>,
}

impl PluginInstance {
    pub fn new(instance: Instance, store: Mutex<Store<PluginState>>) -> Self {
        Self { instance, store }
    }

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

        let proxy = wassel_world::HttpPlugin::new(&mut store, &self.instance)
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
