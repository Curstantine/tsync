use clap::ValueEnum;

use crate::errors::{Error, Result};

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, PartialOrd)]
pub enum Codec {
    Opus,
    Vorbis,
    Mp3,
    AacLc,

    Flac,
    Alac,
}

impl Codec {
    pub fn get_extension_str(&self) -> &'static str {
        match *self {
            Codec::Opus => "opus",
            Codec::Vorbis => "ogg",
            Codec::Mp3 => "mp3",
            Codec::AacLc => "m4a",
            Codec::Flac => "flac",
            Codec::Alac => "m4a",
        }
    }

    pub fn get_ffmpeg_lib(&self) -> &'static str {
        match *self {
            Codec::Opus => "libopus",
            Codec::Vorbis => "libvorbis",
            Codec::Mp3 => "libmp3lame",
            Codec::AacLc => "aac",
            Codec::Flac => "flac",
            Codec::Alac => "alac",
        }
    }

    pub fn get_matching_bitrate(&self, optional: Option<u32>) -> Result<u32> {
        match optional {
            Some(opt_bitrate) => {
                let (min, max) = match self {
                    Codec::Opus => (6, 256),
                    Codec::Vorbis => (64, 500),
                    Codec::Mp3 => (32, 320),
                    Codec::AacLc => (32, 320),
                    Codec::Flac => (128, 1024),
                    Codec::Alac => (128, 1024),
                };

                if opt_bitrate < min || opt_bitrate > max {
                    let message = format!("Bitrate must be between {min} and {max} for format {self:?}");
                    return Err(Error::descriptive(message));
                }

                Ok(opt_bitrate)
            }
            None => {
                let default = match self {
                    Codec::Opus => 128,
                    Codec::Vorbis => 192,
                    Codec::Mp3 => 192,
                    Codec::AacLc => 192,
                    Codec::Flac => 512,
                    Codec::Alac => 512,
                };

                Ok(default)
            }
        }
    }
}
