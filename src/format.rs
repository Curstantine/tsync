use std::{fs::File, path::Path};

use clap::ValueEnum;
use symphonia::core::{
    codecs::{
        CodecParameters,
        audio::{
            AudioCodecId,
            well_known::{CODEC_ID_AAC, CODEC_ID_ALAC, CODEC_ID_FLAC, CODEC_ID_MP3, CODEC_ID_OPUS, CODEC_ID_VORBIS},
        },
    },
    formats::{FormatOptions, Track, probe::Hint},
    io::MediaSourceStream,
    meta::MetadataOptions,
};

use crate::errors::{Error, Result};

#[derive(Debug)]
pub struct TrackData {
    pub codec: Codec,
}

pub fn get_track_data(path: &Path, extension: &str) -> Result<TrackData> {
    let path_str = path.to_string_lossy().to_string();
    let source = File::open(path).map_err(|e| Error::from(e).with_context(path_str.clone()))?;

    let mss = MediaSourceStream::new(Box::new(source), Default::default());
    let meta_opts: MetadataOptions = Default::default();
    let fmt_opts: FormatOptions = Default::default();
    let mut hint = Hint::new();
    hint.with_extension(extension);

    let format = symphonia::default::get_probe()
        .probe(&hint, mss, fmt_opts, meta_opts)
        .map_err(|e| Error::descriptive(format!("Failed to probe media format: {e}")).with_context(path_str.clone()))?;

    probe_track(format.tracks()).map_err(|e| e.with_context(path_str))
}

fn probe_track(tracks: &[Track]) -> Result<TrackData> {
    match tracks.first().map(|t| t.codec_params.as_ref()) {
        Some(Some(CodecParameters::Audio(codec_type))) => {
            let codec = Codec::from_symphonia(codec_type.codec)
                .ok_or_else(|| Error::descriptive(format!("Unsupported codec: {codec_type:#?}")))?;
            Ok(TrackData { codec })
        }
        _ => Err(Error::descriptive("Track metadata is not available")),
    }
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
    pub fn from_symphonia(codec_type: AudioCodecId) -> Option<Codec> {
        let codec = match codec_type {
            CODEC_ID_OPUS => Codec::Opus,
            CODEC_ID_VORBIS => Codec::Vorbis,
            CODEC_ID_MP3 => Codec::Mp3,
            CODEC_ID_AAC => Codec::AacLc,
            CODEC_ID_FLAC => Codec::Flac,
            CODEC_ID_ALAC => Codec::Alac,
            _ => return None,
        };

        Some(codec)
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
