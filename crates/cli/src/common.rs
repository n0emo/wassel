use std::{
    collections::HashMap,
    env, fs, io,
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

    #[serde(default = "default_data_folder")]
    pub data_folder: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetaBuild {
    pub cmd: String,

    #[serde(default = "HashMap::default")]
    pub data: HashMap<PathBuf, PathBuf>,
}

#[derive(Debug, Clone)]
pub struct PluginBuildInfo {
    pub path: PathBuf,
    pub id: String,
    pub component: PathBuf,
    pub data_folder: PathBuf,
    pub data: HashMap<PathBuf, PathBuf>,
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
    if let Some(build) = &meta.build {
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
        data_folder: meta.data_folder,
        data: meta.build.map(|b| b.data).unwrap_or_default(),
    })
}

pub fn copy_plugin_to_plugins_folder(
    plugins_folder: &Path,
    info: &PluginBuildInfo,
) -> anyhow::Result<()> {
    let id = &info.id;
    let plugin_directory = plugins_folder.join(id);
    fs::create_dir_all(&plugin_directory).context("Creating plugin directory")?;
    fs::copy(&info.component, plugin_directory.join("plugin.wasm"))
        .context(format!("Copying plugin `{id}`"))?;
    fs::copy(
        info.path.join("plugin.toml"),
        plugin_directory.join("plugin.toml"),
    )
    .context(format!("Copying plugin metadata `{id}`"))?;

    fs::create_dir_all(plugin_directory.join(&info.data_folder))
        .context("Creating plugin data folder")?;
    for (source_path, destination_path) in &info.data {
        let from = info.path.join(source_path);
        let to = plugin_directory
            .join(&info.data_folder)
            .join(destination_path);
        copy_all(&from, &to).context(format!("Copying plugin data `{from:?}` -> `{to:?}`"))?;
    }

    Ok(())
}

fn copy_all(from: impl AsRef<Path>, to: impl AsRef<Path>) -> io::Result<()> {
    let from = from.as_ref();
    let to = to.as_ref();

    if from.is_file() {
        fs::copy(from, to).map(|_| ())
    } else {
        fs::create_dir_all(to)?;
        for entry in fs::read_dir(from)? {
            let entry = entry?;
            copy_all(entry.path(), to.join(entry.file_name()))?;
        }
        Ok(())
    }
}

fn default_data_folder() -> PathBuf {
    PathBuf::from("data")
}
