use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Syncs a music library to an ADB-connected Android device.
    Sync {
        /// The source directory to sync from.
        source: String,

        /// The directory on the device to sync to.
        target: String,

        #[arg(
            long,
            short,
            long_help = "\
The format to transcode to while syncing with the device. 
If supplied, all files that are not in this format will be transcoded to it.
Supported formats: opus, ogg, mp3"
        )]
        format: Option<String>,

        #[arg(
            long,
            short,
            long_help = "\
The bitrate to used for transcoding.
Will only be used if `format` is supplied, and the source file is not in the specified format.
Supported bitrates: opus: 6-510, ogg: 45-500, mp3: 8-320"
        )]
        bitrate: Option<u32>,

        #[arg(
            long,
            long_help = "\
Extensions to filter by when syncing.
Multiple extensions can be specified by separating them with a comma.
Example: --filter-ext flac,opus would only sync files with the extensions flac and opus."
        )]
        filter_ext: Option<String>,
    },
}
