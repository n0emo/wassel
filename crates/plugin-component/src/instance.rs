use std::{ops::DerefMut as _, str::FromStr};

use http::{Uri, uri::PathAndQuery};
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
    endpoint: String,
}

impl PluginInstance {
    pub fn new(instance: Instance, store: Mutex<Store<PluginState>>, endpoint: String) -> Self {
        Self {
            instance,
            store,
            endpoint,
        }
    }

    pub async fn handle(
        &self,
        mut req: Request<Incoming>,
    ) -> Result<Response<HyperOutgoingBody>, PluginHandleError> {
        let (sender, reciever) = tokio::sync::oneshot::channel();

        let mut store_guard = self.store.lock().await;
        let mut store = MutexGuard::deref_mut(&mut store_guard);

        let mut parts = req.uri().clone().into_parts();
        let paq = parts
            .path_and_query
            .expect("Path and query should be present in request");
        let paq = paq
            .as_str()
            .strip_prefix(&self.endpoint)
            .expect("URI must start with the plugin prefix");
        let paq = "/".to_owned() + paq;
        parts.path_and_query = Some(
            PathAndQuery::from_str(&paq)
                .expect("Parts and query should still be valid after stripping prefix"),
        );
        *req.uri_mut() =
            Uri::from_parts(parts).expect("URI should still be valid after stripping prefix");

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
