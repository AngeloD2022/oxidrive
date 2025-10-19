mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
    #[command(about = "Display a summary of available commands")]
    Show,
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

    match cli.command.unwrap_or_else(|| Commands::Show) {
        Commands::Show => commands::show::show()?,
        Commands::List(args) => commands::list::execute(args).await?,
        Commands::Info(args) => commands::info::execute(args).await?,
        Commands::Download(args) => commands::download::execute(args).await?,
        Commands::Install(args) => commands::install::execute(args).await?,
    }

    Ok(())
}
