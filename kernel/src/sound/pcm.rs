// PCM (Pulse Code Modulation) Audio Processing
use super::{AudioFormat, SampleFormat, AudioBuffer};
use alloc::vec::Vec;
use alloc::vec;
use alloc::collections::VecDeque;
use crate::serial_println;

// PCM Ring Buffer for audio streaming
pub struct PcmRingBuffer {
    buffer: Vec<u8>,
    capacity: usize,
    read_pos: usize,
    write_pos: usize,
    available: usize,
}

impl PcmRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: vec![0u8; capacity],
            capacity,
            read_pos: 0,
            write_pos: 0,
            available: 0,
        }
    }
    
    pub fn write(&mut self, data: &[u8]) -> usize {
        let mut written = 0;
        
        for &byte in data {
            if self.available >= self.capacity {
                break; // Buffer full
            }
            
            self.buffer[self.write_pos] = byte;
            self.write_pos = (self.write_pos + 1) % self.capacity;
            self.available += 1;
            written += 1;
        }
        
        written
    }
    
    pub fn read(&mut self, data: &mut [u8]) -> usize {
        let mut read = 0;
        
        for byte in data.iter_mut() {
            if self.available == 0 {
                break; // Buffer empty
            }
            
            *byte = self.buffer[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity;
            self.available -= 1;
            read += 1;
        }
        
        read
    }
    
    pub fn available(&self) -> usize {
        self.available
    }
    
    pub fn free_space(&self) -> usize {
        self.capacity - self.available
    }
    
    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
        self.available = 0;
        self.buffer.fill(0);
    }
}

// PCM Channel for managing audio data flow
pub struct PcmChannel {
    format: AudioFormat,
    ring_buffer: PcmRingBuffer,
    underrun_count: u32,
    overrun_count: u32,
}

impl PcmChannel {
    pub fn new(format: AudioFormat, buffer_size: usize) -> Self {
        Self {
            format: format.clone(),
            ring_buffer: PcmRingBuffer::new(buffer_size),
            underrun_count: 0,
            overrun_count: 0,
        }
    }
    
    pub fn write_frames(&mut self, frames: &[u8]) -> Result<usize, &'static str> {
        let frame_size = self.format.channels as usize * self.format.format.bytes_per_sample();
        let num_frames = frames.len() / frame_size;
        
        if frames.len() % frame_size != 0 {
            return Err("Invalid frame size");
        }
        
        let written = self.ring_buffer.write(frames);
        
        if written < frames.len() {
            self.overrun_count += 1;
            serial_println!("PCM: Buffer overrun ({})", self.overrun_count);
        }
        
        Ok(written / frame_size)
    }
    
    pub fn read_frames(&mut self, frames: &mut [u8]) -> Result<usize, &'static str> {
        let frame_size = self.format.channels as usize * self.format.format.bytes_per_sample();
        let num_frames = frames.len() / frame_size;
        
        if frames.len() % frame_size != 0 {
            return Err("Invalid frame size");
        }
        
        let read = self.ring_buffer.read(frames);
        
        if read < frames.len() {
            self.underrun_count += 1;
            serial_println!("PCM: Buffer underrun ({})", self.underrun_count);
            
            // Fill remaining with silence
            for i in read..frames.len() {
                frames[i] = 0;
            }
        }
        
        Ok(read / frame_size)
    }
    
    pub fn available_frames(&self) -> usize {
        let frame_size = self.format.channels as usize * self.format.format.bytes_per_sample();
        self.ring_buffer.available() / frame_size
    }
    
    pub fn set_format(&mut self, format: AudioFormat) {
        self.format = format;
        self.ring_buffer.clear();
    }
}

// PCM Processor for audio effects and transformations
pub struct PcmProcessor {
    input_format: AudioFormat,
    output_format: AudioFormat,
}

impl PcmProcessor {
    pub fn new(input: AudioFormat, output: AudioFormat) -> Self {
        Self {
            input_format: input,
            output_format: output,
        }
    }
    
    pub fn process(&self, input: &[u8]) -> Vec<u8> {
        // Convert to floating point for processing
        let samples = self.to_float_samples(input);
        
        // Apply any necessary conversions
        let processed = if self.input_format.sample_rate != self.output_format.sample_rate {
            self.resample(&samples)
        } else {
            samples
        };
        
        // Convert back to output format
        self.from_float_samples(&processed)
    }
    
    fn to_float_samples(&self, data: &[u8]) -> Vec<f32> {
        let mut samples = Vec::new();
        
        match self.input_format.format {
            SampleFormat::U8 => {
                for &byte in data {
                    samples.push((byte as f32 - 128.0) / 128.0);
                }
            }
            SampleFormat::S16LE => {
                for chunk in data.chunks_exact(2) {
                    let value = i16::from_le_bytes([chunk[0], chunk[1]]);
                    samples.push(value as f32 / 32768.0);
                }
            }
            SampleFormat::S24LE => {
                for chunk in data.chunks_exact(3) {
                    let value = (chunk[0] as i32) | 
                               ((chunk[1] as i32) << 8) | 
                               ((chunk[2] as i32) << 16);
                    let value = if value & 0x800000 != 0 {
                        value | 0xFF000000u32 as i32
                    } else {
                        value
                    };
                    samples.push(value as f32 / 8388608.0);
                }
            }
            SampleFormat::S32LE => {
                for chunk in data.chunks_exact(4) {
                    let value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(value as f32 / 2147483648.0);
                }
            }
            SampleFormat::F32LE => {
                for chunk in data.chunks_exact(4) {
                    let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(value);
                }
            }
        }
        
        samples
    }
    
    fn from_float_samples(&self, samples: &[f32]) -> Vec<u8> {
        let mut data = Vec::new();
        
        match self.output_format.format {
            SampleFormat::U8 => {
                for &sample in samples {
                    let value = ((sample * 128.0) + 128.0).clamp(0.0, 255.0) as u8;
                    data.push(value);
                }
            }
            SampleFormat::S16LE => {
                for &sample in samples {
                    let value = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                    data.extend_from_slice(&value.to_le_bytes());
                }
            }
            SampleFormat::S24LE => {
                for &sample in samples {
                    let value = (sample * 8388607.0).clamp(-8388608.0, 8388607.0) as i32;
                    data.push((value & 0xFF) as u8);
                    data.push(((value >> 8) & 0xFF) as u8);
                    data.push(((value >> 16) & 0xFF) as u8);
                }
            }
            SampleFormat::S32LE => {
                for &sample in samples {
                    let value = (sample * 2147483647.0).clamp(-2147483648.0, 2147483647.0) as i32;
                    data.extend_from_slice(&value.to_le_bytes());
                }
            }
            SampleFormat::F32LE => {
                for &sample in samples {
                    data.extend_from_slice(&sample.to_le_bytes());
                }
            }
        }
        
        data
    }
    
    fn resample(&self, input: &[f32]) -> Vec<f32> {
        let ratio = self.output_format.sample_rate as f32 / self.input_format.sample_rate as f32;
        let output_len = (input.len() as f32 * ratio) as usize;
        let mut output = vec![0.0; output_len];
        
        // Simple linear interpolation
        for i in 0..output_len {
            let src_pos = i as f32 / ratio;
            let src_idx = src_pos as usize;
            let frac = src_pos - src_idx as f32;
            
            if src_idx < input.len() - 1 {
                output[i] = input[src_idx] * (1.0 - frac) + input[src_idx + 1] * frac;
            } else if src_idx < input.len() {
                output[i] = input[src_idx];
            }
        }
        
        output
    }
}

// PCM Statistics
pub struct PcmStats {
    pub frames_played: u64,
    pub frames_recorded: u64,
    pub underruns: u32,
    pub overruns: u32,
    pub peak_level: f32,
    pub rms_level: f32,
}

impl PcmStats {
    pub fn new() -> Self {
        Self {
            frames_played: 0,
            frames_recorded: 0,
            underruns: 0,
            overruns: 0,
            peak_level: 0.0,
            rms_level: 0.0,
        }
    }
    
    pub fn update_levels(&mut self, samples: &[f32]) {
        if samples.is_empty() {
            return;
        }
        
        // Calculate peak level
        self.peak_level = samples.iter()
            .map(|s| s.abs())
            .fold(self.peak_level, f32::max);
        
        // Calculate RMS level
        let sum_squares: f32 = samples.iter()
            .map(|s| s * s)
            .sum();
        self.rms_level = sqrt_approx(sum_squares / samples.len() as f32);
    }
    
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

// Math approximation for no_std environment
fn sqrt_approx(x: f32) -> f32 {
    if x <= 0.0 {
        return 0.0;
    }
    
    // Newton-Raphson method
    let mut guess = x;
    let mut prev = 0.0;
    
    while (guess - prev).abs() > 0.0001 {
        prev = guess;
        guess = (guess + x / guess) * 0.5;
    }
    
    guess
}