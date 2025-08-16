use alloc::vec::Vec;
use alloc::string::String;
use alloc::sync::Arc;
use spin::RwLock;
use alloc::collections::BTreeMap;
use super::{MediaError, MediaFormat, MediaType, PixelFormat, AudioFormat};
use super::buffer::Buffer;

pub trait CaptureDevice: Send + Sync {
    fn get_name(&self) -> &str;
    fn get_device_type(&self) -> DeviceType;
    fn get_capabilities(&self) -> DeviceCapabilities;
    fn open(&mut self) -> Result<(), MediaError>;
    fn close(&mut self) -> Result<(), MediaError>;
    fn start_capture(&mut self) -> Result<(), MediaError>;
    fn stop_capture(&mut self) -> Result<(), MediaError>;
    fn capture_frame(&mut self) -> Result<CapturedFrame, MediaError>;
    fn set_format(&mut self, format: &MediaFormat) -> Result<(), MediaError>;
    fn get_format(&self) -> MediaFormat;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Camera,
    Microphone,
    Screen,
    VirtualCamera,
    VirtualMicrophone,
    Loopback,
}

#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    pub supported_formats: Vec<MediaFormat>,
    pub min_resolution: Option<(u32, u32)>,
    pub max_resolution: Option<(u32, u32)>,
    pub supported_framerates: Vec<f32>,
    pub supported_sample_rates: Vec<u32>,
    pub supported_channels: Vec<u32>,
    pub has_autofocus: bool,
    pub has_zoom: bool,
    pub has_flash: bool,
}

#[derive(Debug)]
pub enum CapturedFrame {
    Video(VideoFrame),
    Audio(AudioFrame),
}

#[derive(Debug)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub pixel_format: PixelFormat,
    pub timestamp: u64,
}

#[derive(Debug)]
pub struct AudioFrame {
    pub data: Vec<u8>,
    pub sample_rate: u32,
    pub channels: u32,
    pub format: AudioFormat,
    pub timestamp: u64,
}

pub struct CameraDevice {
    name: String,
    device_id: usize,
    format: MediaFormat,
    is_open: bool,
    is_capturing: bool,
    frame_count: u64,
}

impl CameraDevice {
    pub fn new(name: &str, device_id: usize) -> Self {
        Self {
            name: String::from(name),
            device_id,
            format: MediaFormat {
                media_type: MediaType::Video,
                codec: String::from("raw"),
                bitrate: None,
                sample_rate: None,
                channels: None,
                width: Some(1920),
                height: Some(1080),
                framerate: Some(30.0),
                pixel_format: Some(PixelFormat::YUV420P),
                audio_format: None,
                extra_data: Vec::new(),
            },
            is_open: false,
            is_capturing: false,
            frame_count: 0,
        }
    }
}

impl CaptureDevice for CameraDevice {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Camera
    }

    fn get_capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            supported_formats: vec![self.format.clone()],
            min_resolution: Some((640, 480)),
            max_resolution: Some((3840, 2160)),
            supported_framerates: vec![24.0, 25.0, 30.0, 60.0],
            supported_sample_rates: vec![],
            supported_channels: vec![],
            has_autofocus: true,
            has_zoom: true,
            has_flash: false,
        }
    }

    fn open(&mut self) -> Result<(), MediaError> {
        if self.is_open {
            return Err(MediaError::InvalidState);
        }
        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<(), MediaError> {
        if !self.is_open {
            return Err(MediaError::InvalidState);
        }
        if self.is_capturing {
            self.stop_capture()?;
        }
        self.is_open = false;
        Ok(())
    }

    fn start_capture(&mut self) -> Result<(), MediaError> {
        if !self.is_open || self.is_capturing {
            return Err(MediaError::InvalidState);
        }
        self.is_capturing = true;
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), MediaError> {
        if !self.is_capturing {
            return Err(MediaError::InvalidState);
        }
        self.is_capturing = false;
        Ok(())
    }

    fn capture_frame(&mut self) -> Result<CapturedFrame, MediaError> {
        if !self.is_capturing {
            return Err(MediaError::InvalidState);
        }

        let width = self.format.width.unwrap_or(1920);
        let height = self.format.height.unwrap_or(1080);
        let size = (width * height * 3 / 2) as usize;
        
        self.frame_count += 1;
        
        Ok(CapturedFrame::Video(VideoFrame {
            data: vec![0; size],
            width,
            height,
            pixel_format: self.format.pixel_format.unwrap_or(PixelFormat::YUV420P),
            timestamp: self.frame_count * 1000 / 30,
        }))
    }

    fn set_format(&mut self, format: &MediaFormat) -> Result<(), MediaError> {
        if self.is_capturing {
            return Err(MediaError::InvalidState);
        }
        self.format = format.clone();
        Ok(())
    }

    fn get_format(&self) -> MediaFormat {
        self.format.clone()
    }
}

pub struct MicrophoneDevice {
    name: String,
    device_id: usize,
    format: MediaFormat,
    is_open: bool,
    is_capturing: bool,
    sample_count: u64,
}

impl MicrophoneDevice {
    pub fn new(name: &str, device_id: usize) -> Self {
        Self {
            name: String::from(name),
            device_id,
            format: MediaFormat {
                media_type: MediaType::Audio,
                codec: String::from("pcm"),
                bitrate: None,
                sample_rate: Some(48000),
                channels: Some(2),
                width: None,
                height: None,
                framerate: None,
                pixel_format: None,
                audio_format: Some(AudioFormat::F32LE),
                extra_data: Vec::new(),
            },
            is_open: false,
            is_capturing: false,
            sample_count: 0,
        }
    }
}

impl CaptureDevice for MicrophoneDevice {
    fn get_name(&self) -> &str {
        &self.name
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Microphone
    }

    fn get_capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            supported_formats: vec![self.format.clone()],
            min_resolution: None,
            max_resolution: None,
            supported_framerates: vec![],
            supported_sample_rates: vec![8000, 16000, 22050, 44100, 48000, 96000],
            supported_channels: vec![1, 2, 4, 6, 8],
            has_autofocus: false,
            has_zoom: false,
            has_flash: false,
        }
    }

    fn open(&mut self) -> Result<(), MediaError> {
        if self.is_open {
            return Err(MediaError::InvalidState);
        }
        self.is_open = true;
        Ok(())
    }

    fn close(&mut self) -> Result<(), MediaError> {
        if !self.is_open {
            return Err(MediaError::InvalidState);
        }
        if self.is_capturing {
            self.stop_capture()?;
        }
        self.is_open = false;
        Ok(())
    }

    fn start_capture(&mut self) -> Result<(), MediaError> {
        if !self.is_open || self.is_capturing {
            return Err(MediaError::InvalidState);
        }
        self.is_capturing = true;
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), MediaError> {
        if !self.is_capturing {
            return Err(MediaError::InvalidState);
        }
        self.is_capturing = false;
        Ok(())
    }

    fn capture_frame(&mut self) -> Result<CapturedFrame, MediaError> {
        if !self.is_capturing {
            return Err(MediaError::InvalidState);
        }

        let sample_rate = self.format.sample_rate.unwrap_or(48000);
        let channels = self.format.channels.unwrap_or(2) as usize;
        let samples_per_frame = 1024;
        let size = samples_per_frame * channels * 4;
        
        self.sample_count += samples_per_frame as u64;
        
        Ok(CapturedFrame::Audio(AudioFrame {
            data: vec![0; size],
            sample_rate,
            channels: channels as u32,
            format: self.format.audio_format.unwrap_or(AudioFormat::F32LE),
            timestamp: self.sample_count * 1000000 / sample_rate as u64,
        }))
    }

    fn set_format(&mut self, format: &MediaFormat) -> Result<(), MediaError> {
        if self.is_capturing {
            return Err(MediaError::InvalidState);
        }
        self.format = format.clone();
        Ok(())
    }

    fn get_format(&self) -> MediaFormat {
        self.format.clone()
    }
}

pub struct ScreenCapture {
    format: MediaFormat,
    is_capturing: bool,
    frame_count: u64,
}

impl ScreenCapture {
    pub fn new() -> Self {
        Self {
            format: MediaFormat {
                media_type: MediaType::Video,
                codec: String::from("raw"),
                bitrate: None,
                sample_rate: None,
                channels: None,
                width: Some(1920),
                height: Some(1080),
                framerate: Some(60.0),
                pixel_format: Some(PixelFormat::RGBA32),
                audio_format: None,
                extra_data: Vec::new(),
            },
            is_capturing: false,
            frame_count: 0,
        }
    }
}

impl CaptureDevice for ScreenCapture {
    fn get_name(&self) -> &str {
        "Screen Capture"
    }

    fn get_device_type(&self) -> DeviceType {
        DeviceType::Screen
    }

    fn get_capabilities(&self) -> DeviceCapabilities {
        DeviceCapabilities {
            supported_formats: vec![self.format.clone()],
            min_resolution: Some((640, 480)),
            max_resolution: Some((7680, 4320)),
            supported_framerates: vec![15.0, 30.0, 60.0, 120.0],
            supported_sample_rates: vec![],
            supported_channels: vec![],
            has_autofocus: false,
            has_zoom: false,
            has_flash: false,
        }
    }

    fn open(&mut self) -> Result<(), MediaError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), MediaError> {
        if self.is_capturing {
            self.stop_capture()?;
        }
        Ok(())
    }

    fn start_capture(&mut self) -> Result<(), MediaError> {
        self.is_capturing = true;
        Ok(())
    }

    fn stop_capture(&mut self) -> Result<(), MediaError> {
        self.is_capturing = false;
        Ok(())
    }

    fn capture_frame(&mut self) -> Result<CapturedFrame, MediaError> {
        if !self.is_capturing {
            return Err(MediaError::InvalidState);
        }

        let width = self.format.width.unwrap_or(1920);
        let height = self.format.height.unwrap_or(1080);
        let size = (width * height * 4) as usize;
        
        self.frame_count += 1;
        
        Ok(CapturedFrame::Video(VideoFrame {
            data: vec![0; size],
            width,
            height,
            pixel_format: PixelFormat::RGBA32,
            timestamp: self.frame_count * 1000 / 60,
        }))
    }

    fn set_format(&mut self, format: &MediaFormat) -> Result<(), MediaError> {
        self.format = format.clone();
        Ok(())
    }

    fn get_format(&self) -> MediaFormat {
        self.format.clone()
    }
}

pub struct CaptureManager {
    devices: RwLock<Vec<Arc<RwLock<dyn CaptureDevice>>>>,
}

impl CaptureManager {
    pub fn new() -> Self {
        Self {
            devices: RwLock::new(Vec::new()),
        }
    }

    pub fn enumerate_devices(&self) -> Vec<DeviceInfo> {
        self.devices.read()
            .iter()
            .map(|device| {
                let d = device.read();
                DeviceInfo {
                    name: String::from(d.get_name()),
                    device_type: d.get_device_type(),
                    capabilities: d.get_capabilities(),
                }
            })
            .collect()
    }

    pub fn register_device(&self, device: Arc<RwLock<dyn CaptureDevice>>) {
        self.devices.write().push(device);
    }

    pub fn get_device(&self, name: &str) -> Option<Arc<RwLock<dyn CaptureDevice>>> {
        self.devices.read()
            .iter()
            .find(|d| d.read().get_name() == name)
            .cloned()
    }
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub device_type: DeviceType,
    pub capabilities: DeviceCapabilities,
}