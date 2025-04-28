use std::{collections::HashMap, ffi::OsStr, sync::Arc};

use dashmap::DashMap;
use tracing::{debug, error, info};
use wasmtime::Engine;
use wasmtime_wasi_config::WasiConfig;

use crate::config::Config;

use super::{
    http::{HttpPlugin, HttpPluginImage},
    state::State,
};

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

            if module
                .path()
                .extension()
                .is_none_or(|ext| ext != OsStr::new("wasm"))
            {
                continue;
            }

            debug!("Loading {plugin:?}", plugin = module.path());
            let bytes = std::fs::read(module.path())?;

            let mut linker = wasmtime::component::Linker::<State>::new(&engine);
            wasmtime_wasi::p2::add_to_linker_async(&mut linker)?;
            wasmtime_wasi_http::add_only_http_to_linker_async(&mut linker)?;
            wasmtime_wasi_config::add_to_linker(&mut linker, |c| WasiConfig::from(&c.config_vars))?;

            let mut plugin = match HttpPluginImage::load(&bytes, &engine, &mut linker, HashMap::new()) {
                Ok(p) => p,
                Err(e) => {
                    error!("Error loading plugin {path:?}: {e}", path = module.path());
                    errors += 1;
                    continue;
                }
            };

            let instance = plugin.instantiate(&engine).await?;

            let name = instance.name();
            let plugin_config = config.plugins.get(name).cloned().unwrap_or_default();
            let mut base_url = plugin_config.get("base_url").map(|s| s.to_owned()).unwrap_or_default();
            if base_url.ends_with("/") {
                base_url.pop();
            }

            let mut plugin_router = matchit::Router::new();
            for endpoint in instance.endpoints() {
                let endpoint = format!("{base_url}{endpoint}");
                plugin_router
                    .insert(endpoint.to_owned(), instance.name().to_owned())
                    .unwrap();
            }
            plugin.set_config(plugin_config);

            map.insert(instance.name().to_owned(), plugin);
            router.merge(plugin_router)?;
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
