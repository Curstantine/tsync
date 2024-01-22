use anstyle::{AnsiColor, Color, Style};
use clap::{Parser, Subcommand};

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

        #[arg(
            long,
            short,
            long_help = "\
Specifies the filesystem backend to use for syncing.
By default, the value is inferred from the target directory."
        )]
        fs_backend: Option<FSBackend>,

        #[arg(
            long,
            short,
            long_help = "\
The codec to use while syncing (on-the-fly).
Transcoding will only apply if something is passed to this, else only the files matched by `sync_extensions` will synced.

Opus uses the opusenc binary, instead of the prepackaged symphonia library."
        )]
        codec: Option<Codec>,

        #[arg(
            long,
            short,
            long_help = "\
The bitrate to use while transcoding files matched by `transcode_extensions`.
Only applies if `codec` is set.

Default bitrates:
    - opus, vorbis: 128K
    - mp3: 320K
    - aac-lc: 192K"
        )]
        bitrate: Option<u32>,

        #[arg(
            long,
            value_delimiter = ',',
            default_value = "flac, alac",
            long_help = "A comma-separated list of codecs to match to include in the transcode process."
        )]
        transcode_codecs: Option<Vec<Codec>>,

        #[arg(
            long,
            value_delimiter = ',',
            default_value = "opus, vorbis, mp3, aac-lc",
            long_help = "A comma-separated list of codecs to match to include only in the sync process."
        )]
        sync_codecs: Option<Vec<Codec>>,
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
