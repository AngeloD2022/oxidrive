mod commands;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CLI {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Show,
    List(commands::list::ListArgs),
    Info(commands::info::InfoArgs),
    Download(commands::download::DownloadArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = CLI::parse();

    match cli.command.unwrap_or(Commands::Show) {
        Commands::Show => {
            commands::show::show()?;
            Ok(())
        }
        Commands::List(args) => commands::list::execute(args).await,
        Commands::Info(args) => commands::info::execute(args).await,
        Commands::Download(args) => commands::download::execute(args).await,
    }
}
