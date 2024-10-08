use clap::Parser;

use cli::{Cli, Commands};
use commands::sync::SyncOpts;
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
            sync_list,
        } => commands::sync::run(
            source,
            target,
            SyncOpts {
                fs_backend: fs_backend.unwrap(),
                codec,
                bitrate,
                transcode_codecs: transcode_codecs.unwrap_or(Vec::with_capacity(0)),
                sync_codecs: sync_codecs.unwrap(),
                sync_list,
            },
        ),
    };

    if let Err(e) = run {
        match e.type_ {
            ErrorType::Abort => {}
            _ => eprintln!("{}", e),
        }
    }
}
