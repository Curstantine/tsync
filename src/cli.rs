use clap::{
    Parser, Subcommand,
    builder::styling::{AnsiColor, Color, Style},
};
use clap_complete::Shell;

use crate::{format::Codec, utils::fs::FSBackend};

#[derive(Parser)]
#[command(author, version, about, long_about = None, styles = get_styles())]
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

        /// The directory to sync to.
        target: String,

        #[arg(long, short, default_value = "adb")]
        /// Specifies the filesystem backend to use for syncing.
        fs: Option<FSBackend>,

        #[arg(long, short)]
        /// The codec to transcode into for tracks matching the transcode_codecs.
        ///
        /// Transcoding will only apply if a value is passed, else only the files matched by `sync_extensions` will synced.
        codec: Option<Codec>,

        #[arg(long, short)]
        /// The bitrate to use while transcoding files matched by `transcode_extensions`.
        /// Only applies if `codec` is set.
        ///
        /// Default values:
        /// - opus: 128K
        /// - vorbis: 192K
        /// - mp3: 320K
        /// - aac-lc: 192K
        bitrate: Option<u32>,

        #[arg(long, default_value_t = false)]
        /// If set, album cover images will be stripped from synced files.
        strip_covers: bool,

        #[arg(long, value_delimiter = ',', default_value = "flac,alac")]
        /// A comma-separated list of codecs to match to include in the transcode process.
        transcode_codecs: Option<Vec<Codec>>,

        #[arg(long, value_delimiter = ',', default_value = "opus,vorbis,mp3,aac-lc")]
        /// A comma-separated list of codecs to match to include only in the sync process.
        sync_codecs: Option<Vec<Codec>>,

        #[arg(
            long,
            long_help = "\
A text file containing a list of folders to sync. Folders listed must be exist within the source directory.

E.g. source -> ~/Music/Library:
    ESAI
    ~/Music/Library/K03
    ~/Music/Library/Various Artists/Stream Palette 4
    Various Artists/Stream Palette 5 -RANKED-"
        )]
        sync_list: Option<String>,
    },
    Completion {
        #[arg(value_enum)]
        shell: Shell,
    },
}

fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Ansi(AnsiColor::Yellow))),
        )
        .header(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Ansi(AnsiColor::Yellow))),
        )
        .literal(Style::new().fg_color(Some(Color::Ansi(AnsiColor::Green))))
        .invalid(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .error(Style::new().bold().fg_color(Some(Color::Ansi(AnsiColor::Red))))
        .valid(
            Style::new()
                .bold()
                .underline()
                .fg_color(Some(Color::Ansi(AnsiColor::Green))),
        )
        .placeholder(Style::new().fg_color(Some(Color::Ansi(AnsiColor::White))))
}
