// Audio Mixer Implementation
use super::{AudioFormat, SampleFormat};
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::boxed::Box;
use crate::serial_println;

// Mixer Channel
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MixerChannel {
    Master,
    Pcm,
    LineIn,
    Mic,
    Cd,
    Aux,
    Wave,
    Midi,
}

// Mixer Control
#[derive(Debug, Clone)]
pub struct MixerControl {
    pub channel: MixerChannel,
    pub volume_left: f32,
    pub volume_right: f32,
    pub muted: bool,
    pub recording: bool,
}

impl MixerControl {
    pub fn new(channel: MixerChannel) -> Self {
        Self {
            channel,
            volume_left: 1.0,
            volume_right: 1.0,
            muted: false,
            recording: false,
        }
    }
    
    pub fn set_volume(&mut self, left: f32, right: f32) {
        self.volume_left = left.clamp(0.0, 1.0);
        self.volume_right = right.clamp(0.0, 1.0);
    }
    
    pub fn set_mute(&mut self, muted: bool) {
        self.muted = muted;
    }
    
    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
    }
}

// Audio Mixer
pub struct AudioMixer {
    controls: Vec<MixerControl>,
    sample_rate: u32,
    channels: u8,
}

impl AudioMixer {
    pub fn new(sample_rate: u32, channels: u8) -> Self {
        let mut controls = Vec::new();
        
        // Initialize default controls
        controls.push(MixerControl::new(MixerChannel::Master));
        controls.push(MixerControl::new(MixerChannel::Pcm));
        controls.push(MixerControl::new(MixerChannel::LineIn));
        controls.push(MixerControl::new(MixerChannel::Mic));
        controls.push(MixerControl::new(MixerChannel::Cd));
        
        Self {
            controls,
            sample_rate,
            channels,
        }
    }
    
    pub fn get_control(&self, channel: MixerChannel) -> Option<&MixerControl> {
        self.controls.iter().find(|c| c.channel == channel)
    }
    
    pub fn get_control_mut(&mut self, channel: MixerChannel) -> Option<&mut MixerControl> {
        self.controls.iter_mut().find(|c| c.channel == channel)
    }
    
    pub fn set_volume(&mut self, channel: MixerChannel, left: f32, right: f32) {
        if let Some(control) = self.get_control_mut(channel) {
            control.set_volume(left, right);
            serial_println!("Mixer: Set {:?} volume to L:{:.2} R:{:.2}", channel, left, right);
        }
    }
    
    pub fn set_master_volume(&mut self, volume: f32) {
        self.set_volume(MixerChannel::Master, volume, volume);
    }
    
    pub fn mix_samples(&self, inputs: &[(&[f32], MixerChannel)], output: &mut [f32]) {
        // Clear output buffer
        output.fill(0.0);
        
        // Mix all inputs
        for (input, channel) in inputs {
            if let Some(control) = self.get_control(*channel) {
                if control.muted {
                    continue;
                }
                
                let master = self.get_control(MixerChannel::Master)
                    .map(|c| (c.volume_left, c.volume_right))
                    .unwrap_or((1.0, 1.0));
                
                // Apply channel and master volumes
                for (i, sample) in input.iter().enumerate() {
                    if i >= output.len() {
                        break;
                    }
                    
                    let vol = if i % 2 == 0 {
                        control.volume_left * master.0
                    } else {
                        control.volume_right * master.1
                    };
                    
                    output[i] += sample * vol;
                }
            }
        }
        
        // Clip output to prevent distortion
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
    
    pub fn convert_format(&self, input: &[u8], input_fmt: SampleFormat, output_fmt: SampleFormat) -> Vec<u8> {
        let samples = self.decode_samples(input, input_fmt);
        self.encode_samples(&samples, output_fmt)
    }
    
    fn decode_samples(&self, input: &[u8], format: SampleFormat) -> Vec<f32> {
        let mut samples = Vec::new();
        
        match format {
            SampleFormat::U8 => {
                for &byte in input {
                    let sample = (byte as f32 - 128.0) / 128.0;
                    samples.push(sample);
                }
            }
            SampleFormat::S16LE => {
                for chunk in input.chunks_exact(2) {
                    let value = i16::from_le_bytes([chunk[0], chunk[1]]);
                    let sample = value as f32 / 32768.0;
                    samples.push(sample);
                }
            }
            SampleFormat::S24LE => {
                for chunk in input.chunks_exact(3) {
                    let value = (chunk[0] as i32) | 
                               ((chunk[1] as i32) << 8) | 
                               ((chunk[2] as i32) << 16);
                    let value = if value & 0x800000 != 0 {
                        value | 0xFF000000u32 as i32  // Sign extend
                    } else {
                        value
                    };
                    let sample = value as f32 / 8388608.0;
                    samples.push(sample);
                }
            }
            SampleFormat::S32LE => {
                for chunk in input.chunks_exact(4) {
                    let value = i32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    let sample = value as f32 / 2147483648.0;
                    samples.push(sample);
                }
            }
            SampleFormat::F32LE => {
                for chunk in input.chunks_exact(4) {
                    let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(value);
                }
            }
        }
        
        samples
    }
    
    fn encode_samples(&self, samples: &[f32], format: SampleFormat) -> Vec<u8> {
        let mut output = Vec::new();
        
        match format {
            SampleFormat::U8 => {
                for &sample in samples {
                    let value = ((sample * 128.0) + 128.0) as u8;
                    output.push(value);
                }
            }
            SampleFormat::S16LE => {
                for &sample in samples {
                    let value = (sample * 32767.0) as i16;
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            SampleFormat::S24LE => {
                for &sample in samples {
                    let value = (sample * 8388607.0) as i32;
                    output.push((value & 0xFF) as u8);
                    output.push(((value >> 8) & 0xFF) as u8);
                    output.push(((value >> 16) & 0xFF) as u8);
                }
            }
            SampleFormat::S32LE => {
                for &sample in samples {
                    let value = (sample * 2147483647.0) as i32;
                    output.extend_from_slice(&value.to_le_bytes());
                }
            }
            SampleFormat::F32LE => {
                for &sample in samples {
                    output.extend_from_slice(&sample.to_le_bytes());
                }
            }
        }
        
        output
    }
    
    pub fn resample(&self, input: &[f32], input_rate: u32, output_rate: u32) -> Vec<f32> {
        if input_rate == output_rate {
            return input.to_vec();
        }
        
        let ratio = output_rate as f32 / input_rate as f32;
        let output_len = (input.len() as f32 * ratio) as usize;
        let mut output = vec![0.0; output_len];
        
        // Simple linear interpolation resampling
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

// Effects Processing
pub mod effects {
    use super::*;
    
    pub struct Reverb {
        delay_lines: Vec<Vec<f32>>,
        feedback: f32,
        mix: f32,
    }
    
    impl Reverb {
        pub fn new(sample_rate: u32) -> Self {
            let delays = vec![
                vec![0.0; (sample_rate * 37 / 1000) as usize],  // 37ms
                vec![0.0; (sample_rate * 41 / 1000) as usize],  // 41ms
                vec![0.0; (sample_rate * 43 / 1000) as usize],  // 43ms
                vec![0.0; (sample_rate * 47 / 1000) as usize],  // 47ms
            ];
            
            Self {
                delay_lines: delays,
                feedback: 0.5,
                mix: 0.3,
            }
        }
        
        pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
            let mut output = input.to_vec();
            
            for sample_idx in 0..input.len() {
                let mut reverb = 0.0;
                
                for delay_line in &mut self.delay_lines {
                    let delayed = delay_line[0];
                    reverb += delayed * self.feedback;
                    
                    // Shift delay line
                    let len = delay_line.len();
                    for i in 0..len - 1 {
                        delay_line[i] = delay_line[i + 1];
                    }
                    delay_line[len - 1] = input[sample_idx] + delayed * self.feedback;
                }
                
                output[sample_idx] = input[sample_idx] * (1.0 - self.mix) + reverb * self.mix;
            }
            
            output
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
            let frequencies = vec![60.0, 250.0, 1000.0, 4000.0, 16000.0];
            let mut bands = Vec::new();
            
            for freq in frequencies {
                bands.push(EqBand::new(freq, 0.0, 1.0, sample_rate));
            }
            
            Self { bands }
        }
        
        pub fn set_band_gain(&mut self, band_idx: usize, gain_db: f32) {
            if band_idx < self.bands.len() {
                self.bands[band_idx].set_gain(gain_db);
            }
        }
        
        pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
            let mut output = input.to_vec();
            
            for band in &mut self.bands {
                output = band.process(&output);
            }
            
            output
        }
    }
    
    impl EqBand {
        fn new(frequency: f32, gain_db: f32, q: f32, sample_rate: u32) -> Self {
            let mut band = Self {
                frequency,
                gain: gain_db,
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
        
        fn set_gain(&mut self, gain_db: f32) {
            self.gain = gain_db;
            // Recalculate coefficients with new gain
        }
        
        fn calculate_coefficients(&mut self, sample_rate: u32) {
            let omega = 2.0 * core::f32::consts::PI * self.frequency / sample_rate as f32;
            let sin_omega = sine_approx(omega);
            let cos_omega = cosine_approx(omega);
            let alpha = sin_omega / (2.0 * self.q);
            let a = pow10_approx(self.gain / 40.0);
            
            // Peaking EQ coefficients
            self.b1 = -2.0 * cos_omega;
            self.b2 = 1.0 - alpha;
            self.a0 = 1.0 + alpha * a;
            self.a1 = -2.0 * cos_omega;
            self.a2 = 1.0 - alpha * a;
            
            // Normalize
            let norm = 1.0 / (1.0 + alpha);
            self.a0 *= norm;
            self.a1 *= norm;
            self.a2 *= norm;
            self.b1 *= norm;
            self.b2 *= norm;
        }
        
        fn process(&mut self, input: &[f32]) -> Vec<f32> {
            let mut output = vec![0.0; input.len()];
            
            for (i, &x) in input.iter().enumerate() {
                let y = self.a0 * x + self.a1 * self.x1 + self.a2 * self.x2
                      - self.b1 * self.y1 - self.b2 * self.y2;
                
                output[i] = y;
                
                // Update state
                self.x2 = self.x1;
                self.x1 = x;
                self.y2 = self.y1;
                self.y1 = y;
            }
            
            output
        }
    }
}

// Math approximations for no_std environment
fn sine_approx(x: f32) -> f32 {
    // Normalize angle to [-pi, pi]
    let pi = core::f32::consts::PI;
    let mut angle = x % (2.0 * pi);
    if angle > pi {
        angle -= 2.0 * pi;
    } else if angle < -pi {
        angle += 2.0 * pi;
    }
    
    // Taylor series approximation
    let x2 = angle * angle;
    let x3 = x2 * angle;
    let x5 = x3 * x2;
    let x7 = x5 * x2;
    
    angle - x3 / 6.0 + x5 / 120.0 - x7 / 5040.0
}

fn cosine_approx(x: f32) -> f32 {
    sine_approx(x + core::f32::consts::FRAC_PI_2)
}

fn pow10_approx(x: f32) -> f32 {
    // 10^x = e^(x*ln(10))
    let ln10 = 2.302585092994045684017991454684;
    let x_ln10 = x * ln10;
    
    // exp approximation
    let mut sum = 1.0;
    let mut term = 1.0;
    for i in 1..10 {
        term *= x_ln10 / i as f32;
        sum += term;
    }
    sum
}