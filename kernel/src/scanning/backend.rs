use alloc::{vec::Vec, string::String};
use super::{Scanner, ScannerCapabilities, ScanSettings, ScanMode, ScanSource, ImageFormat, ScannerStatus};

pub struct ScannerBackend {
    sane_backend: super::sane::SANEBackend,
    twain_backend: Option<super::twain::TWAINBackend>,
}

impl ScannerBackend {
    pub fn new() -> Self {
        Self {
            sane_backend: super::sane::SANEBackend::new(),
            twain_backend: None,
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.sane_backend.init()?;
        Ok(())
    }

    pub fn discover_devices(&mut self) -> Result<Vec<Scanner>, &'static str> {
        let mut scanners = Vec::new();
        
        scanners.push(Scanner {
            id: 1,
            name: String::from("Virtual Scanner"),
            vendor: String::from("Generic"),
            model: String::from("Virtual Scanner 1.0"),
            device_type: String::from("virtual"),
            status: ScannerStatus::Idle,
            capabilities: ScannerCapabilities {
                sources: vec![ScanSource::Flatbed, ScanSource::ADF],
                modes: vec![ScanMode::Color, ScanMode::Grayscale, ScanMode::Lineart],
                resolutions: vec![75, 100, 150, 200, 300, 600, 1200, 2400],
                max_width: 216.0,
                max_height: 356.0,
                bit_depths: vec![1, 8, 16, 24],
                supports_duplex: true,
                supports_preview: true,
                supports_ocr: true,
                formats: vec![
                    ImageFormat::JPEG,
                    ImageFormat::PNG,
                    ImageFormat::TIFF,
                    ImageFormat::PDF,
                    ImageFormat::BMP,
                ],
            },
            current_settings: ScanSettings::default(),
        });
        
        let sane_devices = self.sane_backend.get_devices()?;
        for device in sane_devices {
            scanners.push(device);
        }
        
        Ok(scanners)
    }

    pub fn start_scan(&mut self, scanner_id: u32, settings: ScanSettings) -> Result<(), &'static str> {
        self.sane_backend.start_scan(scanner_id, settings)
    }

    pub fn cancel_scan(&mut self, scanner_id: u32) -> Result<(), &'static str> {
        self.sane_backend.cancel_scan(scanner_id)
    }

    pub fn preview_scan(&mut self, scanner_id: u32, settings: ScanSettings) -> Result<Vec<u8>, &'static str> {
        self.sane_backend.preview_scan(scanner_id, settings)
    }

    pub fn set_option(&mut self, scanner_id: u32, option: &str, value: &str) -> Result<(), &'static str> {
        self.sane_backend.set_option(scanner_id, option, value)
    }

    pub fn get_option(&self, scanner_id: u32, option: &str) -> Result<String, &'static str> {
        self.sane_backend.get_option(scanner_id, option)
    }

    pub fn calibrate(&mut self, scanner_id: u32) -> Result<(), &'static str> {
        self.sane_backend.calibrate(scanner_id)
    }

    pub fn read_scan_data(&mut self, scanner_id: u32) -> Result<Vec<u8>, &'static str> {
        self.sane_backend.read_data(scanner_id)
    }

    pub fn get_scan_parameters(&self, scanner_id: u32) -> Result<ScanParameters, &'static str> {
        self.sane_backend.get_parameters(scanner_id)
    }
}

#[derive(Debug, Clone)]
pub struct ScanParameters {
    pub format: FrameFormat,
    pub last_frame: bool,
    pub bytes_per_line: u32,
    pub pixels_per_line: u32,
    pub lines: u32,
    pub depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameFormat {
    Gray,
    RGB,
    Red,
    Green,
    Blue,
}