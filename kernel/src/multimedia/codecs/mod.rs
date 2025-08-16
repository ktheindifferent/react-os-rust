pub mod mp3;
pub mod aac;
pub mod vorbis;
pub mod flac;
pub mod opus;
pub mod pcm;
pub mod h264;
pub mod h265;
pub mod vp9;
pub mod av1;

use super::{MediaError, MediaFormat};
use super::plugin::{Codec, CodecType, Decoder, Encoder, DecodedFrame, EncoderInput};
use super::format::CodecId;
use alloc::vec::Vec;

pub trait AudioDecoder: Send + Sync {
    fn decode_frame(&mut self, data: &[u8]) -> Result<Vec<f32>, MediaError>;
    fn get_sample_rate(&self) -> u32;
    fn get_channels(&self) -> u32;
    fn reset(&mut self);
}

pub trait AudioEncoder: Send + Sync {
    fn encode_frame(&mut self, samples: &[f32]) -> Result<Vec<u8>, MediaError>;
    fn set_sample_rate(&mut self, rate: u32) -> Result<(), MediaError>;
    fn set_channels(&mut self, channels: u32) -> Result<(), MediaError>;
    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), MediaError>;
    fn flush(&mut self) -> Result<Vec<u8>, MediaError>;
}

pub trait VideoDecoder: Send + Sync {
    fn decode_frame(&mut self, data: &[u8]) -> Result<VideoFrame, MediaError>;
    fn get_width(&self) -> u32;
    fn get_height(&self) -> u32;
    fn get_pixel_format(&self) -> super::PixelFormat;
    fn reset(&mut self);
}

pub trait VideoEncoder: Send + Sync {
    fn encode_frame(&mut self, frame: &VideoFrame) -> Result<Vec<u8>, MediaError>;
    fn set_resolution(&mut self, width: u32, height: u32) -> Result<(), MediaError>;
    fn set_pixel_format(&mut self, format: super::PixelFormat) -> Result<(), MediaError>;
    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), MediaError>;
    fn set_framerate(&mut self, fps: f32) -> Result<(), MediaError>;
    fn request_keyframe(&mut self);
    fn flush(&mut self) -> Result<Vec<u8>, MediaError>;
}

#[derive(Debug, Clone)]
pub struct VideoFrame {
    pub data: Vec<Vec<u8>>,
    pub linesize: Vec<usize>,
    pub width: u32,
    pub height: u32,
    pub pixel_format: super::PixelFormat,
    pub pts: i64,
    pub key_frame: bool,
}

pub struct CodecManager {
    decoders: alloc::collections::BTreeMap<CodecId, alloc::sync::Arc<dyn Decoder>>,
    encoders: alloc::collections::BTreeMap<CodecId, alloc::sync::Arc<dyn Encoder>>,
}

impl CodecManager {
    pub fn new() -> Self {
        let mut manager = Self {
            decoders: alloc::collections::BTreeMap::new(),
            encoders: alloc::collections::BTreeMap::new(),
        };
        
        manager.register_builtin_codecs();
        manager
    }

    fn register_builtin_codecs(&mut self) {
        // Register PCM codecs (always available)
        // MP3, AAC, etc. would be registered here if implementations were available
    }

    pub fn find_decoder(&self, codec_id: CodecId) -> Option<alloc::sync::Arc<dyn Decoder>> {
        self.decoders.get(&codec_id).cloned()
    }

    pub fn find_encoder(&self, codec_id: CodecId) -> Option<alloc::sync::Arc<dyn Encoder>> {
        self.encoders.get(&codec_id).cloned()
    }

    pub fn register_decoder(&mut self, codec_id: CodecId, decoder: alloc::sync::Arc<dyn Decoder>) {
        self.decoders.insert(codec_id, decoder);
    }

    pub fn register_encoder(&mut self, codec_id: CodecId, encoder: alloc::sync::Arc<dyn Encoder>) {
        self.encoders.insert(codec_id, encoder);
    }
}

pub fn probe_codec(data: &[u8]) -> Option<CodecId> {
    // MP3 sync word
    if data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xE0) == 0xE0 {
        return Some(CodecId::MP3);
    }
    
    // AAC ADTS sync word
    if data.len() >= 2 && data[0] == 0xFF && (data[1] & 0xF0) == 0xF0 {
        return Some(CodecId::AAC);
    }
    
    // Ogg signature
    if data.len() >= 4 && &data[0..4] == b"OggS" {
        // Could be Vorbis or Opus
        if data.len() > 28 {
            if data[28..].starts_with(b"\x01vorbis") {
                return Some(CodecId::Vorbis);
            } else if data[28..].starts_with(b"OpusHead") {
                return Some(CodecId::Opus);
            }
        }
    }
    
    // FLAC signature
    if data.len() >= 4 && &data[0..4] == b"fLaC" {
        return Some(CodecId::FLAC);
    }
    
    None
}

pub fn get_codec_name(codec_id: CodecId) -> &'static str {
    match codec_id {
        CodecId::None => "none",
        CodecId::H264 => "H.264/AVC",
        CodecId::H265 => "H.265/HEVC",
        CodecId::VP8 => "VP8",
        CodecId::VP9 => "VP9",
        CodecId::AV1 => "AV1",
        CodecId::MPEG2Video => "MPEG-2 Video",
        CodecId::MPEG4 => "MPEG-4",
        CodecId::MJPEG => "Motion JPEG",
        CodecId::ProRes => "ProRes",
        CodecId::DNxHD => "DNxHD",
        CodecId::MP3 => "MP3",
        CodecId::AAC => "AAC",
        CodecId::AC3 => "AC-3",
        CodecId::EAC3 => "E-AC-3",
        CodecId::DTS => "DTS",
        CodecId::Vorbis => "Vorbis",
        CodecId::Opus => "Opus",
        CodecId::FLAC => "FLAC",
        CodecId::ALAC => "ALAC",
        CodecId::PCM_S16LE => "PCM S16LE",
        CodecId::PCM_S16BE => "PCM S16BE",
        CodecId::PCM_S24LE => "PCM S24LE",
        CodecId::PCM_S24BE => "PCM S24BE",
        CodecId::PCM_S32LE => "PCM S32LE",
        CodecId::PCM_S32BE => "PCM S32BE",
        CodecId::PCM_F32LE => "PCM F32LE",
        CodecId::PCM_F32BE => "PCM F32BE",
        CodecId::SubRip => "SubRip",
        CodecId::ASS => "ASS",
        CodecId::WebVTT => "WebVTT",
        CodecId::DVDSub => "DVD Subtitle",
        CodecId::PGS => "PGS",
    }
}