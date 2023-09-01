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
The codec to use while syncing (on-the-fly).
Transcoding will only apply if something is passed to this, else only the files matched by `sync_extensions` will synced.

Supported codecs: opus (opusenc), libopus (ffmpeg), libvorbis (ffmpeg), libmp3lame (ffmpeg)"
        )]
        codec: Option<String>,

        #[arg(
            long,
            short,
            long_help = "\
The bitrate to use while transcoding files matched by `transcode_extensions`.
Only applies if `codec` is set.

Supported bitrates:
    - opus, libopus: 6-256 (128 default)
    - libvorbis: 32-500 (192 default)
    - libmp3lame: 32-500 (192 default)"
        )]
        bitrate: Option<u32>,

        #[arg(
            long,
            default_value = "flac,alac",
            long_help = "A comma-separated list of extensions to include in the transcoding process."
        )]
        transcode_extensions: Option<String>,

        #[arg(
            long,
            default_value = "opus,ogg,mp3",
            long_help = "A comma-separated list of extensions to include in the sync, but not to transcode."
        )]
        sync_extensions: Option<String>,
    },
}
