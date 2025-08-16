use super::{AudioDecoder, AudioEncoder};
use crate::multimedia::MediaError;
use alloc::vec::Vec;

pub struct OpusDecoder {
    sample_rate: u32,
    channels: u32,
}

impl OpusDecoder {
    pub fn new() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
        }
    }
}

impl AudioDecoder for OpusDecoder {
    fn decode_frame(&mut self, _data: &[u8]) -> Result<Vec<f32>, MediaError> {
        Ok(vec![0.0; 960 * self.channels as usize])
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn get_channels(&self) -> u32 {
        self.channels
    }

    fn reset(&mut self) {}
}

pub struct OpusEncoder {
    sample_rate: u32,
    channels: u32,
    bitrate: u32,
}

impl OpusEncoder {
    pub fn new() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            bitrate: 64000,
        }
    }
}

impl AudioEncoder for OpusEncoder {
    fn encode_frame(&mut self, _samples: &[f32]) -> Result<Vec<u8>, MediaError> {
        Ok(vec![0x4F, 0x70, 0x75, 0x73])
    }

    fn set_sample_rate(&mut self, rate: u32) -> Result<(), MediaError> {
        self.sample_rate = rate;
        Ok(())
    }

    fn set_channels(&mut self, channels: u32) -> Result<(), MediaError> {
        self.channels = channels;
        Ok(())
    }

    fn set_bitrate(&mut self, bitrate: u32) -> Result<(), MediaError> {
        self.bitrate = bitrate;
        Ok(())
    }

    fn flush(&mut self) -> Result<Vec<u8>, MediaError> {
        Ok(Vec::new())
    }
}