use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as _, bail};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use subprocess::Redirection;

#[derive(Debug, Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    command: PluginCommand,

    #[arg(long, short, default_value = ".")]
    path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    /// Build plugin
    Build,

    /// Start Wassel application and serve using single plugin
    Serve,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginMeta {
    id: String,
    component: PathBuf,
    build: Option<PluginMetaBuild>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginMetaBuild {
    cmd: String,
}

pub fn run(args: PluginArgs) -> anyhow::Result<()> {
    match args.command {
        PluginCommand::Build => cmd_build(&args.path),
        PluginCommand::Serve => cmd_serve(&args.path),
    }
}

fn cmd_build(path: &Path) -> anyhow::Result<()> {
    let meta_path = path.join("plugin.toml");
    let meta = std::fs::read(&meta_path).context(format!(
        "Reading plugin metadata at `{}`",
        meta_path.to_string_lossy()
    ))?;
    let meta: PluginMeta = toml::from_slice(&meta).context(format!(
        "Serializing plugin metadata at `{}`",
        meta_path.to_string_lossy()
    ))?;

    let envs = HashMap::<String, String>::from_iter(env::vars());
    if let Some(build) = meta.build {
        let cmd =
            subst::substitute(&build.cmd, &envs).context("Substituting environment variables")?;

        println!("Running `{}`", build.cmd);
        let status = subprocess::Exec::shell(cmd)
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

    if !Path::new(&component).exists() {
        bail!("Component not present after build (missing file `{component}`)");
    }

    println!("Component build successfully at `{component}`");

    Ok(())
}

fn cmd_serve(path: &Path) -> anyhow::Result<()> {
    cmd_build(path)?;

    let meta_path = path.join("plugin.toml");
    let meta = std::fs::read(&meta_path).context(format!(
        "Reading plugin metadata at `{}`",
        meta_path.to_string_lossy()
    ))?;
    let meta: PluginMeta = toml::from_slice(&meta).context(format!(
        "Serializing plugin metadata at `{}`",
        meta_path.to_string_lossy()
    ))?;

    let envs = HashMap::<String, String>::from_iter(env::vars());
    let component = subst::substitute(&meta.component.to_string_lossy(), &envs)
        .context("Substituting environment variables")?;
    let component = Path::new(&component);

    let plugins_path = Path::new("plugins");
    let plugin_dir = plugins_path.join(meta.id);
    if plugins_path.exists() {
        fs::remove_dir_all(plugins_path).context(format!(
            "Removing plugins directory at `{}`",
            plugin_dir.to_string_lossy()
        ))?;
    }
    fs::create_dir_all(&plugin_dir).context(format!(
        "Creating plugins directory at `{}`",
        plugin_dir.to_string_lossy()
    ))?;
    fs::copy(component, plugin_dir.join("plugin.wasm"))
        .context("Copying plugin.wasm to plugin directory")?;
    fs::copy(meta_path, plugin_dir.join("plugin.toml"))
        .context("Copying plugin.toml to plugin directory")?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Building tokio runtime")?
        .block_on(wassel_server::run_server())?;

    Ok(())
}
