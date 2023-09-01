use std::fmt::{self, Display, Formatter};

use crate::errors::{Error, Result};

#[derive(PartialEq, PartialOrd)]
pub enum CodecFormat {
    Opus,
    Vorbis,
    Mp3,
}

impl Display for CodecFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            CodecFormat::Opus => write!(f, "opus"),
            CodecFormat::Vorbis => write!(f, "vorbis"),
            CodecFormat::Mp3 => write!(f, "mp3"),
        }
    }
}

impl CodecFormat {
    pub fn from_str(s: &str) -> Result<CodecFormat> {
        match s {
            "opus" => Ok(CodecFormat::Opus),
            "vorbis" => Ok(CodecFormat::Vorbis),
            "mp3" => Ok(CodecFormat::Mp3),
            _ => Err(Error::Descriptive(format!("codec {}, is not supported!", s))),
        }
    }

    pub fn to_extension_str(&self) -> &str {
        match *self {
            CodecFormat::Opus => "opus",
            CodecFormat::Vorbis => "ogg",
            CodecFormat::Mp3 => "mp3",
        }
    }

    pub fn to_ffmpeg_lib(&self) -> &str {
        match *self {
            CodecFormat::Opus => "libopus",
            CodecFormat::Vorbis => "libvorbis",
            CodecFormat::Mp3 => "libmp3lame",
        }
    }
}
