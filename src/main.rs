use clap::{CommandFactory, Parser};
use clap_complete::generate;

use cli::{Cli, Commands};
use errors::ErrorType;

mod cli;
mod commands;
mod errors;
mod format;
mod utils;

fn main() {
    let cli = Cli::parse();
    let run = match cli.command {
        Commands::Sync(opts) => commands::sync::run(opts),
        Commands::Completion { shell } => {
            let mut cmd = Cli::command();
            generate(shell, &mut cmd, "tsync", &mut std::io::stdout());
            return;
        }
    };

    if let Err(e) = run
        && e.type_ != ErrorType::Abort
    {
        eprintln!("{e}");
    }
}
