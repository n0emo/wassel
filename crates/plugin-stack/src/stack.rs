use std::{collections::HashMap, ops::Deref, path::Path, sync::Arc};

use anyhow::Context;
use tokio::fs;
use tracing::{debug, error, info, trace};
use wasmtime::Engine;
use wassel_plugin_component::{PluginImage, PluginInstance};

use crate::config::StackConfig;

#[derive(Clone)]
pub struct Stack(Arc<StackInner>);

impl Deref for Stack {
    type Target = StackInner;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl Stack {
    pub async fn load(base_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(Self(Arc::new(StackInner::load(base_path).await?)))
    }

    pub async fn get_plugin(&self, route: &str) -> Result<Option<PluginInstance>, anyhow::Error> {
        let name = self.router.at(route).map(|m| m.value.as_str())?;
        let Some(image) = self.map.get(name) else {
            return Ok(None);
        };
        trace!("Found plugin image for {route}");
        let plugin = image.instantiate(&self.0.engine).await?;
        debug!("Instantiated plugin {} to handle {}", image.id(), route);
        Ok(Some(plugin))
    }
}

pub struct StackInner {
    map: HashMap<String, PluginImage>,
    engine: Engine,
    router: matchit::Router<String>,
}

impl StackInner {
    pub async fn load(base_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config = StackConfig::load(&base_path).await.context(format!(
            "Loading config in `{}`",
            base_path.as_ref().to_string_lossy()
        ))?;

        info!("Loading plugins");
        let mut successes = 0;
        let mut errors = 0;

        let engine = {
            let mut config = wasmtime::Config::new();
            config.async_support(true);
            Engine::new(&config).context("Creating Engine")?
        };

        let mut map = HashMap::new();
        let mut router = matchit::Router::new();

        for (plugin_id, plugin_meta) in config.plugins {
            let plugin_path = &config.plugin_paths[&plugin_id];
            debug!("Loading `{}`", plugin_path.to_string_lossy());

            let mut base_url = plugin_meta.endpoint.clone();
            if !base_url.ends_with('/') {
                base_url += "/";
            }
            let base_url_catchall = base_url.clone() + "{*path}";

            let wasm_path = plugin_path.join("plugin.wasm");
            let bytes = fs::read(&wasm_path)
                .await
                .context(format!("Reading `{}`", wasm_path.to_string_lossy()))?;
            let data_dir = plugin_path.join(&plugin_meta.data_dir);
            if !data_dir.exists() {
                fs::create_dir_all(&data_dir).await?;
            }
            let plugin = match PluginImage::load(&engine, &bytes, plugin_meta, data_dir).await {
                Ok(p) => p,
                Err(e) => {
                    error!(
                        "Error loading plugin `{path:?}`: {e:#}",
                        path = plugin_path.to_string_lossy()
                    );
                    errors += 1;
                    continue;
                }
            };

            trace!("Registering plugin at route {base_url}");

            if router.at(&base_url).is_ok() || router.at(&base_url_catchall).is_ok() {
                error!("Same url `{base_url}` is already handled by another plugin");
                errors += 1;
                continue;
            }

            router
                .insert(base_url, plugin.id().to_owned())
                .context("Inserting plugin into router")?;
            router
                .insert(base_url_catchall, plugin.id().to_owned())
                .context("Inserting plugin into router")?;

            map.insert(plugin.id().to_owned(), plugin);

            successes += 1;
        }

        info!("Loaded {successes} plugins with {errors} errors");

        Ok(Self {
            map,
            engine,
            router,
        })
    }
}
