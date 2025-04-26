use std::{ffi::OsStr, sync::Arc};

use dashmap::DashMap;
use tracing::{debug, error, info};
use wasmtime::{Engine, component::Linker};

use super::{http::HttpPlugin, state::State};

pub struct PoolConfig {
    pub plugins_directory: String,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            plugins_directory: "plugins".to_owned(),
        }
    }
}

#[derive(Clone)]
pub struct PluginPool(Arc<PoolInner>);

#[allow(unused)]
struct PoolInner {
    map: dashmap::DashMap<String, Arc<HttpPlugin>>,
    engine: Engine,
    linker: Linker<State>,
    router: matchit::Router<String>,
}

impl PluginPool {
    pub async fn new(config: &PoolConfig) -> anyhow::Result<Self> {
        info!("Loading plugins");
        let mut successes = 0;
        let mut errors = 0;

        let engine = {
            let mut config = wasmtime::Config::new();
            config.async_support(true);
            Engine::new(&config)?
        };

        let mut linker = wasmtime::component::Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
        wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)?;

        let map = DashMap::new();
        let mut router = matchit::Router::new();

        for module in std::fs::read_dir(&config.plugins_directory)? {
            let Ok(module) = module else {
                continue;
            };

            if module
                .path()
                .extension()
                .is_none_or(|ext| ext != OsStr::new("wasm"))
            {
                continue;
            }

            debug!("Loading {plugin:?}", plugin = module.path());
            let bytes = std::fs::read(module.path())?;

            let plugin = match HttpPlugin::load(&bytes, &engine, &mut linker).await {
                Ok(p) => p,
                Err(e) => {
                    error!("Error loading plugin {path:?}: {e}", path = module.path());
                    errors += 1;
                    continue;
                }
            };

            for endpoint in plugin.endpoints() {
                router
                    .insert(endpoint.to_owned(), plugin.name().to_owned())
                    .unwrap();
            }

            map.insert(plugin.name().to_owned(), Arc::new(plugin));
            successes += 1;
        }

        info!("Loaded {successes} plugins with {errors} errors");

        Ok(Self(Arc::new(PoolInner {
            map,
            engine,
            linker,
            router,
        })))
    }

    pub fn plugin_at(&self, route: &str) -> Option<Arc<HttpPlugin>> {
        let name = self.0.router.at(route).map(|m| m.value).ok()?;
        let pair = self.0.map.get(name)?;
        Some(Arc::clone(pair.value()))
    }
}
