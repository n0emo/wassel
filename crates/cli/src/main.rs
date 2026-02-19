use clap::{Parser, Subcommand};

use crate::{plugin::PluginArgs, stack::StackArgs};

mod common;
mod plugin;
mod stack;

#[derive(Debug, Parser)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Manage application stack
    Stack(StackArgs),

    /// Operations on single plugin
    Plugin(PluginArgs),
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    match args.cmd {
        Command::Stack(stack_args) => stack::run(stack_args),
        Command::Plugin(plugin_args) => plugin::run(plugin_args),
    }
}
