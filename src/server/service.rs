use std::pin::Pin;

use hyper::{Request, StatusCode, body::Incoming, service::Service};

use crate::plugin::PluginPool;

use super::{
    errors::ServeError,
    response::{self, IntoResponse},
};

#[derive(Clone)]
pub struct WasselService {
    pool: PluginPool,
}

impl WasselService {
    pub fn new(pool: PluginPool) -> Self {
        Self { pool }
    }
}

impl Service<Request<Incoming>> for WasselService {
    type Response = response::Response;
    type Error = ServeError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, req: Request<Incoming>) -> Self::Future {
        let s = self.clone();

        let future = async move {
            let Some(plugin) = s.pool.plugin_at(req.uri().path()) else {
                return Ok(StatusCode::NOT_FOUND.into_response());
            };

            let result = plugin.handle(req).await.map_err(ServeError::PluginError);

            Ok(result.into_response())
        };

        Box::pin(future)
    }
}
