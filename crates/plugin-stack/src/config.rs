use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tokio::fs;
use tracing::{debug, error};
use wassel_plugin_component::PluginMeta;

#[derive(Debug, Clone, Default)]
pub struct StackConfig {
    pub meta: StackMeta,
    pub plugin_paths: HashMap<String, PathBuf>,
    pub plugins: HashMap<String, PluginMeta>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackMeta {
    #[serde(default = "HashMap::default")]
    pub variables: HashMap<String, String>,
}

impl StackConfig {
    pub async fn load(base_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let meta_path = base_path.as_ref().join("wassel.toml");
        debug!("Reading stack config at `{}`", meta_path.to_string_lossy());

        let meta: StackMeta = if !meta_path.exists() {
            StackMeta::default()
        } else {
            toml::from_slice(&fs::read(&meta_path).await.context(format!(
                "Reading stack meta at `{}`",
                meta_path.to_string_lossy()
            ))?)
            .context(format!(
                "Deserializing stack meta at `{}`",
                meta_path.to_string_lossy()
            ))?
        };

        let mut plugins = HashMap::new();
        let mut plugin_paths = HashMap::new();
        let plugins_path = base_path.as_ref().join("plugins");
        if matches!(fs::try_exists(&plugins_path).await, Ok(true)) {
            let mut read_dir = fs::read_dir(&plugins_path).await.context(format!(
                "Reading plugins directory at `{}`",
                plugins_path.to_string_lossy()
            ))?;
            loop {
                match read_dir.next_entry().await {
                    Ok(Some(dir)) => {
                        read_plugin_entry(&mut plugins, &mut plugin_paths, &dir)
                            .await
                            .context(format!(
                                "Reading plugin entry in `{}`",
                                dir.path().to_string_lossy()
                            ))?;
                    }
                    Ok(None) => break,
                    Err(e) => error!("Could not read plugin directory: {e}"),
                }
            }
        }

        Ok(Self {
            meta,
            plugins,
            plugin_paths,
        })
    }

    pub fn merge_plugin_config(
        &mut self,
        id: impl Into<String>,
        meta: impl Into<PluginMeta>,
    ) -> &mut Self {
        self.plugins.insert(id.into(), meta.into());
        self
    }
}

async fn read_plugin_entry(
    plugins: &mut HashMap<String, PluginMeta>,
    plugin_paths: &mut HashMap<String, PathBuf>,
    dir: &fs::DirEntry,
) -> Result<(), anyhow::Error> {
    let plugin_meta_path = dir.path().join("plugin.toml");
    let plugin_meta: PluginMeta =
        toml::from_slice(&fs::read(&plugin_meta_path).await.context(format!(
            "Reading plugin meta at `{}`",
            plugin_meta_path.to_string_lossy()
        ))?)
        .context(format!(
            "Deserializing plugin meta at `{}`",
            plugin_meta_path.to_string_lossy()
        ))?;
    let id = plugin_meta.id.clone();

    if let Some(val) = plugins.insert(id.clone(), plugin_meta) {
        bail!("Multiple plugins with the same id `{}`", val.id);
    }

    plugin_paths.insert(id, dir.path());

    Ok(())
}
