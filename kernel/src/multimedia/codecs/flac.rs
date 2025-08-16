use super::{AudioDecoder, AudioEncoder};
use crate::multimedia::MediaError;
use alloc::vec::Vec;

pub struct FlacDecoder {
    sample_rate: u32,
    channels: u32,
    bits_per_sample: u32,
}

impl FlacDecoder {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
        }
    }
}

impl AudioDecoder for FlacDecoder {
    fn decode_frame(&mut self, _data: &[u8]) -> Result<Vec<f32>, MediaError> {
        Ok(vec![0.0; 4096 * self.channels as usize])
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn get_channels(&self) -> u32 {
        self.channels
    }

    fn reset(&mut self) {}
}

pub struct FlacEncoder {
    sample_rate: u32,
    channels: u32,
    compression_level: u32,
}

impl FlacEncoder {
    pub fn new() -> Self {
        Self {
            sample_rate: 44100,
            channels: 2,
            compression_level: 5,
        }
    }
}

impl AudioEncoder for FlacEncoder {
    fn encode_frame(&mut self, _samples: &[f32]) -> Result<Vec<u8>, MediaError> {
        Ok(vec![0x66, 0x4C, 0x61, 0x43])
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