use std::sync::Arc;

use dashmap::DashMap;
use tracing::{debug, error, info};
use wasmtime::Engine;

use crate::config::Config;

use super::http::{HttpPlugin, HttpPluginImage};

#[derive(Clone)]
pub struct PluginPool(Arc<PoolInner>);

struct PoolInner {
    map: dashmap::DashMap<String, HttpPluginImage>,
    engine: Engine,
    router: matchit::Router<String>,
}

impl PluginPool {
    pub async fn new(config: &Config) -> anyhow::Result<Self> {
        info!("Loading plugins");
        let mut successes = 0;
        let mut errors = 0;

        let engine = {
            let mut config = wasmtime::Config::new();
            config.async_support(true);
            Engine::new(&config)?
        };

        let map = DashMap::new();
        let mut router = matchit::Router::new();

        for module in std::fs::read_dir(&config.plugins_directory)? {
            let Ok(module) = module else {
                continue;
            };

            if !module.path().is_dir() {
                continue;
            }

            debug!("Loading {plugin:?}", plugin = module.path());

            let plugin =
                match HttpPluginImage::load(&module.path(), &engine, config).await {
                    Ok(p) => p,
                    Err(e) => {
                        error!("Error loading plugin {path:?}: {e}", path = module.path());
                        errors += 1;
                        continue;
                    }
                };

            for path in plugin.paths() {
                router.insert(path, plugin.id().to_owned())?;
            }
            map.insert(plugin.id().to_owned(), plugin);
            successes += 1;
        }

        info!("Loaded {successes} plugins with {errors} errors");

        Ok(Self(Arc::new(PoolInner {
            map,
            engine,
            router,
        })))
    }

    pub async fn get_plugin(&self, route: &str) -> Result<HttpPlugin, anyhow::Error> {
        let name = self.0.router.at(route).map(|m| m.value.as_str())?;
        let pair = self
            .0
            .map
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("Could not get plugin blugin by name"))?;
        let plugin = pair.value().instantiate(&self.0.engine).await?;
        Ok(plugin)
    }
}
