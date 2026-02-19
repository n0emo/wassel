use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context as _;
use clap::{Args, Subcommand};

use crate::common::{self, build_plugin_at};

#[derive(Debug, Args)]
pub struct StackArgs {
    #[command(subcommand)]
    command: StackCommand,

    #[arg(long, short, default_value = ".")]
    manifest_path: PathBuf,
}

#[derive(Debug, Subcommand)]
pub enum StackCommand {
    Build,
    Serve,
}

pub fn run(args: StackArgs) -> anyhow::Result<()> {
    match args.command {
        StackCommand::Build => cmd_build(&args.manifest_path),
        StackCommand::Serve => cmd_serve(&args.manifest_path),
    }
}

pub fn cmd_build(path: &Path) -> anyhow::Result<()> {
    build_entire_stack(path)?;
    Ok(())
}

pub fn cmd_serve(path: &Path) -> anyhow::Result<()> {
    build_entire_stack(path)?;
    println!("All plugins built successfully");
    println!("Starting wassel server");
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Building tokio runtime")?
        .block_on(wassel_server::run_server())?;
    Ok(())
}

fn build_entire_stack(path: &Path) -> anyhow::Result<()> {
    let meta_path = path.join("wassel.toml");
    let meta = fs::read(&meta_path).context(format!("Reading wassel config at `{meta_path:?}`"))?;
    let meta: common::WasselMeta = toml::from_slice(&meta)
        .context(format!("Deserializing wassel config at `{meta_path:?}`"))?;

    let plugins_path = path.join("plugins");
    if plugins_path.exists() {
        fs::remove_dir_all(&plugins_path).context("Removing plugins directory")?;
    }

    fs::create_dir_all(&plugins_path).context("Creating plugins directory")?;

    for plugin_path in meta.stack.plugins {
        let info =
            build_plugin_at(&plugin_path).context(format!("Building plugin `{plugin_path:?}`"))?;
        let id = &info.id;
        let plugin_directory = plugins_path.join(id);
        fs::create_dir_all(&plugin_directory).context("Creating plugin directory")?;
        fs::copy(info.component, plugin_directory.join("plugin.wasm"))
            .context(format!("Copying plugin `{id}`"))?;
        fs::copy(
            plugin_path.join("plugin.toml"),
            plugin_directory.join("plugin.toml"),
        )
        .context(format!("Copying plugin metadata `{id}`"))?;
    }

    Ok(())
}
