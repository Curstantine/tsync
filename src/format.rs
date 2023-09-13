use std::fmt::{self, Display, Formatter};

use crate::errors::{Error, Result};

#[derive(PartialEq, PartialOrd)]
pub enum CodecFormat {
    Opus,
    LibOpus,
    Vorbis,
    Mp3,
}

impl Display for CodecFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            CodecFormat::Opus => write!(f, "opus"),
            CodecFormat::LibOpus => write!(f, "libopus"),
            CodecFormat::Vorbis => write!(f, "vorbis"),
            CodecFormat::Mp3 => write!(f, "mp3"),
        }
    }
}

impl CodecFormat {
    pub fn from_str<S: AsRef<str> + Display>(str: S) -> Result<CodecFormat> {
        match str.as_ref() {
            "opus" => Ok(CodecFormat::Opus),
            "libopus" => Ok(CodecFormat::LibOpus),
            "vorbis" => Ok(CodecFormat::Vorbis),
            "mp3" => Ok(CodecFormat::Mp3),
            _ => Err(Error::descriptive(format!("codec {}, is not supported!", str))),
        }
    }

    pub fn get_extension_str(&self) -> &str {
        match *self {
            CodecFormat::Opus | CodecFormat::LibOpus => "opus",
            CodecFormat::Vorbis => "ogg",
            CodecFormat::Mp3 => "mp3",
        }
    }

    pub fn get_ffmpeg_lib(&self) -> &str {
        match *self {
            CodecFormat::Opus => panic!("opus codec is not supported by ffmpeg, use libopus instead!"),
            CodecFormat::LibOpus => "libopus",
            CodecFormat::Vorbis => "libvorbis",
            CodecFormat::Mp3 => "libmp3lame",
        }
    }

    pub fn get_matching_bitrate(&self, optional: Option<u32>) -> Result<u32> {
        match optional {
            Some(opt_bitrate) => {
                let (min, max) = match self {
                    CodecFormat::Opus | CodecFormat::LibOpus => (6, 256),
                    CodecFormat::Vorbis => (64, 500),
                    CodecFormat::Mp3 => (32, 320),
                };

                if opt_bitrate < min || opt_bitrate > max {
                    let message = format!("Bitrate must be between {} and {} for format {}", min, max, self);
                    return Err(Error::descriptive(message));
                }

                Ok(opt_bitrate)
            }
            None => {
                let default = match self {
                    CodecFormat::Opus | CodecFormat::LibOpus => 128,
                    CodecFormat::Vorbis => 192,
                    CodecFormat::Mp3 => 192,
                };

                Ok(default)
            }
        }
    }
}
