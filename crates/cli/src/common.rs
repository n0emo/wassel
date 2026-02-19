use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail};
use serde::{Deserialize, Serialize};
use subprocess::{Exec, Redirection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasselMeta {
    #[serde(default = "StackMeta::default")]
    pub stack: StackMeta,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackMeta {
    #[serde(default = "Vec::default")]
    pub plugins: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub id: String,
    pub component: PathBuf,
    pub build: Option<PluginMetaBuild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetaBuild {
    pub cmd: String,
}

#[derive(Debug, Clone)]
pub struct PluginBuildInfo {
    pub path: PathBuf,
    pub id: String,
    pub component: PathBuf,
}

/// Returns path to the built component
pub fn build_plugin_at(path: &Path) -> anyhow::Result<PluginBuildInfo> {
    let meta_path = path.join("plugin.toml");
    let meta = fs::read(&meta_path).context(format!(
        "Reading plugin metadata at `{}`",
        meta_path.to_string_lossy()
    ))?;
    let meta: PluginMeta = toml::from_slice(&meta).context(format!(
        "Serializing plugin metadata at `{}`",
        meta_path.to_string_lossy()
    ))?;

    println!("Building plugin `{}`", meta.id);

    let envs = HashMap::<String, String>::from_iter(env::vars());
    if let Some(build) = meta.build {
        let cmd =
            subst::substitute(&build.cmd, &envs).context("Substituting environment variables")?;

        println!("Running `{}`", build.cmd);
        let status = Exec::shell(cmd)
            .cwd(path)
            .stdin(Redirection::None)
            .stdout(Redirection::None)
            .stderr(Redirection::None)
            .join()
            .context("Error executing command")?;

        if !status.success() {
            bail!("Build command returned status {status}");
        }
    } else {
        println!("Component does not have build step; assuming it already prebuilt");
    }

    let component = subst::substitute(&meta.component.to_string_lossy(), &envs)
        .context("Substituting environment variables")?;

    let component = {
        let mut c = PathBuf::from(component);
        if !c.has_root() {
            c = path.join(c);
        }
        c
    };

    if !Path::new(&component).exists() {
        bail!(
            "Component not present after build (missing file `{}`)",
            component.display()
        );
    }

    println!("Component built successfully at `{}`", component.display());

    Ok(PluginBuildInfo {
        path: path.to_owned(),
        id: meta.id,
        component,
    })
}
