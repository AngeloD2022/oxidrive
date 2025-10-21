mod commands;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Command-line companion for the Oxidrive hyperdrive toolkit",
    long_about = None,
    propagate_version = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(about = "List products in a channel with optional filtering")]
    List(commands::list::ListArgs),
    #[command(about = "Show detailed metadata for a product")]
    Info(commands::info::InfoArgs),
    #[command(about = "Download product packages to disk")]
    Download(commands::download::DownloadArgs),
    #[command(about = "Prepare installation media for a product")]
    Install(commands::install::InstallArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::List(args)) => commands::list::execute(args).await?,
        Some(Commands::Info(args)) => commands::info::execute(args).await?,
        Some(Commands::Download(args)) => commands::download::execute(args).await?,
        Some(Commands::Install(args)) => commands::install::execute(args).await?,
        None => {
            Cli::command().print_help()?;
            println!();
        }
    }

    Ok(())
}
