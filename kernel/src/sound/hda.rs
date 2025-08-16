// Intel HD Audio (High Definition Audio) Driver Implementation
use super::{AudioDriver, AudioStream, AudioCaps, AudioFormat, StreamDirection, StreamState, SampleFormat};
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::boxed::Box;
use crate::{println, serial_println};
use crate::memory::PHYS_MEM_OFFSET;
use core::sync::atomic::{AtomicU32, Ordering};

// HDA Memory Mapped Registers
const HDA_REG_GCAP: usize = 0x00;      // Global Capabilities
const HDA_REG_VMIN: usize = 0x02;      // Minor Version
const HDA_REG_VMAJ: usize = 0x03;      // Major Version
const HDA_REG_OUTPAY: usize = 0x04;    // Output Payload Capability
const HDA_REG_INPAY: usize = 0x06;     // Input Payload Capability
const HDA_REG_GCTL: usize = 0x08;      // Global Control
const HDA_REG_WAKEEN: usize = 0x0C;    // Wake Enable
const HDA_REG_STATESTS: usize = 0x0E;  // State Change Status
const HDA_REG_GSTS: usize = 0x10;      // Global Status
const HDA_REG_INTCTL: usize = 0x20;    // Interrupt Control
const HDA_REG_INTSTS: usize = 0x24;    // Interrupt Status
const HDA_REG_COUNTER: usize = 0x30;    // Wall Clock Counter
const HDA_REG_SSYNC: usize = 0x38;     // Stream Synchronization
const HDA_REG_CORBLBASE: usize = 0x40; // CORB Lower Base Address
const HDA_REG_CORBUBASE: usize = 0x44; // CORB Upper Base Address
const HDA_REG_CORBWP: usize = 0x48;    // CORB Write Pointer
const HDA_REG_CORBRP: usize = 0x4A;    // CORB Read Pointer
const HDA_REG_CORBCTL: usize = 0x4C;   // CORB Control
const HDA_REG_CORBSIZE: usize = 0x4E;  // CORB Size
const HDA_REG_RIRBLBASE: usize = 0x50; // RIRB Lower Base Address
const HDA_REG_RIRBUBASE: usize = 0x54; // RIRB Upper Base Address
const HDA_REG_RIRBWP: usize = 0x58;    // RIRB Write Pointer
const HDA_REG_RINTCNT: usize = 0x5A;   // Response Interrupt Count
const HDA_REG_RIRBCTL: usize = 0x5C;   // RIRB Control
const HDA_REG_RIRBSTS: usize = 0x5D;   // RIRB Status
const HDA_REG_RIRBSIZE: usize = 0x5E;  // RIRB Size
const HDA_REG_DPLBASE: usize = 0x70;   // DMA Position Lower Base
const HDA_REG_DPUBASE: usize = 0x74;   // DMA Position Upper Base

// Stream Descriptor Registers (SDn)
const HDA_SD_CTL: usize = 0x00;        // Control
const HDA_SD_STS: usize = 0x03;        // Status
const HDA_SD_LPIB: usize = 0x04;       // Link Position in Buffer
const HDA_SD_CBL: usize = 0x08;        // Cyclic Buffer Length
const HDA_SD_LVI: usize = 0x0C;        // Last Valid Index
const HDA_SD_FIFOW: usize = 0x0E;      // FIFO Watermark
const HDA_SD_FIFOS: usize = 0x10;      // FIFO Size
const HDA_SD_FORMAT: usize = 0x12;     // Format
const HDA_SD_BDLPL: usize = 0x18;      // BDL Pointer Lower
const HDA_SD_BDLPU: usize = 0x1C;      // BDL Pointer Upper

// Global Control Bits
const GCTL_CRST: u32 = 1 << 0;         // Controller Reset
const GCTL_FCNTRL: u32 = 1 << 1;       // Flush Control
const GCTL_UNSOL: u32 = 1 << 8;        // Accept Unsolicited Response Enable

// Stream Control Bits
const SD_CTL_SRST: u8 = 1 << 0;        // Stream Reset
const SD_CTL_RUN: u8 = 1 << 1;         // Run
const SD_CTL_IOCE: u8 = 1 << 2;        // Interrupt on Completion Enable
const SD_CTL_FEIE: u8 = 1 << 3;        // FIFO Error Interrupt Enable
const SD_CTL_DEIE: u8 = 1 << 4;        // Descriptor Error Interrupt Enable

// Stream Status Bits
const SD_STS_BCIS: u8 = 1 << 2;        // Buffer Completion Interrupt Status
const SD_STS_FIFOE: u8 = 1 << 3;       // FIFO Error
const SD_STS_DESE: u8 = 1 << 4;        // Descriptor Error
const SD_STS_FIFORDY: u8 = 1 << 5;     // FIFO Ready

// HDA Verbs (Commands)
const VERB_GET_PARAM: u32 = 0xF00;
const VERB_GET_CONN_SELECT: u32 = 0xF01;
const VERB_SET_CONN_SELECT: u32 = 0x701;
const VERB_GET_CONN_LIST: u32 = 0xF02;
const VERB_GET_STREAM_FORMAT: u32 = 0xA00;
const VERB_SET_STREAM_FORMAT: u32 = 0x200;
const VERB_GET_AMP_GAIN_MUTE: u32 = 0xB00;
const VERB_SET_AMP_GAIN_MUTE: u32 = 0x300;
const VERB_GET_PIN_WIDGET_CTRL: u32 = 0xF07;
const VERB_SET_PIN_WIDGET_CTRL: u32 = 0x707;
const VERB_GET_PIN_SENSE: u32 = 0xF09;
const VERB_SET_POWER_STATE: u32 = 0x705;
const VERB_GET_POWER_STATE: u32 = 0xF05;
const VERB_GET_CHANNEL_STREAM_ID: u32 = 0xF06;
const VERB_SET_CHANNEL_STREAM_ID: u32 = 0x706;
const VERB_GET_CONFIG_DEFAULT: u32 = 0xF1C;
const VERB_GET_SUBSYSTEM_ID: u32 = 0xF20;

// Parameter IDs
const PARAM_VENDOR_ID: u32 = 0x00;
const PARAM_REVISION_ID: u32 = 0x02;
const PARAM_NODE_COUNT: u32 = 0x04;
const PARAM_FUNCTION_TYPE: u32 = 0x05;
const PARAM_AUDIO_FG_CAP: u32 = 0x08;
const PARAM_AUDIO_WIDGET_CAP: u32 = 0x09;
const PARAM_PCM: u32 = 0x0A;
const PARAM_STREAM: u32 = 0x0B;
const PARAM_PIN_CAP: u32 = 0x0C;
const PARAM_INPUT_AMP_CAP: u32 = 0x0D;
const PARAM_OUTPUT_AMP_CAP: u32 = 0x12;
const PARAM_CONNECTION_LIST_LEN: u32 = 0x0E;
const PARAM_POWER_STATE: u32 = 0x0F;
const PARAM_GPIO_CAP: u32 = 0x11;
const PARAM_VOLUME_KNOB_CAP: u32 = 0x13;

// Buffer Descriptor List Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct BdlEntry {
    addr: u64,      // Physical address of buffer
    length: u32,    // Length in bytes
    ioc: u32,       // Interrupt on completion flag
}

// CORB Entry (Command Output Ring Buffer)
type CorbEntry = u32;

// RIRB Entry (Response Input Ring Buffer)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct RirbEntry {
    response: u32,
    resp_ex: u32,   // Response extended (codec address and unsolicited flag)
}

pub struct HdaController {
    base_addr: u64,
    capabilities: AudioCaps,
    corb: Vec<CorbEntry>,
    rirb: Vec<RirbEntry>,
    corb_wp: AtomicU32,
    streams: Vec<HdaStreamDescriptor>,
}

struct HdaStreamDescriptor {
    index: u8,
    offset: usize,
    bdl: Vec<BdlEntry>,
    buffers: Vec<Vec<u8>>,
    format: AudioFormat,
    active: bool,
}

impl HdaController {
    pub fn new(base_addr: u64) -> Self {
        Self {
            base_addr,
            capabilities: AudioCaps {
                name: String::from("Intel HD Audio"),
                vendor_id: 0,
                device_id: 0,
                sample_rates: vec![8000, 11025, 16000, 22050, 32000, 44100, 48000, 88200, 96000, 176400, 192000],
                formats: vec![SampleFormat::U8, SampleFormat::S16LE, SampleFormat::S24LE, SampleFormat::S32LE],
                min_channels: 1,
                max_channels: 8,
                has_input: true,
                has_output: true,
            },
            corb: Vec::new(),
            rirb: Vec::new(),
            corb_wp: AtomicU32::new(0),
            streams: Vec::new(),
        }
    }
    
    unsafe fn read32(&self, reg: usize) -> u32 {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *const u32;
        addr.read_volatile()
    }
    
    unsafe fn write32(&self, reg: usize, value: u32) {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *mut u32;
        addr.write_volatile(value);
    }
    
    unsafe fn read16(&self, reg: usize) -> u16 {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *const u16;
        addr.read_volatile()
    }
    
    unsafe fn write16(&self, reg: usize, value: u16) {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *mut u16;
        addr.write_volatile(value);
    }
    
    unsafe fn read8(&self, reg: usize) -> u8 {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *const u8;
        addr.read_volatile()
    }
    
    unsafe fn write8(&self, reg: usize, value: u8) {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + reg as u64) as *mut u8;
        addr.write_volatile(value);
    }
    
    fn reset_controller(&mut self) -> Result<(), &'static str> {
        unsafe {
            // Enter reset
            self.write32(HDA_REG_GCTL, 0);
            
            // Wait for reset to take effect
            for _ in 0..1000 {
                if self.read32(HDA_REG_GCTL) & GCTL_CRST == 0 {
                    break;
                }
                for _ in 0..1000 {
                    core::hint::spin_loop();
                }
            }
            
            // Exit reset
            self.write32(HDA_REG_GCTL, GCTL_CRST);
            
            // Wait for controller to exit reset
            for _ in 0..1000 {
                if self.read32(HDA_REG_GCTL) & GCTL_CRST != 0 {
                    return Ok(());
                }
                for _ in 0..1000 {
                    core::hint::spin_loop();
                }
            }
        }
        
        Err("HDA controller reset timeout")
    }
    
    fn setup_corb_rirb(&mut self) -> Result<(), &'static str> {
        // Allocate CORB and RIRB
        self.corb = vec![0; 256];
        self.rirb = vec![RirbEntry { response: 0, resp_ex: 0 }; 256];
        
        unsafe {
            // Stop CORB and RIRB
            self.write8(HDA_REG_CORBCTL, 0);
            self.write8(HDA_REG_RIRBCTL, 0);
            
            // Set CORB/RIRB sizes (256 entries)
            self.write8(HDA_REG_CORBSIZE, 0x02);
            self.write8(HDA_REG_RIRBSIZE, 0x02);
            
            // Set CORB/RIRB base addresses
            let corb_addr = self.corb.as_ptr() as u64;
            let rirb_addr = self.rirb.as_ptr() as u64;
            
            self.write32(HDA_REG_CORBLBASE, corb_addr as u32);
            self.write32(HDA_REG_CORBUBASE, (corb_addr >> 32) as u32);
            self.write32(HDA_REG_RIRBLBASE, rirb_addr as u32);
            self.write32(HDA_REG_RIRBUBASE, (rirb_addr >> 32) as u32);
            
            // Reset read/write pointers
            self.write16(HDA_REG_CORBRP, 0x8000); // Reset bit
            while self.read16(HDA_REG_CORBRP) & 0x8000 != 0 {
                core::hint::spin_loop();
            }
            
            self.write16(HDA_REG_RIRBWP, 0x8000); // Reset bit
            
            // Start CORB and RIRB
            self.write8(HDA_REG_CORBCTL, 0x02); // DMA Enable
            self.write8(HDA_REG_RIRBCTL, 0x02); // DMA Enable + Interrupt Enable
        }
        
        Ok(())
    }
    
    fn send_command(&mut self, codec: u8, nid: u8, verb: u32, param: u16) -> Result<u32, &'static str> {
        let cmd = ((codec as u32) << 28) | 
                  ((nid as u32) << 20) | 
                  (verb << 8) | 
                  (param as u32);
        
        // Write to CORB
        let wp = self.corb_wp.fetch_add(1, Ordering::SeqCst) % 256;
        self.corb[wp as usize] = cmd;
        
        unsafe {
            self.write16(HDA_REG_CORBWP, wp as u16);
        }
        
        // Wait for response in RIRB
        for _ in 0..1000 {
            unsafe {
                let rirb_wp = self.read16(HDA_REG_RIRBWP);
                if rirb_wp != 0 {
                    // Read response
                    let entry = self.rirb[0];
                    return Ok(entry.response);
                }
            }
            for _ in 0..1000 {
                core::hint::spin_loop();
            }
        }
        
        Err("HDA command timeout")
    }
    
    fn enumerate_codecs(&mut self) -> Result<(), &'static str> {
        unsafe {
            let statests = self.read16(HDA_REG_STATESTS);
            
            for codec in 0..15 {
                if statests & (1 << codec) != 0 {
                    serial_println!("HDA: Found codec at address {}", codec);
                    
                    // Get vendor ID
                    if let Ok(vendor_id) = self.send_command(codec, 0, VERB_GET_PARAM, PARAM_VENDOR_ID as u16) {
                        self.capabilities.vendor_id = (vendor_id >> 16) as u16;
                        self.capabilities.device_id = (vendor_id & 0xFFFF) as u16;
                        serial_println!("HDA: Codec vendor ID: 0x{:08x}", vendor_id);
                    }
                    
                    // Initialize codec
                    self.init_codec(codec)?;
                }
            }
        }
        
        Ok(())
    }
    
    fn init_codec(&mut self, codec: u8) -> Result<(), &'static str> {
        // Get function group
        let node_count = self.send_command(codec, 0, VERB_GET_PARAM, PARAM_NODE_COUNT as u16)?;
        let start_nid = (node_count >> 16) & 0xFF;
        let num_nodes = node_count & 0xFF;
        
        serial_println!("HDA: Codec {} has {} nodes starting at {}", codec, num_nodes, start_nid);
        
        // Power on all nodes
        for nid in start_nid..start_nid + num_nodes {
            self.send_command(codec, nid as u8, VERB_SET_POWER_STATE, 0).ok();
        }
        
        Ok(())
    }
}

impl AudioDriver for HdaController {
    fn init(&mut self) -> Result<(), &'static str> {
        serial_println!("HDA: Initializing controller at 0x{:x}", self.base_addr);
        
        // Reset controller
        self.reset_controller()?;
        
        unsafe {
            // Read capabilities
            let gcap = self.read16(HDA_REG_GCAP);
            let num_iss = ((gcap >> 8) & 0x0F) + 1;
            let num_oss = ((gcap >> 12) & 0x0F) + 1;
            let num_bss = ((gcap >> 3) & 0x1F) + 1;
            
            serial_println!("HDA: {} input, {} output, {} bidirectional streams", 
                          num_iss, num_oss, num_bss);
            
            // Get version
            let vmaj = self.read8(HDA_REG_VMAJ);
            let vmin = self.read8(HDA_REG_VMIN);
            serial_println!("HDA: Version {}.{}", vmaj, vmin);
        }
        
        // Setup CORB/RIRB for codec communication
        self.setup_corb_rirb()?;
        
        // Enumerate and initialize codecs
        self.enumerate_codecs()?;
        
        // Initialize streams
        unsafe {
            let gcap = self.read16(HDA_REG_GCAP);
            let num_streams = ((gcap >> 8) & 0x0F) + ((gcap >> 12) & 0x0F) + 2;
            
            for i in 0..num_streams {
                let offset = 0x80 + (i as usize * 0x20);
                self.streams.push(HdaStreamDescriptor {
                    index: i as u8,
                    offset,
                    bdl: Vec::new(),
                    buffers: Vec::new(),
                    format: AudioFormat::default(),
                    active: false,
                });
            }
        }
        
        serial_println!("HDA: Initialization complete");
        Ok(())
    }
    
    fn get_capabilities(&self) -> AudioCaps {
        self.capabilities.clone()
    }
    
    fn open_stream(&mut self, direction: StreamDirection, format: AudioFormat) -> Result<Box<dyn AudioStream>, &'static str> {
        // Find available stream
        for stream in &mut self.streams {
            if !stream.active {
                stream.active = true;
                stream.format = format.clone();
                
                let hda_stream = HdaStream::new(
                    self.base_addr,
                    stream.index,
                    stream.offset,
                    direction,
                    format,
                );
                
                return Ok(Box::new(hda_stream));
            }
        }
        
        Err("No available streams")
    }
    
    fn close_stream(&mut self, _stream: Box<dyn AudioStream>) -> Result<(), &'static str> {
        // Mark stream as inactive
        Ok(())
    }
    
    fn set_master_volume(&mut self, volume: f32) -> Result<(), &'static str> {
        // Set output amplifier gain
        let gain = (volume * 127.0) as u16;
        self.send_command(0, 0x02, VERB_SET_AMP_GAIN_MUTE, gain | 0x8000)?; // Output, unmuted
        Ok(())
    }
    
    fn get_master_volume(&self) -> f32 {
        1.0 // Default
    }
}

pub struct HdaStream {
    base_addr: u64,
    index: u8,
    offset: usize,
    direction: StreamDirection,
    format: AudioFormat,
    state: StreamState,
    position: u64,
}

impl HdaStream {
    pub fn new(base_addr: u64, index: u8, offset: usize, direction: StreamDirection, format: AudioFormat) -> Self {
        Self {
            base_addr,
            index,
            offset,
            direction,
            format,
            state: StreamState::Stopped,
            position: 0,
        }
    }
    
    unsafe fn read_sd8(&self, reg: usize) -> u8 {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + self.offset as u64 + reg as u64) as *const u8;
        addr.read_volatile()
    }
    
    unsafe fn write_sd8(&self, reg: usize, value: u8) {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + self.offset as u64 + reg as u64) as *mut u8;
        addr.write_volatile(value);
    }
    
    unsafe fn write_sd16(&self, reg: usize, value: u16) {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + self.offset as u64 + reg as u64) as *mut u16;
        addr.write_volatile(value);
    }
    
    unsafe fn write_sd32(&self, reg: usize, value: u32) {
        let addr = (PHYS_MEM_OFFSET + self.base_addr + self.offset as u64 + reg as u64) as *mut u32;
        addr.write_volatile(value);
    }
}

impl AudioStream for HdaStream {
    fn start(&mut self) -> Result<(), &'static str> {
        unsafe {
            // Set stream format
            let fmt = calculate_stream_format(&self.format);
            self.write_sd16(HDA_SD_FORMAT, fmt);
            
            // Start stream
            let ctl = self.read_sd8(HDA_SD_CTL);
            self.write_sd8(HDA_SD_CTL, ctl | SD_CTL_RUN);
        }
        
        self.state = StreamState::Playing;
        Ok(())
    }
    
    fn stop(&mut self) -> Result<(), &'static str> {
        unsafe {
            let ctl = self.read_sd8(HDA_SD_CTL);
            self.write_sd8(HDA_SD_CTL, ctl & !SD_CTL_RUN);
        }
        
        self.state = StreamState::Stopped;
        Ok(())
    }
    
    fn pause(&mut self) -> Result<(), &'static str> {
        self.stop()?;
        self.state = StreamState::Paused;
        Ok(())
    }
    
    fn resume(&mut self) -> Result<(), &'static str> {
        self.start()?;
        Ok(())
    }
    
    fn write(&mut self, _buffer: &[u8]) -> Result<usize, &'static str> {
        // Write to BDL buffers
        Ok(0)
    }
    
    fn read(&mut self, _buffer: &mut [u8]) -> Result<usize, &'static str> {
        // Read from BDL buffers
        Ok(0)
    }
    
    fn drain(&mut self) -> Result<(), &'static str> {
        self.state = StreamState::Draining;
        Ok(())
    }
    
    fn get_position(&self) -> u64 {
        self.position
    }
    
    fn get_state(&self) -> StreamState {
        self.state
    }
    
    fn set_volume(&mut self, _volume: f32) -> Result<(), &'static str> {
        Ok(())
    }
}

fn calculate_stream_format(format: &AudioFormat) -> u16 {
    let mut fmt = 0u16;
    
    // Sample rate
    let base_rate = if format.sample_rate % 44100 == 0 { 1 } else { 0 };
    let mult = if format.sample_rate > 48000 {
        if format.sample_rate > 96000 { 3 } else { 1 }
    } else { 0 };
    let div = if format.sample_rate < 44100 {
        if format.sample_rate < 22050 { 3 } else { 1 }
    } else { 0 };
    
    fmt |= (base_rate << 14) | (mult << 11) | (div << 8);
    
    // Bits per sample
    let bits = match format.format {
        SampleFormat::U8 => 0,
        SampleFormat::S16LE => 1,
        SampleFormat::S24LE => 3,
        SampleFormat::S32LE => 4,
        _ => 1,
    };
    fmt |= bits << 4;
    
    // Channels
    fmt |= (format.channels as u16 - 1) & 0x0F;
    
    fmt
}

pub fn detect_hda() -> Option<HdaController> {
    // Check for Intel HD Audio controller on PCI bus
    // Common memory-mapped base addresses
    // In reality, this would come from PCI enumeration
    None // Stub for now
}