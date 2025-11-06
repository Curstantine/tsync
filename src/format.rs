use std::{fs::File, path::Path};

use clap::ValueEnum;
use symphonia::core::{
    codecs::{
        CODEC_TYPE_AAC, CODEC_TYPE_ALAC, CODEC_TYPE_FLAC, CODEC_TYPE_MP3, CODEC_TYPE_OPUS, CODEC_TYPE_VORBIS, CodecType,
    },
    formats::Track,
    io::MediaSourceStream,
    probe::Hint,
};

use crate::errors::{Error, Result};

#[derive(Debug)]
pub struct TrackData {
    pub codec: Codec,
}

pub fn get_track_data(path: &Path, extension: &str) -> Result<TrackData> {
    let source = File::open(path)?;

    let mss = MediaSourceStream::new(Box::new(source), Default::default());
    let mut hint = Hint::new();
    hint.with_extension(extension);

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &Default::default(), &Default::default())
        .map_err(|e| Error::descriptive(format!("Failed to probe format: {e}")))?;

    probe_track(probed.format.tracks()).map_err(|e| e.with_context(path.to_string_lossy().into_owned()))
}

fn probe_track(tracks: &[Track]) -> Result<TrackData> {
    let track = tracks
        .first()
        .ok_or_else(|| Error::descriptive("No tracks found in file"))?;

    Ok(TrackData {
        codec: Codec::from_symphonia(track.codec_params.codec),
    })
}

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
    pub fn from_symphonia(codec_type: CodecType) -> Codec {
        match codec_type {
            CODEC_TYPE_OPUS => Codec::Opus,
            CODEC_TYPE_VORBIS => Codec::Vorbis,
            CODEC_TYPE_MP3 => Codec::Mp3,
            CODEC_TYPE_AAC => Codec::AacLc,
            CODEC_TYPE_FLAC => Codec::Flac,
            CODEC_TYPE_ALAC => Codec::Alac,
            _ => unimplemented!("Unknown codec {codec_type:#?}"),
        }
    }

    pub fn extenstion_str(&self) -> &'static str {
        match *self {
            Codec::Opus => "opus",
            Codec::Vorbis => "ogg",
            Codec::Mp3 => "mp3",
            Codec::AacLc => "m4a",
            Codec::Flac => "flac",
            Codec::Alac => "m4a",
        }
    }

    pub fn ffmpeg_lib(&self) -> &'static str {
        match *self {
            Codec::Opus => "libopus",
            Codec::Vorbis => "libvorbis",
            Codec::Mp3 => "libmp3lame",
            Codec::AacLc => "aac",
            Codec::Flac => "flac",
            Codec::Alac => "alac",
        }
    }

    pub fn matching_bitrate(&self, optional: Option<u32>) -> Result<u32> {
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
