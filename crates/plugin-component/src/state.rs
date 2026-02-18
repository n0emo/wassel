use anyhow::Context as _;
use wasmtime_wasi::{
    DirPerms, FilePerms, ResourceTable, WasiCtx, WasiCtxBuilder, WasiCtxView, WasiView,
};
use wasmtime_wasi_config::WasiConfigVariables;
use wasmtime_wasi_http::{WasiHttpCtx, WasiHttpView, body::HostIncomingBody};

use std::{path::Path, pin::Pin, time::Duration};

use bytes::Bytes;
use futures_util::{Stream, TryStreamExt as _};
use http::Method;
use http::method::InvalidMethod;
use http_body_util::{BodyExt as _, combinators::UnsyncBoxBody};
use wasmtime::component::Resource;
use wassel_world::{
    wasi::http::types::{ErrorCode, Method as WasiMethod},
    wassel::foundation::http_client::{self, IncomingResponse, OutgoingRequest},
};

pub struct PluginState {
    ctx: WasiCtx,
    config_vars: WasiConfigVariables,
    table: ResourceTable,
    http_ctx: WasiHttpCtx,
    http_client: reqwest::Client,
}

impl PluginState {
    pub fn new(data_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        let ctx = {
            let mut builder = WasiCtxBuilder::new();
            builder.inherit_stdout();
            builder.inherit_stderr();
            builder
                .preopened_dir(data_dir.as_ref(), ".", DirPerms::all(), FilePerms::all())
                .context(format!(
                    "Preopening data directory `{}`",
                    data_dir.as_ref().to_string_lossy()
                ))?;
            builder.build()
        };

        let s = Self {
            ctx,
            config_vars: WasiConfigVariables::new(),
            table: ResourceTable::new(),
            http_ctx: WasiHttpCtx::new(),
            http_client: reqwest::Client::new(),
        };

        Ok(s)
    }

    pub fn config_vars(&self) -> &WasiConfigVariables {
        &self.config_vars
    }
}

impl WasiView for PluginState {
    fn ctx(&mut self) -> WasiCtxView<'_> {
        WasiCtxView {
            ctx: &mut self.ctx,
            table: &mut self.table,
        }
    }
}

impl WasiHttpView for PluginState {
    fn ctx(&mut self) -> &mut WasiHttpCtx {
        &mut self.http_ctx
    }

    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl http_client::Host for PluginState {
    async fn send(
        &mut self,
        url: http_client::Url,
        req: Resource<OutgoingRequest>,
    ) -> Result<Resource<IncomingResponse>, ErrorCode> {
        let req = self.table.get_mut(&req).map_err(|e| {
            ErrorCode::InternalError(Some(format!(
                "Could not get OutgoingRequest resource: {e:?}"
            )))
        })?;

        let method = convert_wasi_method_to_reqwest_method(&req.method)
            .map_err(|_| ErrorCode::HttpRequestMethodInvalid)?;

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
            .map_err(convert_reqwest_error_to_error_code)?;

        let status = response.status().into();
        let headers = response.headers().to_owned();

        let body_stream = Box::pin(response.bytes_stream());

        let hyper_body = UnsyncBoxBody::new(StreamBody {
            stream: body_stream,
        });
        let incoming_body = HostIncomingBody::new(hyper_body, Duration::from_secs(5));

        let response = IncomingResponse {
            status,
            headers,
            body: Some(incoming_body),
        };

        let resource = self.table.push(response).map_err(|e| {
            ErrorCode::InternalError(Some(format!(
                "Could not create IncomingResponse resource: {e:?}"
            )))
        })?;

        Ok(resource)
    }
}

struct StreamBody<S> {
    stream: S,
}

impl<S> hyper::body::Body for StreamBody<Pin<Box<S>>>
where
    S: Stream<Item = reqwest::Result<Bytes>>,
{
    type Data = Bytes;

    type Error = ErrorCode;

    fn poll_frame(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        match self.get_mut().stream.try_poll_next_unpin(cx) {
            std::task::Poll::Ready(v) => match v {
                Some(result) => std::task::Poll::Ready(Some(match result {
                    Ok(bytes) => Ok(hyper::body::Frame::data(bytes)),
                    Err(e) => Err(convert_reqwest_error_to_error_code(e)),
                })),
                None => std::task::Poll::Ready(None),
            },
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}

fn convert_wasi_method_to_reqwest_method(method: &WasiMethod) -> Result<Method, InvalidMethod> {
    let method = match method {
        WasiMethod::Get => Method::GET,
        WasiMethod::Head => Method::HEAD,
        WasiMethod::Post => Method::POST,
        WasiMethod::Put => Method::PUT,
        WasiMethod::Delete => Method::DELETE,
        WasiMethod::Connect => Method::CONNECT,
        WasiMethod::Options => Method::OPTIONS,
        WasiMethod::Trace => Method::TRACE,
        WasiMethod::Patch => Method::PATCH,
        WasiMethod::Other(m) => Method::from_bytes(m.as_bytes())?,
    };

    Ok(method)
}

fn convert_reqwest_error_to_error_code(e: reqwest::Error) -> ErrorCode {
    ErrorCode::InternalError(Some(format!("Reqwest error: {e:?}")))
}
