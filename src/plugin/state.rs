use std::time::Duration;

use http_body_util::{BodyExt, combinators::UnsyncBoxBody};
use wasmtime::component::Resource;
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView};
use wasmtime_wasi_config::WasiConfigVariables;
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView, body::HostIncomingBody};

use crate::{
    plugin::bindings::wassel::foundation::http_client::{self, IncomingResponse, OutgoingRequest},
    server::body::FullBody,
};

pub struct State {
    pub ctx: WasiCtx,
    pub http_ctx: WasiHttpCtx,
    pub http_client: reqwest::Client,
    pub config_vars: WasiConfigVariables,
    pub table: ResourceTable,
}

impl Default for State {
    fn default() -> Self {
        let ctx = {
            let mut builder = WasiCtxBuilder::new();
            builder.inherit_stdout();
            builder.inherit_stderr();
            builder.build()
        };
        let http_ctx = WasiHttpCtx::new();
        let table = ResourceTable::new();
        let config_vars = WasiConfigVariables::new();

        Self {
            ctx,
            http_ctx,
            http_client: reqwest::Client::new(),
            config_vars,
            table,
        }
    }
}

impl WasiView for State {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for State {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl http_client::Host for State {
    async fn send(
        &mut self,
        url: http_client::Url,
        req: Resource<OutgoingRequest>,
    ) -> Result<Resource<IncomingResponse>, http_client::ErrorCode> {
        use wasmtime_wasi_http::bindings::http::types::Method as WasiMethod;
        let req = self
            .table
            .get_mut(&req)
            // TODO: better error handling
            .map_err(|e| http_client::ErrorCode::InternalError(Some(e.to_string())))?;

        let method = match &req.method {
            WasiMethod::Get => reqwest::Method::GET,
            WasiMethod::Head => reqwest::Method::HEAD,
            WasiMethod::Post => reqwest::Method::POST,
            WasiMethod::Put => reqwest::Method::PUT,
            WasiMethod::Delete => reqwest::Method::DELETE,
            WasiMethod::Connect => reqwest::Method::CONNECT,
            WasiMethod::Options => reqwest::Method::OPTIONS,
            WasiMethod::Trace => reqwest::Method::TRACE,
            WasiMethod::Patch => reqwest::Method::PATCH,
            WasiMethod::Other(m) => reqwest::Method::from_bytes(m.as_bytes())
                // TODO: better error handling
                .map_err(|e| http_client::ErrorCode::InternalError(Some(e.to_string())))?,
        };

        let mut request = self
            .http_client
            .request(method, url)
            .headers(req.headers.clone());

        if let Some(body) = req.body.take() {
            let body = reqwest::Body::wrap_stream(body.into_data_stream());
            request = request.body(body);
        }

        let response = request
            .send()
            .await
            .map_err(|_e| http_client::ErrorCode::HttpRequestDenied)?;

        let status = response.status().into();
        let headers = response.headers().to_owned();

        // TODO: stream body
        let hyper_body = UnsyncBoxBody::new(FullBody::new(
            response
                .bytes()
                .await
                // TODO: better error handling
                .map_err(|e| http_client::ErrorCode::InternalError(Some(e.to_string())))?,
        ));
        let incoming_body = HostIncomingBody::new(hyper_body, Duration::from_secs(5));

        let response = IncomingResponse {
            status,
            headers,
            body: Some(incoming_body),
        };

        let resource = self
            .table
            .push(response)
            // TODO: better error handling
            .map_err(|e| http_client::ErrorCode::InternalError(Some(e.to_string())))?;

        Ok(resource)
    }
}
