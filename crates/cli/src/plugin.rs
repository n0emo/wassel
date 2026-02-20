use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use clap::{Args, Subcommand};

use crate::common;

#[derive(Debug, Args)]
pub struct PluginArgs {
    #[command(subcommand)]
    pub command: PluginCommand,

    #[arg(long, short, default_value = ".")]
    pub path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommand {
    /// Build plugin
    Build,

    /// Start Wassel application and serve using single plugin
    Serve,
}

pub fn run(args: PluginArgs) -> anyhow::Result<()> {
    match args.command {
        PluginCommand::Build => cmd_build(&args.path),
        PluginCommand::Serve => cmd_serve(&args.path),
    }
}

fn cmd_build(path: &Path) -> anyhow::Result<()> {
    common::build_plugin_at(path)?;
    Ok(())
}

fn cmd_serve(path: &Path) -> anyhow::Result<()> {
    let info = common::build_plugin_at(path)?;

    let plugins_path = Path::new("plugins");
    let plugin_dir = plugins_path.join(&info.id);
    if plugins_path.exists() {
        fs::remove_dir_all(plugins_path).context(format!(
            "Removing plugins directory at `{}`",
            plugin_dir.to_string_lossy()
        ))?;
    }

    common::copy_plugin_to_plugins_folder(plugins_path, &info)?;

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Building tokio runtime")?
        .block_on(wassel_server::run_server())?;

    Ok(())
}
