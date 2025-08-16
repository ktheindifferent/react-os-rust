use super::{AudioDecoder, AudioEncoder};
use crate::multimedia::MediaError;
use alloc::vec::Vec;

pub struct VorbisDecoder {
    sample_rate: u32,
    channels: u32,
}

impl VorbisDecoder {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
        }
    }
}

impl AudioDecoder for VorbisDecoder {
    fn decode_frame(&mut self, _data: &[u8]) -> Result<Vec<f32>, MediaError> {
        Ok(vec![0.0; 1024 * self.channels as usize])
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn get_channels(&self) -> u32 {
        self.channels
    }

    fn reset(&mut self) {}
}

pub struct VorbisEncoder {
    sample_rate: u32,
    channels: u32,
    quality: f32,
}

impl VorbisEncoder {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            quality: 0.5,
        }
    }
}

impl AudioEncoder for VorbisEncoder {
    fn encode_frame(&mut self, _samples: &[f32]) -> Result<Vec<u8>, MediaError> {
        Ok(vec![0x4F, 0x67, 0x67, 0x53])
    }

    fn set_sample_rate(&mut self, rate: u32) -> Result<(), MediaError> {
        self.sample_rate = rate;
        Ok(())
    }

    fn set_channels(&mut self, channels: u32) -> Result<(), MediaError> {
        self.channels = channels;
        Ok(())
    }

    fn set_bitrate(&mut self, _bitrate: u32) -> Result<(), MediaError> {
        Ok(())
    }

    fn flush(&mut self) -> Result<Vec<u8>, MediaError> {
        Ok(Vec::new())
    }
}