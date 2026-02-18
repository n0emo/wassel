use clap::{Parser, Subcommand};

use crate::plugin::PluginArgs;

mod plugin;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage application stack
    Stack,

    /// Operations on single plugin
    Plugin(PluginArgs),
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Stack => todo!(),
        Command::Plugin(plugin_args) => plugin::run(plugin_args),
    }
}
