use clap::Parser;

use cli::{Cli, Commands};
use errors::Error;

mod cli;
mod commands;
mod constants;
mod errors;
mod utils;

fn main() {
    if let Err(e) = run() {
        match e {
            Error::Abort => {}
            _ => eprintln!("{}", e),
        }
    };
}

fn run() -> errors::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync {
            source,
            target,
            format,
            bitrate,
            filter_ext,
        } => commands::sync::sync(source, target, format, bitrate, filter_ext),
    }?;

    Ok(())
}
