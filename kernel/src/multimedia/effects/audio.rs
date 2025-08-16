use alloc::vec::Vec;
use alloc::collections::VecDeque;
use super::{Effect, EffectType};
use crate::multimedia::MediaError;

pub struct Reverb {
    delay_lines: Vec<DelayLine>,
    mix: f32,
    room_size: f32,
    damping: f32,
}

struct DelayLine {
    buffer: VecDeque<f32>,
    delay: usize,
    feedback: f32,
}

impl Reverb {
    pub fn new(sample_rate: u32) -> Self {
        let mut delay_lines = Vec::new();
        
        // Create multiple delay lines for reverb
        for i in 0..4 {
            let delay = (sample_rate as f32 * (0.01 + 0.003 * i as f32)) as usize;
            delay_lines.push(DelayLine {
                buffer: VecDeque::from(vec![0.0; delay]),
                delay,
                feedback: 0.5,
            });
        }
        
        Self {
            delay_lines,
            mix: 0.3,
            room_size: 0.5,
            damping: 0.5,
        }
    }

    fn process_sample(&mut self, input: f32) -> f32 {
        let mut output = input;
        
        for delay_line in &mut self.delay_lines {
            let delayed = delay_line.buffer.pop_front().unwrap_or(0.0);
            let filtered = delayed * (1.0 - self.damping) + delay_line.feedback * self.damping;
            delay_line.buffer.push_back(input + filtered * self.room_size);
            output += delayed * self.mix;
        }
        
        output
    }
}

impl Effect for Reverb {
    fn name(&self) -> &str {
        "Reverb"
    }

    fn effect_type(&self) -> EffectType {
        EffectType::AudioFilter
    }

    fn process(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), MediaError> {
        // Assume input is f32 samples
        let samples = input.len() / 4;
        output.reserve(input.len());
        
        for i in 0..samples {
            let sample = f32::from_le_bytes([
                input[i * 4],
                input[i * 4 + 1],
                input[i * 4 + 2],
                input[i * 4 + 3],
            ]);
            
            let processed = self.process_sample(sample);
            output.extend_from_slice(&processed.to_le_bytes());
        }
        
        Ok(())
    }

    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), MediaError> {
        match name {
            "mix" => self.mix = value.clamp(0.0, 1.0),
            "room_size" => self.room_size = value.clamp(0.0, 1.0),
            "damping" => self.damping = value.clamp(0.0, 1.0),
            _ => return Err(MediaError::InvalidFormat),
        }
        Ok(())
    }

    fn get_parameter(&self, name: &str) -> Option<f32> {
        match name {
            "mix" => Some(self.mix),
            "room_size" => Some(self.room_size),
            "damping" => Some(self.damping),
            _ => None,
        }
    }

    fn reset(&mut self) {
        for delay_line in &mut self.delay_lines {
            delay_line.buffer.clear();
            delay_line.buffer.resize(delay_line.delay, 0.0);
        }
    }
}

pub struct Echo {
    delay_buffer: VecDeque<f32>,
    delay_time: f32,
    feedback: f32,
    mix: f32,
    sample_rate: u32,
}

impl Echo {
    pub fn new(sample_rate: u32) -> Self {
        let delay_samples = (sample_rate as f32 * 0.5) as usize;
        Self {
            delay_buffer: VecDeque::from(vec![0.0; delay_samples]),
            delay_time: 0.5,
            feedback: 0.5,
            mix: 0.5,
            sample_rate,
        }
    }
}

impl Effect for Echo {
    fn name(&self) -> &str {
        "Echo"
    }

    fn effect_type(&self) -> EffectType {
        EffectType::AudioFilter
    }

    fn process(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), MediaError> {
        let samples = input.len() / 4;
        output.reserve(input.len());
        
        for i in 0..samples {
            let sample = f32::from_le_bytes([
                input[i * 4],
                input[i * 4 + 1],
                input[i * 4 + 2],
                input[i * 4 + 3],
            ]);
            
            let delayed = self.delay_buffer.pop_front().unwrap_or(0.0);
            self.delay_buffer.push_back(sample + delayed * self.feedback);
            
            let processed = sample + delayed * self.mix;
            output.extend_from_slice(&processed.to_le_bytes());
        }
        
        Ok(())
    }

    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), MediaError> {
        match name {
            "delay_time" => {
                self.delay_time = value.clamp(0.0, 2.0);
                let new_size = (self.sample_rate as f32 * self.delay_time) as usize;
                self.delay_buffer.resize(new_size, 0.0);
            }
            "feedback" => self.feedback = value.clamp(0.0, 0.95),
            "mix" => self.mix = value.clamp(0.0, 1.0),
            _ => return Err(MediaError::InvalidFormat),
        }
        Ok(())
    }

    fn get_parameter(&self, name: &str) -> Option<f32> {
        match name {
            "delay_time" => Some(self.delay_time),
            "feedback" => Some(self.feedback),
            "mix" => Some(self.mix),
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.delay_buffer.clear();
        let size = (self.sample_rate as f32 * self.delay_time) as usize;
        self.delay_buffer.resize(size, 0.0);
    }
}

pub struct Equalizer {
    bands: Vec<EqBand>,
}

struct EqBand {
    frequency: f32,
    gain: f32,
    q: f32,
    // Biquad filter coefficients
    a0: f32,
    a1: f32,
    a2: f32,
    b1: f32,
    b2: f32,
    // State variables
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Self {
        let frequencies = vec![60.0, 170.0, 350.0, 1000.0, 3500.0, 10000.0];
        let mut bands = Vec::new();
        
        for freq in frequencies {
            bands.push(EqBand::new(freq, 0.0, 1.0, sample_rate));
        }
        
        Self { bands }
    }
}

impl EqBand {
    fn new(frequency: f32, gain: f32, q: f32, sample_rate: u32) -> Self {
        let mut band = Self {
            frequency,
            gain,
            q,
            a0: 1.0,
            a1: 0.0,
            a2: 0.0,
            b1: 0.0,
            b2: 0.0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        };
        band.calculate_coefficients(sample_rate);
        band
    }

    fn calculate_coefficients(&mut self, sample_rate: u32) {
        let omega = 2.0 * core::f32::consts::PI * self.frequency / sample_rate as f32;
        let sin = omega.sin();
        let cos = omega.cos();
        let a = 10.0_f32.powf(self.gain / 40.0);
        let alpha = sin / (2.0 * self.q);
        
        // Peaking EQ
        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * cos;
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * cos;
        let a2 = 1.0 - alpha / a;
        
        self.a0 = b0 / a0;
        self.a1 = b1 / a0;
        self.a2 = b2 / a0;
        self.b1 = a1 / a0;
        self.b2 = a2 / a0;
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.a0 * input + self.a1 * self.x1 + self.a2 * self.x2
            - self.b1 * self.y1 - self.b2 * self.y2;
        
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        
        output
    }
}

impl Effect for Equalizer {
    fn name(&self) -> &str {
        "Equalizer"
    }

    fn effect_type(&self) -> EffectType {
        EffectType::AudioFilter
    }

    fn process(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), MediaError> {
        let samples = input.len() / 4;
        output.reserve(input.len());
        
        for i in 0..samples {
            let mut sample = f32::from_le_bytes([
                input[i * 4],
                input[i * 4 + 1],
                input[i * 4 + 2],
                input[i * 4 + 3],
            ]);
            
            for band in &mut self.bands {
                sample = band.process(sample);
            }
            
            output.extend_from_slice(&sample.to_le_bytes());
        }
        
        Ok(())
    }

    fn set_parameter(&mut self, name: &str, value: f32) -> Result<(), MediaError> {
        if let Some(band_num) = name.strip_prefix("band_").and_then(|s| s.parse::<usize>().ok()) {
            if band_num < self.bands.len() {
                self.bands[band_num].gain = value.clamp(-12.0, 12.0);
                self.bands[band_num].calculate_coefficients(48000); // Default sample rate
                return Ok(());
            }
        }
        Err(MediaError::InvalidFormat)
    }

    fn get_parameter(&self, name: &str) -> Option<f32> {
        if let Some(band_num) = name.strip_prefix("band_").and_then(|s| s.parse::<usize>().ok()) {
            if band_num < self.bands.len() {
                return Some(self.bands[band_num].gain);
            }
        }
        None
    }

    fn reset(&mut self) {
        for band in &mut self.bands {
            band.x1 = 0.0;
            band.x2 = 0.0;
            band.y1 = 0.0;
            band.y2 = 0.0;
        }
    }
}