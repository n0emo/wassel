use std::pin::Pin;

use hyper::{Request, StatusCode, body::Incoming, service::Service};
use tracing::{error, trace};

use crate::Stack;

use crate::{
    errors::ServeError,
    response::{self, IntoResponse},
};

impl Service<Request<Incoming>> for Stack {
    type Response = response::Response;
    type Error = ServeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let s = self.clone();

        let future = async move {
            let plugin = match s.get_plugin(req.uri().path()).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    trace!("No plugin found for {}", req.uri().path());
                    return Ok(StatusCode::NOT_FOUND.into_response());
                }
                Err(e) => {
                    error!("Could not get plugin for {}: {:#}", req.uri().path(), e);
                    return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
                }
            };

            let result = plugin.handle(req).await.map_err(ServeError::PluginError);
            Ok(result?.into_response())
        };

        Box::pin(future)
    }
}
