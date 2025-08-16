use super::{AudioDecoder, AudioEncoder};
use crate::multimedia::{MediaError, AudioFormat};
use alloc::vec::Vec;

pub struct PcmDecoder {
    format: AudioFormat,
    sample_rate: u32,
    channels: u32,
}

impl PcmDecoder {
    pub fn new(format: AudioFormat, sample_rate: u32, channels: u32) -> Self {
        Self {
            format,
            sample_rate,
            channels,
        }
    }

    fn bytes_per_sample(&self) -> usize {
        match self.format {
            AudioFormat::U8 => 1,
            AudioFormat::S16LE | AudioFormat::S16BE => 2,
            AudioFormat::S24LE | AudioFormat::S24BE => 3,
            AudioFormat::S32LE | AudioFormat::S32BE | AudioFormat::F32LE | AudioFormat::F32BE => 4,
            AudioFormat::F64LE | AudioFormat::F64BE => 8,
        }
    }
}

impl AudioDecoder for PcmDecoder {
    fn decode_frame(&mut self, data: &[u8]) -> Result<Vec<f32>, MediaError> {
        let bytes_per_sample = self.bytes_per_sample();
        let sample_count = data.len() / bytes_per_sample;
        let mut samples = Vec::with_capacity(sample_count);

        match self.format {
            AudioFormat::U8 => {
                for byte in data {
                    let sample = (*byte as f32 - 128.0) / 128.0;
                    samples.push(sample);
                }
            }
            AudioFormat::S16LE => {
                for chunk in data.chunks_exact(2) {
                    let value = i16::from_le_bytes([chunk[0], chunk[1]]);
                    samples.push(value as f32 / 32768.0);
                }
            }
            AudioFormat::S16BE => {
                for chunk in data.chunks_exact(2) {
                    let value = i16::from_be_bytes([chunk[0], chunk[1]]);
                    samples.push(value as f32 / 32768.0);
                }
            }
            AudioFormat::S24LE => {
                for chunk in data.chunks_exact(3) {
                    let value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], 0]) >> 8;
                    samples.push(value as f32 / 8388608.0);
                }
            }
            AudioFormat::S24BE => {
                for chunk in data.chunks_exact(3) {
                    let value = i32::from_be_bytes([0, chunk[0], chunk[1], chunk[2]]) >> 8;
                    samples.push(value as f32 / 8388608.0);
                }
            }
            AudioFormat::S32LE => {
                for chunk in data.chunks_exact(4) {
                    let value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(value as f32 / 2147483648.0);
                }
            }
            AudioFormat::S32BE => {
                for chunk in data.chunks_exact(4) {
                    let value = i32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(value as f32 / 2147483648.0);
                }
            }
            AudioFormat::F32LE => {
                for chunk in data.chunks_exact(4) {
                    let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(value);
                }
            }
            AudioFormat::F32BE => {
                for chunk in data.chunks_exact(4) {
                    let value = f32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(value);
                }
            }
            AudioFormat::F64LE => {
                for chunk in data.chunks_exact(8) {
                    let bytes = [chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7]];
                    let value = f64::from_le_bytes(bytes);
                    samples.push(value as f32);
                }
            }
            AudioFormat::F64BE => {
                for chunk in data.chunks_exact(8) {
                    let bytes = [chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7]];
                    let value = f64::from_be_bytes(bytes);
                    samples.push(value as f32);
                }
            }
        }

        Ok(samples)
    }

    fn get_sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn get_channels(&self) -> u32 {
        self.channels
    }

    fn reset(&mut self) {
        // PCM is stateless, nothing to reset
    }
}

pub struct PcmEncoder {
    format: AudioFormat,
    sample_rate: u32,
    channels: u32,
}

impl PcmEncoder {
    pub fn new(format: AudioFormat, sample_rate: u32, channels: u32) -> Self {
        Self {
            format,
            sample_rate,
            channels,
        }
    }
}

impl AudioEncoder for PcmEncoder {
    fn encode_frame(&mut self, samples: &[f32]) -> Result<Vec<u8>, MediaError> {
        let mut output = Vec::new();

        match self.format {
            AudioFormat::U8 => {
                for &sample in samples {
                    let value = ((sample.clamp(-1.0, 1.0) * 128.0) + 128.0) as u8;
                    output.push(value);
                }
            }
            AudioFormat::S16LE => {
                for &sample in samples {
                    let value = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            AudioFormat::S16BE => {
                for &sample in samples {
                    let value = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                    output.extend_from_slice(&value.to_be_bytes());
                }
            }
            AudioFormat::S24LE => {
                for &sample in samples {
                    let value = (sample.clamp(-1.0, 1.0) * 8388607.0) as i32;
                    let bytes = value.to_le_bytes();
                    output.extend_from_slice(&bytes[0..3]);
                }
            }
            AudioFormat::S24BE => {
                for &sample in samples {
                    let value = (sample.clamp(-1.0, 1.0) * 8388607.0) as i32;
                    let bytes = value.to_be_bytes();
                    output.extend_from_slice(&bytes[1..4]);
                }
            }
            AudioFormat::S32LE => {
                for &sample in samples {
                    let value = (sample.clamp(-1.0, 1.0) * 2147483647.0) as i32;
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            AudioFormat::S32BE => {
                for &sample in samples {
                    let value = (sample.clamp(-1.0, 1.0) * 2147483647.0) as i32;
                    output.extend_from_slice(&value.to_be_bytes());
                }
            }
            AudioFormat::F32LE => {
                for &sample in samples {
                    output.extend_from_slice(&sample.to_le_bytes());
                }
            }
            AudioFormat::F32BE => {
                for &sample in samples {
                    output.extend_from_slice(&sample.to_be_bytes());
                }
            }
            AudioFormat::F64LE => {
                for &sample in samples {
                    let value = sample as f64;
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            AudioFormat::F64BE => {
                for &sample in samples {
                    let value = sample as f64;
                    output.extend_from_slice(&value.to_be_bytes());
                }
            }
        }

        Ok(output)
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
        // PCM doesn't have variable bitrate
        Ok(())
    }

    fn flush(&mut self) -> Result<Vec<u8>, MediaError> {
        // PCM is stateless, nothing to flush
        Ok(Vec::new())
    }
}