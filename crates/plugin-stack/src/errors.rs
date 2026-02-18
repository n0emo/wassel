use hyper::StatusCode;
use wassel_plugin_component::PluginHandleError;

use crate::response::IntoResponse;

#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("WASI runtime error: {0}")]
    PluginError(#[from] PluginHandleError),
}

impl IntoResponse for ServeError {
    fn into_response(self) -> super::response::Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
