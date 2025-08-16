// Audio Codec Support
use super::{AudioFormat, SampleFormat};
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::boxed::Box;
use crate::serial_println;

// Codec Type
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CodecType {
    Pcm,        // Raw PCM
    Mp3,        // MPEG Layer 3
    Aac,        // Advanced Audio Coding
    Vorbis,     // Ogg Vorbis
    Flac,       // Free Lossless Audio Codec
    Opus,       // Opus codec
    Wma,        // Windows Media Audio
}

// Codec Information
#[derive(Debug, Clone)]
pub struct CodecInfo {
    pub codec_type: CodecType,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub supported_rates: Vec<u32>,
    pub supported_formats: Vec<SampleFormat>,
    pub max_channels: u8,
}

// Base Codec Trait
pub trait AudioCodec: Send + Sync {
    fn get_info(&self) -> CodecInfo;
    fn encode(&mut self, pcm: &[f32], format: &AudioFormat) -> Result<Vec<u8>, &'static str>;
    fn decode(&mut self, data: &[u8]) -> Result<(Vec<f32>, AudioFormat), &'static str>;
    fn reset(&mut self);
}

// PCM Codec (no compression)
pub struct PcmCodec {
    info: CodecInfo,
}

impl PcmCodec {
    pub fn new() -> Self {
        Self {
            info: CodecInfo {
                codec_type: CodecType::Pcm,
                name: String::from("PCM"),
                vendor: String::from("Standard"),
                version: String::from("1.0"),
                supported_rates: vec![8000, 11025, 16000, 22050, 44100, 48000, 96000, 192000],
                supported_formats: vec![
                    SampleFormat::U8,
                    SampleFormat::S16LE,
                    SampleFormat::S24LE,
                    SampleFormat::S32LE,
                    SampleFormat::F32LE,
                ],
                max_channels: 8,
            },
        }
    }
}

impl AudioCodec for PcmCodec {
    fn get_info(&self) -> CodecInfo {
        self.info.clone()
    }
    
    fn encode(&mut self, pcm: &[f32], format: &AudioFormat) -> Result<Vec<u8>, &'static str> {
        let mut output = Vec::new();
        
        match format.format {
            SampleFormat::U8 => {
                for &sample in pcm {
                    let value = ((sample * 128.0) + 128.0).clamp(0.0, 255.0) as u8;
                    output.push(value);
                }
            }
            SampleFormat::S16LE => {
                for &sample in pcm {
                    let value = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            SampleFormat::S24LE => {
                for &sample in pcm {
                    let value = (sample * 8388607.0).clamp(-8388608.0, 8388607.0) as i32;
                    output.push((value & 0xFF) as u8);
                    output.push(((value >> 8) & 0xFF) as u8);
                    output.push(((value >> 16) & 0xFF) as u8);
                }
            }
            SampleFormat::S32LE => {
                for &sample in pcm {
                    let value = (sample * 2147483647.0).clamp(-2147483648.0, 2147483647.0) as i32;
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            SampleFormat::F32LE => {
                for &sample in pcm {
                    output.extend_from_slice(&sample.to_le_bytes());
                }
            }
        }
        
        Ok(output)
    }
    
    fn decode(&mut self, data: &[u8]) -> Result<(Vec<f32>, AudioFormat), &'static str> {
        // For PCM, we need to know the format beforehand
        // This is a simplified implementation
        let format = AudioFormat {
            sample_rate: 48000,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 512,
        };
        
        let mut samples = Vec::new();
        
        // Assume S16LE for now
        for chunk in data.chunks_exact(2) {
            let value = i16::from_le_bytes([chunk[0], chunk[1]]);
            samples.push(value as f32 / 32768.0);
        }
        
        Ok((samples, format))
    }
    
    fn reset(&mut self) {
        // PCM codec has no state to reset
    }
}

// ADPCM Codec (Simple compression)
pub struct AdpcmCodec {
    info: CodecInfo,
    prev_sample: i16,
    step_index: usize,
}

impl AdpcmCodec {
    pub fn new() -> Self {
        Self {
            info: CodecInfo {
                codec_type: CodecType::Pcm, // Using PCM type for ADPCM
                name: String::from("ADPCM"),
                vendor: String::from("IMA"),
                version: String::from("1.0"),
                supported_rates: vec![8000, 11025, 16000, 22050, 44100, 48000],
                supported_formats: vec![SampleFormat::S16LE],
                max_channels: 2,
            },
            prev_sample: 0,
            step_index: 0,
        }
    }
    
    const STEP_TABLE: [i16; 89] = [
        7, 8, 9, 10, 11, 12, 13, 14, 16, 17,
        19, 21, 23, 25, 28, 31, 34, 37, 41, 45,
        50, 55, 60, 66, 73, 80, 88, 97, 107, 118,
        130, 143, 157, 173, 190, 209, 230, 253, 279, 307,
        337, 371, 408, 449, 494, 544, 598, 658, 724, 796,
        876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
        2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358,
        5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899,
        15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767
    ];
    
    const INDEX_TABLE: [i8; 16] = [
        -1, -1, -1, -1, 2, 4, 6, 8,
        -1, -1, -1, -1, 2, 4, 6, 8
    ];
    
    fn encode_sample(&mut self, sample: i16) -> u8 {
        let step = Self::STEP_TABLE[self.step_index];
        let diff = sample - self.prev_sample;
        let sign = if diff < 0 { 8 } else { 0 };
        let abs_diff = diff.abs();
        
        let mut nibble = 0;
        let mut predictor = 0;
        
        if abs_diff >= step {
            nibble |= 4;
            predictor += step;
        }
        if abs_diff >= step / 2 {
            nibble |= 2;
            predictor += step / 2;
        }
        if abs_diff >= step / 4 {
            nibble |= 1;
            predictor += step / 4;
        }
        
        if sign != 0 {
            self.prev_sample -= predictor;
        } else {
            self.prev_sample += predictor;
        }
        
        self.prev_sample = self.prev_sample.clamp(-32768, 32767);
        
        self.step_index = (self.step_index as i8 + Self::INDEX_TABLE[nibble as usize]) as usize;
        self.step_index = self.step_index.clamp(0, 88);
        
        sign | nibble
    }
    
    fn decode_nibble(&mut self, nibble: u8) -> i16 {
        let step = Self::STEP_TABLE[self.step_index];
        let mut diff = 0;
        
        if nibble & 4 != 0 { diff += step; }
        if nibble & 2 != 0 { diff += step >> 1; }
        if nibble & 1 != 0 { diff += step >> 2; }
        diff += step >> 3;
        
        if nibble & 8 != 0 {
            self.prev_sample -= diff;
        } else {
            self.prev_sample += diff;
        }
        
        self.prev_sample = self.prev_sample.clamp(-32768, 32767);
        
        self.step_index = (self.step_index as i8 + Self::INDEX_TABLE[nibble as usize]) as usize;
        self.step_index = self.step_index.clamp(0, 88);
        
        self.prev_sample
    }
}

impl AudioCodec for AdpcmCodec {
    fn get_info(&self) -> CodecInfo {
        self.info.clone()
    }
    
    fn encode(&mut self, pcm: &[f32], format: &AudioFormat) -> Result<Vec<u8>, &'static str> {
        let mut output = Vec::new();
        
        // Convert to S16 and encode
        for pair in pcm.chunks(2) {
            let mut byte = 0u8;
            
            for (i, &sample) in pair.iter().enumerate() {
                let s16_sample = (sample * 32767.0) as i16;
                let nibble = self.encode_sample(s16_sample);
                
                if i == 0 {
                    byte = nibble;
                } else {
                    byte |= nibble << 4;
                }
            }
            
            output.push(byte);
        }
        
        Ok(output)
    }
    
    fn decode(&mut self, data: &[u8]) -> Result<(Vec<f32>, AudioFormat), &'static str> {
        let mut samples = Vec::new();
        
        for &byte in data {
            // Decode low nibble
            let sample1 = self.decode_nibble(byte & 0x0F);
            samples.push(sample1 as f32 / 32768.0);
            
            // Decode high nibble
            let sample2 = self.decode_nibble(byte >> 4);
            samples.push(sample2 as f32 / 32768.0);
        }
        
        let format = AudioFormat {
            sample_rate: 48000,
            channels: 1,
            format: SampleFormat::S16LE,
            buffer_size: 512,
        };
        
        Ok((samples, format))
    }
    
    fn reset(&mut self) {
        self.prev_sample = 0;
        self.step_index = 0;
    }
}

// Codec Manager
pub struct CodecManager {
    codecs: Vec<Box<dyn AudioCodec>>,
}

impl CodecManager {
    pub fn new() -> Self {
        let mut manager = Self {
            codecs: Vec::new(),
        };
        
        // Register built-in codecs
        manager.register(Box::new(PcmCodec::new()));
        manager.register(Box::new(AdpcmCodec::new()));
        
        manager
    }
    
    pub fn register(&mut self, codec: Box<dyn AudioCodec>) {
        let info = codec.get_info();
        serial_println!("Codec: Registered {} codec v{}", info.name, info.version);
        self.codecs.push(codec);
    }
    
    pub fn find_codec(&self, codec_type: CodecType) -> Option<&dyn AudioCodec> {
        self.codecs.iter()
            .find(|c| c.get_info().codec_type == codec_type)
            .map(|c| c.as_ref())
    }
    
    pub fn list_codecs(&self) -> Vec<CodecInfo> {
        self.codecs.iter()
            .map(|c| c.get_info())
            .collect()
    }
}