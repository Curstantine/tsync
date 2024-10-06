use clap::Parser;

use cli::{Cli, Commands};
use errors::ErrorType;

mod cli;
mod commands;
mod constants;
mod errors;
mod format;
mod utils;

fn main() {
    let cli = Cli::parse();
    let run = match cli.command {
        Commands::Sync {
            source,
            target,
            fs_backend,
            codec,
            bitrate,
            transcode_codecs,
            sync_codecs,
        } => commands::sync::run(
            source,
            target,
            fs_backend.unwrap(),
            codec,
            bitrate,
            transcode_codecs.unwrap(),
            sync_codecs.unwrap(),
        ),
    };

    if let Err(e) = run {
        match e.type_ {
            ErrorType::Abort => {}
            _ => eprintln!("{}", e),
        }
    }
}
