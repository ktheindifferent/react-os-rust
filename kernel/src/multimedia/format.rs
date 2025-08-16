use alloc::string::String;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use super::{MediaType, PixelFormat, AudioFormat, MediaFormat};

#[derive(Debug, Clone)]
pub struct FormatContext {
    pub streams: Vec<StreamInfo>,
    pub duration: Option<i64>,
    pub bit_rate: Option<u64>,
    pub metadata: BTreeMap<String, String>,
    pub format_name: String,
    pub format_long_name: String,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub index: usize,
    pub media_type: MediaType,
    pub codec_id: CodecId,
    pub codec_params: CodecParameters,
    pub time_base: Rational,
    pub start_time: Option<i64>,
    pub duration: Option<i64>,
    pub nb_frames: Option<u64>,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecId {
    None,
    
    // Video codecs
    H264,
    H265,
    VP8,
    VP9,
    AV1,
    MPEG2Video,
    MPEG4,
    MJPEG,
    ProRes,
    DNxHD,
    
    // Audio codecs  
    MP3,
    AAC,
    AC3,
    EAC3,
    DTS,
    Vorbis,
    Opus,
    FLAC,
    ALAC,
    PCM_S16LE,
    PCM_S16BE,
    PCM_S24LE,
    PCM_S24BE,
    PCM_S32LE,
    PCM_S32BE,
    PCM_F32LE,
    PCM_F32BE,
    
    // Subtitle codecs
    SubRip,
    ASS,
    WebVTT,
    DVDSub,
    PGS,
}

#[derive(Debug, Clone)]
pub struct CodecParameters {
    pub codec_type: MediaType,
    pub codec_id: CodecId,
    pub bit_rate: Option<u64>,
    pub extra_data: Vec<u8>,
    
    // Video parameters
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub pixel_format: Option<PixelFormat>,
    pub frame_rate: Option<Rational>,
    pub aspect_ratio: Option<Rational>,
    pub color_space: Option<ColorSpace>,
    pub color_range: Option<ColorRange>,
    pub field_order: Option<FieldOrder>,
    
    // Audio parameters
    pub sample_rate: Option<u32>,
    pub channels: Option<u32>,
    pub channel_layout: Option<u64>,
    pub audio_format: Option<AudioFormat>,
    pub frame_size: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    RGB,
    BT709,
    BT601,
    BT2020,
    SMPTE170M,
    SMPTE240M,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorRange {
    Limited,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldOrder {
    Progressive,
    TopFirst,
    BottomFirst,
}

#[derive(Debug, Clone, Copy)]
pub struct Rational {
    pub num: u32,
    pub den: u32,
}

impl Rational {
    pub fn new(num: u32, den: u32) -> Self {
        let gcd = Self::gcd(num, den);
        Self {
            num: num / gcd,
            den: den / gcd,
        }
    }

    fn gcd(a: u32, b: u32) -> u32 {
        if b == 0 {
            a
        } else {
            Self::gcd(b, a % b)
        }
    }

    pub fn as_f32(&self) -> f32 {
        self.num as f32 / self.den as f32
    }
}

pub struct FormatNegotiator {
    supported_formats: Vec<FormatCapability>,
}

#[derive(Debug, Clone)]
pub struct FormatCapability {
    pub media_type: MediaType,
    pub codecs: Vec<CodecId>,
    pub pixel_formats: Option<Vec<PixelFormat>>,
    pub audio_formats: Option<Vec<AudioFormat>>,
    pub sample_rates: Option<Vec<u32>>,
    pub channel_counts: Option<Vec<u32>>,
    pub resolutions: Option<Vec<Resolution>>,
    pub frame_rates: Option<Vec<Rational>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl FormatNegotiator {
    pub fn new() -> Self {
        Self {
            supported_formats: Vec::new(),
        }
    }

    pub fn add_capability(&mut self, capability: FormatCapability) {
        self.supported_formats.push(capability);
    }

    pub fn negotiate(&self, requested: &MediaFormat) -> Result<MediaFormat, FormatError> {
        for capability in &self.supported_formats {
            if capability.media_type != requested.media_type {
                continue;
            }

            let codec_id = codec_from_string(&requested.codec);
            if !capability.codecs.contains(&codec_id) {
                continue;
            }

            let mut negotiated = requested.clone();

            // Negotiate video parameters
            if requested.media_type == MediaType::Video {
                if let Some(ref pixel_formats) = capability.pixel_formats {
                    if let Some(requested_format) = requested.pixel_format {
                        if !pixel_formats.contains(&requested_format) {
                            negotiated.pixel_format = Some(pixel_formats[0]);
                        }
                    }
                }

                if let Some(ref resolutions) = capability.resolutions {
                    if let (Some(width), Some(height)) = (requested.width, requested.height) {
                        let requested_res = Resolution { width, height };
                        if !resolutions.iter().any(|r| r.width == requested_res.width && r.height == requested_res.height) {
                            let closest = Self::find_closest_resolution(&requested_res, resolutions);
                            negotiated.width = Some(closest.width);
                            negotiated.height = Some(closest.height);
                        }
                    }
                }
            }

            // Negotiate audio parameters
            if requested.media_type == MediaType::Audio {
                if let Some(ref audio_formats) = capability.audio_formats {
                    if let Some(requested_format) = requested.audio_format {
                        if !audio_formats.contains(&requested_format) {
                            negotiated.audio_format = Some(audio_formats[0]);
                        }
                    }
                }

                if let Some(ref sample_rates) = capability.sample_rates {
                    if let Some(requested_rate) = requested.sample_rate {
                        if !sample_rates.contains(&requested_rate) {
                            let closest = Self::find_closest_value(requested_rate, sample_rates);
                            negotiated.sample_rate = Some(closest);
                        }
                    }
                }
            }

            return Ok(negotiated);
        }

        Err(FormatError::UnsupportedFormat)
    }

    fn find_closest_resolution(target: &Resolution, resolutions: &[Resolution]) -> Resolution {
        resolutions.iter()
            .min_by_key(|r| {
                let width_diff = (r.width as i32 - target.width as i32).abs();
                let height_diff = (r.height as i32 - target.height as i32).abs();
                width_diff + height_diff
            })
            .copied()
            .unwrap_or(Resolution { width: 1920, height: 1080 })
    }

    fn find_closest_value(target: u32, values: &[u32]) -> u32 {
        values.iter()
            .min_by_key(|&&v| (v as i32 - target as i32).abs())
            .copied()
            .unwrap_or(target)
    }
}

fn codec_from_string(codec: &str) -> CodecId {
    match codec.to_lowercase().as_str() {
        "h264" | "avc" => CodecId::H264,
        "h265" | "hevc" => CodecId::H265,
        "vp8" => CodecId::VP8,
        "vp9" => CodecId::VP9,
        "av1" => CodecId::AV1,
        "mp3" => CodecId::MP3,
        "aac" => CodecId::AAC,
        "vorbis" => CodecId::Vorbis,
        "opus" => CodecId::Opus,
        "flac" => CodecId::FLAC,
        _ => CodecId::None,
    }
}

#[derive(Debug)]
pub enum FormatError {
    UnsupportedFormat,
    InvalidParameters,
    NegotiationFailed,
}