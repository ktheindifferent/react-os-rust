// Sound Card Driver Implementation
pub mod ac97;
pub mod hda;
pub mod mixer;
pub mod pcm;
pub mod codec;

use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::boxed::Box;
use alloc::collections::VecDeque;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::{println, serial_println};

// Audio Sample Formats
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SampleFormat {
    U8,          // 8-bit unsigned
    S16LE,       // 16-bit signed little-endian
    S24LE,       // 24-bit signed little-endian
    S32LE,       // 32-bit signed little-endian
    F32LE,       // 32-bit float little-endian
}

impl SampleFormat {
    pub fn bytes_per_sample(&self) -> usize {
        match self {
            SampleFormat::U8 => 1,
            SampleFormat::S16LE => 2,
            SampleFormat::S24LE => 3,
            SampleFormat::S32LE => 4,
            SampleFormat::F32LE => 4,
        }
    }
}

// Audio Stream Parameters
#[derive(Debug, Clone)]
pub struct AudioFormat {
    pub sample_rate: u32,      // Hz (e.g., 44100, 48000)
    pub channels: u8,          // Number of channels (1=mono, 2=stereo)
    pub format: SampleFormat,  // Sample format
    pub buffer_size: usize,    // Buffer size in frames
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            format: SampleFormat::S16LE,
            buffer_size: 512,
        }
    }
}

// Audio Device Capabilities
#[derive(Debug, Clone)]
pub struct AudioCaps {
    pub name: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub sample_rates: Vec<u32>,
    pub formats: Vec<SampleFormat>,
    pub min_channels: u8,
    pub max_channels: u8,
    pub has_input: bool,
    pub has_output: bool,
}

// Audio Stream Direction
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamDirection {
    Playback,
    Capture,
}

// Audio Stream State
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StreamState {
    Stopped,
    Playing,
    Paused,
    Draining,
}

// Audio Buffer
pub struct AudioBuffer {
    pub data: Vec<u8>,
    pub frames: usize,
    pub format: AudioFormat,
}

impl AudioBuffer {
    pub fn new(format: AudioFormat) -> Self {
        let size = format.buffer_size * format.channels as usize * format.format.bytes_per_sample();
        Self {
            data: vec![0u8; size],
            frames: format.buffer_size,
            format,
        }
    }
    
    pub fn clear(&mut self) {
        self.data.fill(0);
    }
    
    pub fn bytes_per_frame(&self) -> usize {
        self.format.channels as usize * self.format.format.bytes_per_sample()
    }
}

// Audio Stream Trait
pub trait AudioStream: Send + Sync {
    fn start(&mut self) -> Result<(), &'static str>;
    fn stop(&mut self) -> Result<(), &'static str>;
    fn pause(&mut self) -> Result<(), &'static str>;
    fn resume(&mut self) -> Result<(), &'static str>;
    fn write(&mut self, buffer: &[u8]) -> Result<usize, &'static str>;
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, &'static str>;
    fn drain(&mut self) -> Result<(), &'static str>;
    fn get_position(&self) -> u64;
    fn get_state(&self) -> StreamState;
    fn set_volume(&mut self, volume: f32) -> Result<(), &'static str>;
}

// Audio Driver Trait
pub trait AudioDriver: Send + Sync {
    fn init(&mut self) -> Result<(), &'static str>;
    fn get_capabilities(&self) -> AudioCaps;
    fn open_stream(&mut self, direction: StreamDirection, format: AudioFormat) -> Result<Box<dyn AudioStream>, &'static str>;
    fn close_stream(&mut self, stream: Box<dyn AudioStream>) -> Result<(), &'static str>;
    fn set_master_volume(&mut self, volume: f32) -> Result<(), &'static str>;
    fn get_master_volume(&self) -> f32;
}

// Audio Manager
pub struct AudioManager {
    drivers: Vec<Box<dyn AudioDriver>>,
    active_driver: Option<usize>,
    playback_stream: Option<Box<dyn AudioStream>>,
    capture_stream: Option<Box<dyn AudioStream>>,
    master_volume: f32,
    playback_queue: VecDeque<AudioBuffer>,
    playback_thread_running: AtomicBool,
}

impl AudioManager {
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
            active_driver: None,
            playback_stream: None,
            capture_stream: None,
            master_volume: 1.0,
            playback_queue: VecDeque::new(),
            playback_thread_running: AtomicBool::new(false),
        }
    }
    
    pub fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("Sound: Initializing audio subsystem");
        
        // Try to detect and initialize AC'97
        if let Some(ac97) = ac97::detect_ac97() {
            serial_println!("Sound: Found AC'97 controller");
            let mut driver = Box::new(ac97);
            driver.init()?;
            self.drivers.push(driver);
        }
        
        // Try to detect and initialize HD Audio
        if let Some(hda) = hda::detect_hda() {
            serial_println!("Sound: Found HD Audio controller");
            let mut driver = Box::new(hda);
            driver.init()?;
            self.drivers.push(driver);
        }
        
        if self.drivers.is_empty() {
            return Err("No sound cards detected");
        }
        
        // Select first driver as active
        self.active_driver = Some(0);
        
        serial_println!("Sound: {} audio device(s) initialized", self.drivers.len());
        Ok(())
    }
    
    pub fn play_buffer(&mut self, buffer: AudioBuffer) -> Result<(), &'static str> {
        // Add buffer to playback queue
        self.playback_queue.push_back(buffer);
        
        // Start playback if not already running
        if !self.playback_thread_running.load(Ordering::Relaxed) {
            self.start_playback()?;
        }
        
        Ok(())
    }
    
    fn start_playback(&mut self) -> Result<(), &'static str> {
        if self.playback_stream.is_none() {
            // Open playback stream
            let driver_idx = self.active_driver.ok_or("No active driver")?;
            let format = AudioFormat::default();
            self.playback_stream = Some(self.drivers[driver_idx].open_stream(StreamDirection::Playback, format)?);
        }
        
        if let Some(ref mut stream) = self.playback_stream {
            stream.start()?;
            self.playback_thread_running.store(true, Ordering::Relaxed);
        }
        
        Ok(())
    }
    
    pub fn stop_playback(&mut self) -> Result<(), &'static str> {
        if let Some(ref mut stream) = self.playback_stream {
            stream.stop()?;
            self.playback_thread_running.store(false, Ordering::Relaxed);
        }
        Ok(())
    }
    
    pub fn set_volume(&mut self, volume: f32) -> Result<(), &'static str> {
        let volume = volume.clamp(0.0, 1.0);
        self.master_volume = volume;
        
        if let Some(driver_idx) = self.active_driver {
            self.drivers[driver_idx].set_master_volume(volume)?;
        }
        
        Ok(())
    }
    
    pub fn get_volume(&self) -> f32 {
        self.master_volume
    }
    
    pub fn list_devices(&self) -> Vec<AudioCaps> {
        self.drivers.iter().map(|d| d.get_capabilities()).collect()
    }
    
    pub fn play_tone(&mut self, frequency: f32, duration_ms: u32) -> Result<(), &'static str> {
        // Generate a simple sine wave tone
        let format = AudioFormat::default();
        let sample_rate = format.sample_rate as f32;
        let samples = (sample_rate * duration_ms as f32 / 1000.0) as usize;
        
        let mut buffer = AudioBuffer::new(format);
        let samples_per_buffer = buffer.frames;
        
        for i in 0..samples_per_buffer {
            let t = i as f32 / sample_rate;
            let angle = t * frequency * 2.0 * core::f32::consts::PI;
            // Simple sine approximation for no_std
            let sample = sine_approx(angle);
            
            // Convert to S16LE
            let sample_i16 = (sample * 32767.0) as i16;
            let bytes = sample_i16.to_le_bytes();
            
            // Write to both channels (stereo)
            let offset = i * 4; // 2 channels * 2 bytes
            if offset + 3 < buffer.data.len() {
                buffer.data[offset] = bytes[0];
                buffer.data[offset + 1] = bytes[1];
                buffer.data[offset + 2] = bytes[0];
                buffer.data[offset + 3] = bytes[1];
            }
        }
        
        self.play_buffer(buffer)?;
        Ok(())
    }
}

// Wave File Support
pub mod wave {
    use super::*;
    
    #[repr(C, packed)]
    pub struct WaveHeader {
        pub riff: [u8; 4],        // "RIFF"
        pub file_size: u32,       // File size - 8
        pub wave: [u8; 4],        // "WAVE"
        pub fmt: [u8; 4],         // "fmt "
        pub fmt_size: u32,        // Format chunk size
        pub audio_format: u16,    // 1 = PCM
        pub channels: u16,        // Number of channels
        pub sample_rate: u32,     // Sample rate
        pub byte_rate: u32,       // Byte rate
        pub block_align: u16,     // Block align
        pub bits_per_sample: u16, // Bits per sample
        pub data: [u8; 4],        // "data"
        pub data_size: u32,       // Data size
    }
    
    impl WaveHeader {
        pub fn new(format: &AudioFormat, data_size: u32) -> Self {
            let bits_per_sample = format.format.bytes_per_sample() as u16 * 8;
            let block_align = format.channels as u16 * bits_per_sample / 8;
            let byte_rate = format.sample_rate * block_align as u32;
            
            Self {
                riff: *b"RIFF",
                file_size: data_size + 36, // 44 - 8
                wave: *b"WAVE",
                fmt: *b"fmt ",
                fmt_size: 16,
                audio_format: 1, // PCM
                channels: format.channels as u16,
                sample_rate: format.sample_rate,
                byte_rate,
                block_align,
                bits_per_sample,
                data: *b"data",
                data_size,
            }
        }
    }
    
    pub fn parse_wave(data: &[u8]) -> Result<(AudioFormat, &[u8]), &'static str> {
        if data.len() < core::mem::size_of::<WaveHeader>() {
            return Err("Wave file too small");
        }
        
        let header = unsafe {
            &*(data.as_ptr() as *const WaveHeader)
        };
        
        if &header.riff != b"RIFF" || &header.wave != b"WAVE" {
            return Err("Invalid WAVE file");
        }
        
        if header.audio_format != 1 {
            return Err("Only PCM format supported");
        }
        
        let format = match header.bits_per_sample {
            8 => SampleFormat::U8,
            16 => SampleFormat::S16LE,
            24 => SampleFormat::S24LE,
            32 => SampleFormat::S32LE,
            _ => return Err("Unsupported bit depth"),
        };
        
        let audio_format = AudioFormat {
            sample_rate: header.sample_rate,
            channels: header.channels as u8,
            format,
            buffer_size: 512,
        };
        
        let data_offset = core::mem::size_of::<WaveHeader>();
        let audio_data = &data[data_offset..data_offset + header.data_size as usize];
        
        Ok((audio_format, audio_data))
    }
}

// MIDI Support
pub mod midi {
    use super::*;
    
    #[derive(Debug, Clone, Copy)]
    pub struct MidiNote {
        pub channel: u8,
        pub note: u8,
        pub velocity: u8,
    }
    
    #[derive(Debug, Clone, Copy)]
    pub enum MidiMessage {
        NoteOff(MidiNote),
        NoteOn(MidiNote),
        PolyPressure { channel: u8, note: u8, pressure: u8 },
        ControlChange { channel: u8, controller: u8, value: u8 },
        ProgramChange { channel: u8, program: u8 },
        ChannelPressure { channel: u8, pressure: u8 },
        PitchBend { channel: u8, value: u16 },
        SystemExclusive(u8),
        TimeCode(u8),
        SongPosition(u16),
        SongSelect(u8),
        TuneRequest,
        Clock,
        Start,
        Continue,
        Stop,
        ActiveSensing,
        Reset,
    }
    
    pub fn parse_midi_message(data: &[u8]) -> Option<MidiMessage> {
        if data.is_empty() {
            return None;
        }
        
        let status = data[0];
        let msg_type = status & 0xF0;
        let channel = status & 0x0F;
        
        match msg_type {
            0x80 => {
                // Note Off
                if data.len() >= 3 {
                    Some(MidiMessage::NoteOff(MidiNote {
                        channel,
                        note: data[1],
                        velocity: data[2],
                    }))
                } else {
                    None
                }
            }
            0x90 => {
                // Note On
                if data.len() >= 3 {
                    Some(MidiMessage::NoteOn(MidiNote {
                        channel,
                        note: data[1],
                        velocity: data[2],
                    }))
                } else {
                    None
                }
            }
            0xB0 => {
                // Control Change
                if data.len() >= 3 {
                    Some(MidiMessage::ControlChange {
                        channel,
                        controller: data[1],
                        value: data[2],
                    })
                } else {
                    None
                }
            }
            0xC0 => {
                // Program Change
                if data.len() >= 2 {
                    Some(MidiMessage::ProgramChange {
                        channel,
                        program: data[1],
                    })
                } else {
                    None
                }
            }
            0xE0 => {
                // Pitch Bend
                if data.len() >= 3 {
                    let value = (data[1] as u16) | ((data[2] as u16) << 7);
                    Some(MidiMessage::PitchBend { channel, value })
                } else {
                    None
                }
            }
            0xF0 => {
                // System messages
                match status {
                    0xF8 => Some(MidiMessage::Clock),
                    0xFA => Some(MidiMessage::Start),
                    0xFB => Some(MidiMessage::Continue),
                    0xFC => Some(MidiMessage::Stop),
                    0xFE => Some(MidiMessage::ActiveSensing),
                    0xFF => Some(MidiMessage::Reset),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    
    pub fn note_to_frequency(note: u8) -> f32 {
        // A4 (note 69) = 440 Hz
        // Using simple approximation for 2^x
        let exponent = (note as f32 - 69.0) / 12.0;
        440.0 * pow2_approx(exponent)
    }
}

// Math approximations for no_std environment
pub fn sine_approx(x: f32) -> f32 {
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

pub fn pow2_approx(x: f32) -> f32 {
    // 2^x approximation using Taylor series
    let ln2 = 0.693147180559945309417232121458;
    let x_ln2 = x * ln2;
    
    // exp(x*ln(2)) approximation
    let mut sum = 1.0;
    let mut term = 1.0;
    for i in 1..10 {
        term *= x_ln2 / i as f32;
        sum += term;
    }
    sum
}

lazy_static! {
    pub static ref AUDIO_MANAGER: Mutex<AudioManager> = Mutex::new(AudioManager::new());
}

pub fn init() {
    AUDIO_MANAGER.lock().init().unwrap_or_else(|e| {
        serial_println!("Sound: Failed to initialize: {}", e);
    });
}

pub fn play_startup_sound() {
    // Play a simple startup chime
    let mut manager = AUDIO_MANAGER.lock();
    
    // Play C-E-G chord (C major)
    manager.play_tone(261.63, 200).ok(); // C4
    manager.play_tone(329.63, 200).ok(); // E4
    manager.play_tone(392.00, 400).ok(); // G4
}