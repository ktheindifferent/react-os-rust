// AC'97 (Audio Codec '97) Driver Implementation
use super::{AudioDriver, AudioStream, AudioCaps, AudioFormat, StreamDirection, StreamState, SampleFormat};
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::boxed::Box;
use x86_64::instructions::port::Port;
use crate::{println, serial_println};
use crate::memory::PHYS_MEM_OFFSET;

// AC'97 I/O Port Offsets
const AC97_RESET: u16 = 0x00;
const AC97_MASTER_VOLUME: u16 = 0x02;
const AC97_AUX_VOLUME: u16 = 0x04;
const AC97_MONO_VOLUME: u16 = 0x06;
const AC97_MASTER_TONE: u16 = 0x08;
const AC97_PC_BEEP_VOLUME: u16 = 0x0A;
const AC97_PHONE_VOLUME: u16 = 0x0C;
const AC97_MIC_VOLUME: u16 = 0x0E;
const AC97_LINE_IN_VOLUME: u16 = 0x10;
const AC97_CD_VOLUME: u16 = 0x12;
const AC97_VIDEO_VOLUME: u16 = 0x14;
const AC97_AUX_IN_VOLUME: u16 = 0x16;
const AC97_PCM_OUT_VOLUME: u16 = 0x18;
const AC97_RECORD_SELECT: u16 = 0x1A;
const AC97_RECORD_GAIN: u16 = 0x1C;
const AC97_RECORD_GAIN_MIC: u16 = 0x1E;
const AC97_GENERAL_PURPOSE: u16 = 0x20;
const AC97_3D_CONTROL: u16 = 0x22;
const AC97_POWERDOWN_CTRL: u16 = 0x26;
const AC97_EXTENDED_AUDIO: u16 = 0x28;
const AC97_EXTENDED_AUDIO_CTRL: u16 = 0x2A;
const AC97_PCM_FRONT_DAC_RATE: u16 = 0x2C;
const AC97_PCM_SURR_DAC_RATE: u16 = 0x2E;
const AC97_PCM_LFE_DAC_RATE: u16 = 0x30;
const AC97_PCM_LR_ADC_RATE: u16 = 0x32;
const AC97_VENDOR_ID1: u16 = 0x7C;
const AC97_VENDOR_ID2: u16 = 0x7E;

// Bus Master Registers
const BM_PCM_OUT_REG: u16 = 0x10;
const BM_PCM_IN_REG: u16 = 0x00;
const BM_MIC_IN_REG: u16 = 0x08;
const BM_CONTROL: u16 = 0x2C;
const BM_STATUS: u16 = 0x30;

// Buffer Descriptor List Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct BufferDescriptor {
    addr: u32,      // Physical address of buffer
    samples: u16,   // Number of samples (0 = 65536)
    flags: u16,     // Control flags
}

// Flags for Buffer Descriptor
const BD_IOC: u16 = 1 << 15;  // Interrupt on completion
const BD_BUP: u16 = 1 << 14;  // Buffer underrun policy

// Control Register Bits
const CTRL_RUN: u8 = 1 << 0;   // Run/pause
const CTRL_RESET: u8 = 1 << 1; // Reset
const CTRL_FEIE: u8 = 1 << 3;  // FIFO error interrupt enable
const CTRL_IOCE: u8 = 1 << 4;  // Interrupt on completion enable

// Status Register Bits  
const STATUS_DCH: u16 = 1 << 0;   // DMA controller halted
const STATUS_CELV: u16 = 1 << 1;  // Current equals last valid
const STATUS_LVBCI: u16 = 1 << 2; // Last valid buffer completion interrupt
const STATUS_BCIS: u16 = 1 << 3;  // Buffer completion interrupt status
const STATUS_FIFOE: u16 = 1 << 4; // FIFO error

pub struct Ac97Controller {
    mixer_base: u16,
    bus_master_base: u16,
    vendor_id: u32,
    capabilities: AudioCaps,
    buffer_descriptors: Vec<BufferDescriptor>,
    audio_buffers: Vec<Vec<u8>>,
}

impl Ac97Controller {
    pub fn new(mixer_base: u16, bus_master_base: u16) -> Self {
        Self {
            mixer_base,
            bus_master_base,
            vendor_id: 0,
            capabilities: AudioCaps {
                name: String::from("AC'97 Audio Codec"),
                vendor_id: 0,
                device_id: 0,
                sample_rates: vec![8000, 11025, 16000, 22050, 44100, 48000],
                formats: vec![SampleFormat::S16LE],
                min_channels: 1,
                max_channels: 2,
                has_input: true,
                has_output: true,
            },
            buffer_descriptors: Vec::new(),
            audio_buffers: Vec::new(),
        }
    }
    
    fn read_mixer(&self, reg: u16) -> u16 {
        unsafe {
            let mut port = Port::<u16>::new(self.mixer_base + reg);
            port.read()
        }
    }
    
    fn write_mixer(&self, reg: u16, value: u16) {
        unsafe {
            let mut port = Port::<u16>::new(self.mixer_base + reg);
            port.write(value);
        }
    }
    
    fn read_bus_master8(&self, reg: u16) -> u8 {
        unsafe {
            let mut port = Port::<u8>::new(self.bus_master_base + reg);
            port.read()
        }
    }
    
    fn write_bus_master8(&self, reg: u16, value: u8) {
        unsafe {
            let mut port = Port::<u8>::new(self.bus_master_base + reg);
            port.write(value);
        }
    }
    
    fn read_bus_master16(&self, reg: u16) -> u16 {
        unsafe {
            let mut port = Port::<u16>::new(self.bus_master_base + reg);
            port.read()
        }
    }
    
    fn write_bus_master16(&self, reg: u16, value: u16) {
        unsafe {
            let mut port = Port::<u16>::new(self.bus_master_base + reg);
            port.write(value);
        }
    }
    
    fn read_bus_master32(&self, reg: u16) -> u32 {
        unsafe {
            let mut port = Port::<u32>::new(self.bus_master_base + reg);
            port.read()
        }
    }
    
    fn write_bus_master32(&self, reg: u16, value: u32) {
        unsafe {
            let mut port = Port::<u32>::new(self.bus_master_base + reg);
            port.write(value);
        }
    }
    
    fn reset_codec(&self) -> Result<(), &'static str> {
        // Cold reset
        self.write_mixer(AC97_RESET, 0x0000);
        
        // Wait for codec ready
        for _ in 0..1000 {
            let powerdown = self.read_mixer(AC97_POWERDOWN_CTRL);
            if powerdown & 0x0F == 0x0F {
                // Codec ready
                return Ok(());
            }
            for _ in 0..10000 {
                core::hint::spin_loop();
            }
        }
        
        Err("AC'97 codec reset timeout")
    }
}

impl AudioDriver for Ac97Controller {
    fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("AC97: Initializing controller");
        
        // Reset codec
        self.reset_codec()?;
        
        // Read vendor ID
        let id1 = self.read_mixer(AC97_VENDOR_ID1);
        let id2 = self.read_mixer(AC97_VENDOR_ID2);
        self.vendor_id = ((id1 as u32) << 16) | (id2 as u32);
        self.capabilities.vendor_id = id1;
        self.capabilities.device_id = id2;
        
        serial_println!("AC97: Vendor ID: 0x{:08x}", self.vendor_id);
        
        // Set default volumes
        self.write_mixer(AC97_MASTER_VOLUME, 0x0000);     // Max volume
        self.write_mixer(AC97_PCM_OUT_VOLUME, 0x0808);    // PCM output volume
        self.write_mixer(AC97_PC_BEEP_VOLUME, 0x0000);    // Mute PC beep
        
        // Enable variable rate audio
        let ext_audio = self.read_mixer(AC97_EXTENDED_AUDIO);
        self.write_mixer(AC97_EXTENDED_AUDIO, ext_audio | 0x0001);
        
        // Set default sample rate (48000 Hz)
        self.write_mixer(AC97_PCM_FRONT_DAC_RATE, 48000);
        self.write_mixer(AC97_PCM_LR_ADC_RATE, 48000);
        
        // Allocate buffer descriptors (32 entries)
        self.buffer_descriptors = vec![BufferDescriptor {
            addr: 0,
            samples: 0,
            flags: 0,
        }; 32];
        
        // Allocate audio buffers (32 x 64KB)
        for _ in 0..32 {
            self.audio_buffers.push(vec![0u8; 65536]);
        }
        
        // Set up buffer descriptor list base addresses
        let bd_list_addr = self.buffer_descriptors.as_ptr() as u32;
        self.write_bus_master32(BM_PCM_OUT_REG + 0, bd_list_addr);
        self.write_bus_master32(BM_PCM_IN_REG + 0, bd_list_addr);
        
        serial_println!("AC97: Initialization complete");
        Ok(())
    }
    
    fn get_capabilities(&self) -> AudioCaps {
        self.capabilities.clone()
    }
    
    fn open_stream(&mut self, direction: StreamDirection, format: AudioFormat) -> Result<Box<dyn AudioStream>, &'static str> {
        let stream = Ac97Stream::new(
            self.mixer_base,
            self.bus_master_base,
            direction,
            format,
        );
        
        Ok(Box::new(stream))
    }
    
    fn close_stream(&mut self, _stream: Box<dyn AudioStream>) -> Result<(), &'static str> {
        // Stop DMA
        self.write_bus_master8(BM_PCM_OUT_REG + 0x0B, 0);
        self.write_bus_master8(BM_PCM_IN_REG + 0x0B, 0);
        Ok(())
    }
    
    fn set_master_volume(&mut self, volume: f32) -> Result<(), &'static str> {
        // Convert 0.0-1.0 to AC'97 volume (0x00 = max, 0x3F = mute)
        let ac97_vol = ((1.0 - volume) * 63.0) as u16;
        let vol_reg = (ac97_vol << 8) | ac97_vol; // Same for both channels
        self.write_mixer(AC97_MASTER_VOLUME, vol_reg);
        Ok(())
    }
    
    fn get_master_volume(&self) -> f32 {
        let vol_reg = self.read_mixer(AC97_MASTER_VOLUME);
        let ac97_vol = (vol_reg & 0x3F) as f32;
        1.0 - (ac97_vol / 63.0)
    }
}

pub struct Ac97Stream {
    mixer_base: u16,
    bus_master_base: u16,
    direction: StreamDirection,
    format: AudioFormat,
    state: StreamState,
    position: u64,
    volume: f32,
}

impl Ac97Stream {
    pub fn new(mixer_base: u16, bus_master_base: u16, direction: StreamDirection, format: AudioFormat) -> Self {
        Self {
            mixer_base,
            bus_master_base,
            direction,
            format,
            state: StreamState::Stopped,
            position: 0,
            volume: 1.0,
        }
    }
    
    fn get_channel_reg(&self) -> u16 {
        match self.direction {
            StreamDirection::Playback => BM_PCM_OUT_REG,
            StreamDirection::Capture => BM_PCM_IN_REG,
        }
    }
}

impl AudioStream for Ac97Stream {
    fn start(&mut self) -> Result<(), &'static str> {
        let reg = self.get_channel_reg();
        
        // Set sample rate
        unsafe {
            let mut port = Port::<u16>::new(self.mixer_base + AC97_PCM_FRONT_DAC_RATE);
            port.write(self.format.sample_rate as u16);
        }
        
        // Start DMA
        unsafe {
            let mut port = Port::<u8>::new(self.bus_master_base + reg + 0x0B);
            port.write(CTRL_RUN | CTRL_IOCE);
        }
        
        self.state = StreamState::Playing;
        Ok(())
    }
    
    fn stop(&mut self) -> Result<(), &'static str> {
        let reg = self.get_channel_reg();
        
        // Stop DMA
        unsafe {
            let mut port = Port::<u8>::new(self.bus_master_base + reg + 0x0B);
            port.write(0);
        }
        
        self.state = StreamState::Stopped;
        Ok(())
    }
    
    fn pause(&mut self) -> Result<(), &'static str> {
        let reg = self.get_channel_reg();
        
        // Clear RUN bit
        unsafe {
            let mut port = Port::<u8>::new(self.bus_master_base + reg + 0x0B);
            let ctrl = port.read();
            port.write(ctrl & !CTRL_RUN);
        }
        
        self.state = StreamState::Paused;
        Ok(())
    }
    
    fn resume(&mut self) -> Result<(), &'static str> {
        let reg = self.get_channel_reg();
        
        // Set RUN bit
        unsafe {
            let mut port = Port::<u8>::new(self.bus_master_base + reg + 0x0B);
            let ctrl = port.read();
            port.write(ctrl | CTRL_RUN);
        }
        
        self.state = StreamState::Playing;
        Ok(())
    }
    
    fn write(&mut self, buffer: &[u8]) -> Result<usize, &'static str> {
        if self.direction != StreamDirection::Playback {
            return Err("Not a playback stream");
        }
        
        // Write to DMA buffer
        // This is simplified - actual implementation would manage ring buffer
        Ok(buffer.len())
    }
    
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize, &'static str> {
        if self.direction != StreamDirection::Capture {
            return Err("Not a capture stream");
        }
        
        // Read from DMA buffer
        // This is simplified - actual implementation would manage ring buffer
        Ok(buffer.len())
    }
    
    fn drain(&mut self) -> Result<(), &'static str> {
        self.state = StreamState::Draining;
        // Wait for buffers to empty
        Ok(())
    }
    
    fn get_position(&self) -> u64 {
        self.position
    }
    
    fn get_state(&self) -> StreamState {
        self.state
    }
    
    fn set_volume(&mut self, volume: f32) -> Result<(), &'static str> {
        self.volume = volume.clamp(0.0, 1.0);
        
        // Set PCM output volume
        let ac97_vol = ((1.0 - self.volume) * 31.0) as u16;
        let vol_reg = (ac97_vol << 8) | ac97_vol;
        
        unsafe {
            let mut port = Port::<u16>::new(self.mixer_base + AC97_PCM_OUT_VOLUME);
            port.write(vol_reg);
        }
        
        Ok(())
    }
}

pub fn detect_ac97() -> Option<Ac97Controller> {
    // Check for AC'97 controller on PCI bus
    // Common I/O port ranges for AC'97
    let common_mixer_bases = [0x200, 0x220, 0x240];
    let common_bm_bases = [0x300, 0x320, 0x340];
    
    for (&mixer, &bm) in common_mixer_bases.iter().zip(common_bm_bases.iter()) {
        // Try to read vendor ID
        unsafe {
            let mut port = Port::<u16>::new(mixer + AC97_VENDOR_ID1);
            let id = port.read();
            
            if id != 0x0000 && id != 0xFFFF {
                serial_println!("AC97: Found controller at mixer=0x{:x}, bm=0x{:x}", mixer, bm);
                return Some(Ac97Controller::new(mixer, bm));
            }
        }
    }
    
    None
}