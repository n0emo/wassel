use bytes::Bytes;
use http_body_util::{BodyExt, combinators::BoxBody};
use wasmtime_wasi_http::bindings::http::types::ErrorCode;

use super::body::EmptyBody;

pub type Response = hyper::Response<BoxBody<Bytes, anyhow::Error>>;

pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl<T: IntoResponse, E: IntoResponse> IntoResponse for Result<T, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(v) => v.into_response(),
            Err(e) => e.into_response(),
        }
    }
}

impl IntoResponse for hyper::http::StatusCode {
    fn into_response(self) -> Response {
        let body = BoxBody::new(EmptyBody::new());
        let mut resp = hyper::Response::new(body);
        *resp.status_mut() = self;
        resp
    }
}

impl IntoResponse for hyper::Response<BoxBody<Bytes, ErrorCode>> {
    fn into_response(self) -> Response {
        let (parts, body) = self.into_parts();
        let body = body.map_err(|e| anyhow::format_err!("{e}")).boxed();
        Response::from_parts(parts, body)
    }
}
