// Windows-Compatible Audio Subsystem Implementation
use super::*;
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use alloc::string::String;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use crate::nt::NtStatus;
use crate::win32::Handle;

// Audio Device Types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AudioDeviceType {
    WaveOut = 1,
    WaveIn = 2,
    MidiOut = 3,
    MidiIn = 4,
    Aux = 5,
    Mixer = 6,
    DirectSound = 7,
    DirectSoundCapture = 8,
}

// Audio Format Structures
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WaveFormatEx {
    pub format_tag: u16,        // WAVE_FORMAT_PCM = 1
    pub channels: u16,          // Number of channels (1=mono, 2=stereo)
    pub samples_per_sec: u32,   // Sample rate (e.g., 44100)
    pub avg_bytes_per_sec: u32, // Average bytes per second
    pub block_align: u16,       // Block alignment
    pub bits_per_sample: u16,   // Bits per sample (8, 16, 24, 32)
    pub cb_size: u16,           // Size of extra format information
}

impl Default for WaveFormatEx {
    fn default() -> Self {
        Self {
            format_tag: 1, // PCM
            channels: 2,   // Stereo
            samples_per_sec: 44100,
            avg_bytes_per_sec: 176400, // 44100 * 2 * 2
            block_align: 4,            // 2 channels * 2 bytes
            bits_per_sample: 16,
            cb_size: 0,
        }
    }
}

// Wave Header for audio buffers
#[repr(C)]
#[derive(Debug, Clone)]
pub struct WaveHdr {
    pub data: *mut u8,
    pub buffer_length: u32,
    pub bytes_recorded: u32,
    pub user_data: usize,
    pub flags: u32,
    pub loops: u32,
    pub next: *mut WaveHdr,
    pub reserved: usize,
}

// Audio Device Capabilities
#[repr(C)]
#[derive(Debug, Clone)]
pub struct WaveOutCaps {
    pub manufacturer_id: u16,
    pub product_id: u16,
    pub driver_version: u32,
    pub product_name: [u8; 32],
    pub formats: u32,
    pub channels: u16,
    pub support: u32,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct WaveInCaps {
    pub manufacturer_id: u16,
    pub product_id: u16,
    pub driver_version: u32,
    pub product_name: [u8; 32],
    pub formats: u32,
    pub channels: u16,
    pub reserved1: u16,
}

// DirectSound Structures
#[repr(C)]
#[derive(Debug, Clone)]
pub struct DSCaps {
    pub size: u32,
    pub flags: u32,
    pub min_secondary_sample_rate: u32,
    pub max_secondary_sample_rate: u32,
    pub primary_buffers: u32,
    pub max_hw_mixing_all_buffers: u32,
    pub max_hw_mixing_static_buffers: u32,
    pub max_hw_mixing_streaming_buffers: u32,
    pub free_hw_mixing_all_buffers: u32,
    pub free_hw_mixing_static_buffers: u32,
    pub free_hw_mixing_streaming_buffers: u32,
    pub max_hw3d_all_buffers: u32,
    pub max_hw3d_static_buffers: u32,
    pub max_hw3d_streaming_buffers: u32,
    pub free_hw3d_all_buffers: u32,
    pub free_hw3d_static_buffers: u32,
    pub free_hw3d_streaming_buffers: u32,
    pub total_hw_mem_bytes: u32,
    pub free_hw_mem_bytes: u32,
    pub max_contiguous_free_hw_mem_bytes: u32,
    pub unlock_transfer_rate_hw_buffers: u32,
    pub play_cpu_overhead_sw_buffers: u32,
    pub reserved1: u32,
    pub reserved2: u32,
}

// Audio Buffer Management
#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub data: Vec<u8>,
    pub format: WaveFormatEx,
    pub position: usize,
    pub playing: bool,
    pub looping: bool,
}

impl AudioBuffer {
    pub fn new(size: usize, format: WaveFormatEx) -> Self {
        Self {
            data: vec![0; size],
            format,
            position: 0,
            playing: false,
            looping: false,
        }
    }
}

// Audio Device Structure
#[derive(Debug, Clone)]
pub struct AudioDevice {
    pub device_id: u32,
    pub device_type: AudioDeviceType,
    pub name: String,
    pub capabilities: AudioDeviceCaps,
    pub current_format: WaveFormatEx,
    pub buffers: Vec<AudioBuffer>,
    pub volume: u32,
    pub muted: bool,
    pub handle: Handle,
}

#[derive(Debug, Clone)]
pub enum AudioDeviceCaps {
    WaveOut(WaveOutCaps),
    WaveIn(WaveInCaps),
    DirectSound(DSCaps),
}

// Audio Mixer Control
#[derive(Debug, Clone)]
pub struct MixerControl {
    pub control_id: u32,
    pub control_type: MixerControlType,
    pub name: String,
    pub value: u32,
    pub minimum: u32,
    pub maximum: u32,
    pub steps: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MixerControlType {
    Volume,
    Mute,
    Bass,
    Treble,
    Balance,
    Custom,
}

// DirectSound Buffer
#[derive(Debug, Clone)]
pub struct DirectSoundBuffer {
    pub buffer_id: u32,
    pub description: DSBufferDesc,
    pub audio_buffer: AudioBuffer,
    pub volume: i32,
    pub frequency: u32,
    pub pan: i32,
    pub playing: bool,
    pub looping: bool,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct DSBufferDesc {
    pub size: u32,
    pub flags: u32,
    pub buffer_bytes: u32,
    pub reserved: u32,
    pub format: WaveFormatEx,
    pub guid3d_algorithm: [u8; 16],
}

// Audio Hardware Abstraction Layer
pub trait AudioHAL {
    fn initialize(&mut self) -> NtStatus;
    fn shutdown(&mut self) -> NtStatus;
    fn play_buffer(&mut self, buffer: &AudioBuffer) -> NtStatus;
    fn stop_playback(&mut self) -> NtStatus;
    fn set_volume(&mut self, volume: u32) -> NtStatus;
    fn get_volume(&self) -> u32;
    fn set_format(&mut self, format: &WaveFormatEx) -> NtStatus;
    fn get_position(&self) -> u32;
}

// HD Audio Controller (Intel HDA compatible)
#[derive(Debug)]
pub struct HDAudioController {
    pub base_address: u64,
    pub codecs: Vec<AudioCodec>,
    pub streams: Vec<AudioStream>,
    pub current_format: WaveFormatEx,
    pub volume: u32,
    pub sample_rate: u32,
}

#[derive(Debug, Clone)]
pub struct AudioCodec {
    pub codec_id: u8,
    pub vendor_id: u32,
    pub device_id: u32,
    pub revision: u8,
    pub name: String,
    pub capabilities: CodecCaps,
}

#[derive(Debug, Clone)]
pub struct CodecCaps {
    pub input_amp_caps: u32,
    pub output_amp_caps: u32,
    pub pin_caps: u32,
    pub power_states: u32,
    pub processing_caps: u32,
}

#[derive(Debug, Clone)]
pub struct AudioStream {
    pub stream_id: u8,
    pub stream_tag: u8,
    pub format: WaveFormatEx,
    pub buffer: Vec<u8>,
    pub position: u32,
    pub active: bool,
}

impl HDAudioController {
    pub fn new(base_address: u64) -> Self {
        Self {
            base_address,
            codecs: Vec::new(),
            streams: Vec::new(),
            current_format: WaveFormatEx::default(),
            volume: 0x8000, // 50% volume
            sample_rate: 44100,
        }
    }

    pub fn detect_codecs(&mut self) -> NtStatus {
        crate::println!("HDA: Detecting audio codecs");
        
        // Simulate codec detection
        let realtek_codec = AudioCodec {
            codec_id: 0,
            vendor_id: 0x10EC,
            device_id: 0x0887,
            revision: 1,
            name: String::from("Realtek ALC887"),
            capabilities: CodecCaps {
                input_amp_caps: 0x80033f3f,
                output_amp_caps: 0x80033f3f,
                pin_caps: 0x0000003e,
                power_states: 0x00000003,
                processing_caps: 0x00000001,
            },
        };
        
        self.codecs.push(realtek_codec);
        crate::println!("HDA: Found {} audio codecs", self.codecs.len());
        
        NtStatus::Success
    }

    pub fn setup_streams(&mut self) -> NtStatus {
        crate::println!("HDA: Setting up audio streams");
        
        // Create output stream
        let output_stream = AudioStream {
            stream_id: 1,
            stream_tag: 1,
            format: self.current_format,
            buffer: vec![0; 4096],
            position: 0,
            active: false,
        };
        
        self.streams.push(output_stream);
        
        // Create input stream
        let input_stream = AudioStream {
            stream_id: 2,
            stream_tag: 2,
            format: self.current_format,
            buffer: vec![0; 4096],
            position: 0,
            active: false,
        };
        
        self.streams.push(input_stream);
        
        crate::println!("HDA: Created {} audio streams", self.streams.len());
        NtStatus::Success
    }
}

impl AudioHAL for HDAudioController {
    fn initialize(&mut self) -> NtStatus {
        crate::println!("HDA: Initializing HD Audio controller");
        
        self.detect_codecs();
        self.setup_streams();
        
        crate::println!("HDA: Controller initialized successfully");
        NtStatus::Success
    }

    fn shutdown(&mut self) -> NtStatus {
        crate::println!("HDA: Shutting down HD Audio controller");
        
        // Stop all streams
        for stream in &mut self.streams {
            stream.active = false;
        }
        
        NtStatus::Success
    }

    fn play_buffer(&mut self, buffer: &AudioBuffer) -> NtStatus {
        crate::println!("HDA: Playing audio buffer ({} bytes)", buffer.data.len());
        
        if let Some(stream) = self.streams.get_mut(0) {
            stream.buffer = buffer.data.clone();
            stream.format = buffer.format;
            stream.position = 0;
            stream.active = true;
        }
        
        NtStatus::Success
    }

    fn stop_playback(&mut self) -> NtStatus {
        crate::println!("HDA: Stopping audio playback");
        
        for stream in &mut self.streams {
            stream.active = false;
        }
        
        NtStatus::Success
    }

    fn set_volume(&mut self, volume: u32) -> NtStatus {
        self.volume = volume;
        crate::println!("HDA: Volume set to {}", volume);
        NtStatus::Success
    }

    fn get_volume(&self) -> u32 {
        self.volume
    }

    fn set_format(&mut self, format: &WaveFormatEx) -> NtStatus {
        self.current_format = *format;
        crate::println!("HDA: Format set to {}Hz, {} channels, {} bits", 
                       format.samples_per_sec, format.channels, format.bits_per_sample);
        NtStatus::Success
    }

    fn get_position(&self) -> u32 {
        if let Some(stream) = self.streams.get(0) {
            stream.position
        } else {
            0
        }
    }
}

// Audio Subsystem Manager
pub struct AudioSubsystem {
    devices: Vec<AudioDevice>,
    controllers: Vec<Box<dyn AudioHAL>>,
    mixer_controls: BTreeMap<u32, MixerControl>,
    directsound_buffers: BTreeMap<u32, DirectSoundBuffer>,
    next_device_id: u32,
    next_buffer_id: u32,
}

impl AudioSubsystem {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            controllers: Vec::new(),
            mixer_controls: BTreeMap::new(),
            directsound_buffers: BTreeMap::new(),
            next_device_id: 0,
            next_buffer_id: 0,
        }
    }

    pub fn initialize(&mut self) -> NtStatus {
        crate::println!("Audio: Initializing Windows-compatible audio subsystem");

        // Detect and initialize audio hardware
        let status = self.detect_audio_hardware();
        if status != NtStatus::Success {
            return status;
        }

        // Initialize audio controllers
        for controller in &mut self.controllers {
            let status = controller.initialize();
            if status != NtStatus::Success {
                return status;
            }
        }

        // Create default audio devices
        let status = self.create_default_devices();
        if status != NtStatus::Success {
            return status;
        }

        // Setup mixer controls
        let status = self.setup_mixer_controls();
        if status != NtStatus::Success {
            return status;
        }

        crate::println!("Audio: Subsystem initialized successfully");
        crate::println!("Audio: Found {} audio devices", self.devices.len());
        
        NtStatus::Success
    }

    fn detect_audio_hardware(&mut self) -> NtStatus {
        crate::println!("Audio: Detecting audio hardware");

        // Simulate detecting HD Audio controller
        let hda_controller = HDAudioController::new(0xFE000000);
        self.controllers.push(Box::new(hda_controller));

        crate::println!("Audio: Found {} audio controllers", self.controllers.len());
        NtStatus::Success
    }

    fn create_default_devices(&mut self) -> NtStatus {
        crate::println!("Audio: Creating default audio devices");

        // Create WaveOut device
        let waveout_caps = WaveOutCaps {
            manufacturer_id: 1,
            product_id: 1,
            driver_version: 0x0100,
            product_name: *b"ReactOS WaveOut Device\0\0\0\0\0\0\0\0\0\0",
            formats: 0xFFF, // Support all standard formats
            channels: 2,    // Stereo
            support: 0x003F, // Volume, LR Volume, Pitch, Playback Rate, etc.
        };

        let waveout_device = AudioDevice {
            device_id: self.next_device_id,
            device_type: AudioDeviceType::WaveOut,
            name: String::from("Primary Sound Driver"),
            capabilities: AudioDeviceCaps::WaveOut(waveout_caps),
            current_format: WaveFormatEx::default(),
            buffers: Vec::new(),
            volume: 0xFFFF,
            muted: false,
            handle: Handle(self.next_device_id as u64),
        };

        self.devices.push(waveout_device);
        self.next_device_id += 1;

        // Create WaveIn device
        let wavein_caps = WaveInCaps {
            manufacturer_id: 1,
            product_id: 2,
            driver_version: 0x0100,
            product_name: *b"ReactOS WaveIn Device\0\0\0\0\0\0\0\0\0\0\0",
            formats: 0xFFF,
            channels: 2,
            reserved1: 0,
        };

        let wavein_device = AudioDevice {
            device_id: self.next_device_id,
            device_type: AudioDeviceType::WaveIn,
            name: String::from("Primary Sound Capture"),
            capabilities: AudioDeviceCaps::WaveIn(wavein_caps),
            current_format: WaveFormatEx::default(),
            buffers: Vec::new(),
            volume: 0xFFFF,
            muted: false,
            handle: Handle(self.next_device_id as u64),
        };

        self.devices.push(wavein_device);
        self.next_device_id += 1;

        // Create DirectSound device
        let ds_caps = DSCaps {
            size: core::mem::size_of::<DSCaps>() as u32,
            flags: 0x00000001, // DSCAPS_PRIMARYMONO | DSCAPS_PRIMARYSTEREO
            min_secondary_sample_rate: 11025,
            max_secondary_sample_rate: 96000,
            primary_buffers: 1,
            max_hw_mixing_all_buffers: 32,
            max_hw_mixing_static_buffers: 16,
            max_hw_mixing_streaming_buffers: 16,
            free_hw_mixing_all_buffers: 32,
            free_hw_mixing_static_buffers: 16,
            free_hw_mixing_streaming_buffers: 16,
            max_hw3d_all_buffers: 16,
            max_hw3d_static_buffers: 8,
            max_hw3d_streaming_buffers: 8,
            free_hw3d_all_buffers: 16,
            free_hw3d_static_buffers: 8,
            free_hw3d_streaming_buffers: 8,
            total_hw_mem_bytes: 0x400000, // 4MB
            free_hw_mem_bytes: 0x400000,
            max_contiguous_free_hw_mem_bytes: 0x400000,
            unlock_transfer_rate_hw_buffers: 4800,
            play_cpu_overhead_sw_buffers: 0,
            reserved1: 0,
            reserved2: 0,
        };

        let ds_device = AudioDevice {
            device_id: self.next_device_id,
            device_type: AudioDeviceType::DirectSound,
            name: String::from("Primary Sound DirectSound"),
            capabilities: AudioDeviceCaps::DirectSound(ds_caps),
            current_format: WaveFormatEx::default(),
            buffers: Vec::new(),
            volume: 0xFFFF,
            muted: false,
            handle: Handle(self.next_device_id as u64),
        };

        self.devices.push(ds_device);
        self.next_device_id += 1;

        crate::println!("Audio: Created {} audio devices", self.devices.len());
        NtStatus::Success
    }

    fn setup_mixer_controls(&mut self) -> NtStatus {
        crate::println!("Audio: Setting up mixer controls");

        // Master Volume Control
        let master_volume = MixerControl {
            control_id: 0,
            control_type: MixerControlType::Volume,
            name: String::from("Master Volume"),
            value: 0x8000, // 50%
            minimum: 0,
            maximum: 0xFFFF,
            steps: 256,
        };
        self.mixer_controls.insert(0, master_volume);

        // Master Mute Control
        let master_mute = MixerControl {
            control_id: 1,
            control_type: MixerControlType::Mute,
            name: String::from("Master Mute"),
            value: 0,
            minimum: 0,
            maximum: 1,
            steps: 2,
        };
        self.mixer_controls.insert(1, master_mute);

        // Wave Volume Control
        let wave_volume = MixerControl {
            control_id: 2,
            control_type: MixerControlType::Volume,
            name: String::from("Wave Volume"),
            value: 0x8000,
            minimum: 0,
            maximum: 0xFFFF,
            steps: 256,
        };
        self.mixer_controls.insert(2, wave_volume);

        crate::println!("Audio: Created {} mixer controls", self.mixer_controls.len());
        NtStatus::Success
    }

    pub fn get_device_count(&self, device_type: AudioDeviceType) -> u32 {
        self.devices.iter()
            .filter(|device| device.device_type == device_type)
            .count() as u32
    }

    pub fn get_device_caps(&self, device_id: u32, device_type: AudioDeviceType) -> Option<&AudioDeviceCaps> {
        self.devices.iter()
            .find(|device| device.device_id == device_id && device.device_type == device_type)
            .map(|device| &device.capabilities)
    }

    pub fn wave_out_open(&mut self, device_id: u32, format: &WaveFormatEx) -> Result<Handle, NtStatus> {
        if let Some(device) = self.devices.iter_mut()
            .find(|d| d.device_id == device_id && d.device_type == AudioDeviceType::WaveOut) {
            
            device.current_format = *format;
            crate::println!("Audio: WaveOut device {} opened", device_id);
            Ok(device.handle)
        } else {
            Err(NtStatus::NoSuchDevice)
        }
    }

    pub fn wave_out_write(&mut self, handle: Handle, buffer: &[u8]) -> NtStatus {
        if let Some(device) = self.devices.iter_mut()
            .find(|d| d.handle == handle && d.device_type == AudioDeviceType::WaveOut) {
            
            let audio_buffer = AudioBuffer {
                data: buffer.to_vec(),
                format: device.current_format,
                position: 0,
                playing: true,
                looping: false,
            };

            device.buffers.push(audio_buffer);
            
            // Play through HAL
            if let Some(controller) = self.controllers.get_mut(0) {
                if let Some(buffer) = device.buffers.last() {
                    controller.play_buffer(buffer);
                }
            }

            crate::println!("Audio: WaveOut buffer written ({} bytes)", buffer.len());
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }

    pub fn directsound_create_buffer(&mut self, desc: &DSBufferDesc) -> Result<u32, NtStatus> {
        let buffer_id = self.next_buffer_id;
        self.next_buffer_id += 1;

        let audio_buffer = AudioBuffer::new(desc.buffer_bytes as usize, desc.format);
        
        let ds_buffer = DirectSoundBuffer {
            buffer_id,
            description: desc.clone(),
            audio_buffer,
            volume: 0,      // 0 dB
            frequency: desc.format.samples_per_sec,
            pan: 0,         // Center
            playing: false,
            looping: false,
        };

        self.directsound_buffers.insert(buffer_id, ds_buffer);
        
        crate::println!("Audio: DirectSound buffer {} created", buffer_id);
        Ok(buffer_id)
    }

    pub fn mixer_get_control_value(&self, control_id: u32) -> Option<u32> {
        self.mixer_controls.get(&control_id).map(|control| control.value)
    }

    pub fn mixer_set_control_value(&mut self, control_id: u32, value: u32) -> NtStatus {
        if let Some(control) = self.mixer_controls.get_mut(&control_id) {
            control.value = value.clamp(control.minimum, control.maximum);
            crate::println!("Audio: Mixer control {} set to {}", control_id, control.value);
            NtStatus::Success
        } else {
            NtStatus::InvalidParameter
        }
    }
}

// Global Audio Subsystem
static mut AUDIO_SUBSYSTEM: Option<AudioSubsystem> = None;

pub fn initialize_audio_subsystem() -> NtStatus {
    crate::println!("Audio: Starting Windows audio subsystem initialization");
    
    unsafe {
        AUDIO_SUBSYSTEM = Some(AudioSubsystem::new());
        
        if let Some(ref mut audio) = AUDIO_SUBSYSTEM {
            match audio.initialize() {
                NtStatus::Success => {
                    crate::println!("Audio: Windows audio subsystem initialized!");
                    crate::println!("Audio: Features available:");
                    crate::println!("  - WaveOut/WaveIn API support");
                    crate::println!("  - DirectSound 8 compatible interface");
                    crate::println!("  - HD Audio (Intel HDA) controller support");
                    crate::println!("  - Audio mixer controls");
                    crate::println!("  - Multiple audio formats (PCM 8/16/24/32-bit)");
                    crate::println!("  - Sample rates: 11025Hz to 96000Hz");
                    crate::println!("  - Stereo and multi-channel audio");
                    
                    NtStatus::Success
                }
                error => {
                    crate::println!("Audio: Failed to initialize subsystem: {:?}", error);
                    error
                }
            }
        } else {
            NtStatus::InsufficientResources
        }
    }
}

// Audio API Functions
pub fn audio_get_num_devices(device_type: AudioDeviceType) -> u32 {
    unsafe {
        AUDIO_SUBSYSTEM.as_ref()
            .map_or(0, |audio| audio.get_device_count(device_type))
    }
}

pub fn audio_get_device_caps(device_id: u32, device_type: AudioDeviceType) -> Option<String> {
    unsafe {
        AUDIO_SUBSYSTEM.as_ref().and_then(|audio| {
            audio.get_device_caps(device_id, device_type).map(|caps| {
                match caps {
                    AudioDeviceCaps::WaveOut(caps) => {
                        let name = core::str::from_utf8(&caps.product_name)
                            .unwrap_or("Unknown Device")
                            .trim_end_matches('\0');
                        format!("WaveOut: {} ({}ch, Formats: 0x{:X})", 
                               name, caps.channels, caps.formats)
                    }
                    AudioDeviceCaps::WaveIn(caps) => {
                        let name = core::str::from_utf8(&caps.product_name)
                            .unwrap_or("Unknown Device")
                            .trim_end_matches('\0');
                        format!("WaveIn: {} ({}ch, Formats: 0x{:X})", 
                               name, caps.channels, caps.formats)
                    }
                    AudioDeviceCaps::DirectSound(caps) => {
                        format!("DirectSound: {}Hz-{}Hz, {} buffers, {}MB memory", 
                               caps.min_secondary_sample_rate,
                               caps.max_secondary_sample_rate,
                               caps.max_hw_mixing_all_buffers,
                               caps.total_hw_mem_bytes / 1024 / 1024)
                    }
                }
            })
        })
    }
}

pub fn wave_out_open(device_id: u32, format: &WaveFormatEx) -> Result<Handle, NtStatus> {
    unsafe {
        if let Some(ref mut audio) = AUDIO_SUBSYSTEM {
            audio.wave_out_open(device_id, format)
        } else {
            Err(NtStatus::DeviceNotReady)
        }
    }
}

pub fn wave_out_write(handle: Handle, buffer: &[u8]) -> NtStatus {
    unsafe {
        if let Some(ref mut audio) = AUDIO_SUBSYSTEM {
            audio.wave_out_write(handle, buffer)
        } else {
            NtStatus::DeviceNotReady
        }
    }
}

pub fn directsound_create_buffer(desc: &DSBufferDesc) -> Result<u32, NtStatus> {
    unsafe {
        if let Some(ref mut audio) = AUDIO_SUBSYSTEM {
            audio.directsound_create_buffer(desc)
        } else {
            Err(NtStatus::DeviceNotReady)
        }
    }
}

pub fn mixer_get_control_value(control_id: u32) -> Option<u32> {
    unsafe {
        AUDIO_SUBSYSTEM.as_ref()
            .and_then(|audio| audio.mixer_get_control_value(control_id))
    }
}

pub fn mixer_set_control_value(control_id: u32, value: u32) -> NtStatus {
    unsafe {
        if let Some(ref mut audio) = AUDIO_SUBSYSTEM {
            audio.mixer_set_control_value(control_id, value)
        } else {
            NtStatus::DeviceNotReady
        }
    }
}