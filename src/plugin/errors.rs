use tokio::sync::oneshot::error::RecvError;
use wasmtime_wasi_http::bindings::http::types::ErrorCode;

#[derive(Debug, thiserror::Error)]
pub enum PluginHandleError {
    #[error("Endpoint for path '{0}' was not found in the plugin")]
    EndpointNotFound(String),

    #[error("Hander export {handler} for {path} was not found in the plugin: {error}")]
    GettingHandlerExport {
        path: String,
        handler: String,
        error: wasmtime::Error,
    },

    #[error("Could not create resource: {0}")]
    CreateResource(wasmtime::Error),

    #[error("Error occured when trying to call handle method: {0}")]
    CallingHandleMethod(wasmtime::Error),

    #[error("Could not recieve response from plugin: {0}")]
    RecieveResponse(#[from] RecvError),

    #[error("Plugin returned error code: {0}")]
    ErrorCode(#[from] ErrorCode),
}
