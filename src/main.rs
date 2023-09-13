use clap::Parser;

use cli::{Cli, Commands};
use errors::ErrorType;

mod cli;
mod commands;
mod constants;
mod errors;
mod format;
mod utils;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        match e.type_ {
            ErrorType::Abort => {}
            _ => eprintln!("{}", e),
        }
    };
}

async fn run() -> errors::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Sync {
            source,
            target,
            codec,
            bitrate,
            transcode_extensions,
            sync_extensions,
        } => commands::sync::run(source, target, codec, bitrate, transcode_extensions, sync_extensions),
    }
    .await?;

    Ok(())
}
