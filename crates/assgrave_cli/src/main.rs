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
}

fn main() -> Result<()> {
    let cli = CLI::parse();

    match cli.command.unwrap_or(Commands::Show) {
        Commands::Show => {
            commands::show::show()?;
            Ok(())
        }
    }
}
