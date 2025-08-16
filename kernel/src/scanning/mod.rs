pub mod backend;
pub mod sane;
pub mod twain;
pub mod image_processing;

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use spin::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScannerStatus {
    Idle,
    Scanning,
    WarmingUp,
    Processing,
    Error,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanMode {
    Color,
    Grayscale,
    Lineart,
    Halftone,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanSource {
    Flatbed,
    ADF,
    ADFDuplex,
    Film,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    JPEG,
    PNG,
    TIFF,
    PDF,
    BMP,
    RAW,
}

#[derive(Debug, Clone)]
pub struct Scanner {
    pub id: u32,
    pub name: String,
    pub vendor: String,
    pub model: String,
    pub device_type: String,
    pub status: ScannerStatus,
    pub capabilities: ScannerCapabilities,
    pub current_settings: ScanSettings,
}

#[derive(Debug, Clone)]
pub struct ScannerCapabilities {
    pub sources: Vec<ScanSource>,
    pub modes: Vec<ScanMode>,
    pub resolutions: Vec<u32>,
    pub max_width: f32,
    pub max_height: f32,
    pub bit_depths: Vec<u8>,
    pub supports_duplex: bool,
    pub supports_preview: bool,
    pub supports_ocr: bool,
    pub formats: Vec<ImageFormat>,
}

#[derive(Debug, Clone)]
pub struct ScanSettings {
    pub source: ScanSource,
    pub mode: ScanMode,
    pub resolution: u32,
    pub bit_depth: u8,
    pub format: ImageFormat,
    pub area: ScanArea,
    pub brightness: i32,
    pub contrast: i32,
    pub gamma: f32,
    pub threshold: u8,
    pub enable_ocr: bool,
    pub enable_deskew: bool,
    pub enable_autocrop: bool,
    pub enable_blank_detection: bool,
    pub multi_page: bool,
    pub compression_quality: u8,
}

#[derive(Debug, Clone, Copy)]
pub struct ScanArea {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone)]
pub struct ScanJob {
    pub id: u32,
    pub scanner_id: u32,
    pub settings: ScanSettings,
    pub status: ScanJobStatus,
    pub pages_scanned: u32,
    pub output_files: Vec<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanJobStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

pub struct ScanSubsystem {
    scanners: RwLock<Vec<Scanner>>,
    active_jobs: RwLock<Vec<ScanJob>>,
    backend: backend::ScannerBackend,
    job_counter: RwLock<u32>,
}

impl ScanSubsystem {
    pub fn new() -> Self {
        Self {
            scanners: RwLock::new(Vec::new()),
            active_jobs: RwLock::new(Vec::new()),
            backend: backend::ScannerBackend::new(),
            job_counter: RwLock::new(0),
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.backend.init()?;
        self.discover_scanners()?;
        Ok(())
    }

    pub fn discover_scanners(&mut self) -> Result<(), &'static str> {
        let scanners = self.backend.discover_devices()?;
        *self.scanners.write() = scanners;
        Ok(())
    }

    pub fn list_scanners(&self) -> Vec<Scanner> {
        self.scanners.read().clone()
    }

    pub fn get_scanner(&self, id: u32) -> Option<Scanner> {
        self.scanners.read().iter().find(|s| s.id == id).cloned()
    }

    pub fn start_scan(&mut self, scanner_id: u32, settings: ScanSettings) -> Result<u32, &'static str> {
        let scanner = self.get_scanner(scanner_id).ok_or("Scanner not found")?;
        
        if scanner.status != ScannerStatus::Idle {
            return Err("Scanner is busy");
        }
        
        let job_id = self.generate_job_id();
        let job = ScanJob {
            id: job_id,
            scanner_id,
            settings,
            status: ScanJobStatus::Pending,
            pages_scanned: 0,
            output_files: Vec::new(),
            error_message: None,
        };
        
        self.active_jobs.write().push(job);
        self.backend.start_scan(scanner_id, settings)?;
        
        Ok(job_id)
    }

    pub fn cancel_scan(&mut self, job_id: u32) -> Result<(), &'static str> {
        let mut jobs = self.active_jobs.write();
        if let Some(job) = jobs.iter_mut().find(|j| j.id == job_id) {
            job.status = ScanJobStatus::Cancelled;
            self.backend.cancel_scan(job.scanner_id)?;
            Ok(())
        } else {
            Err("Job not found")
        }
    }

    pub fn get_scan_progress(&self, job_id: u32) -> Option<(u32, ScanJobStatus)> {
        self.active_jobs.read()
            .iter()
            .find(|j| j.id == job_id)
            .map(|j| (j.pages_scanned, j.status))
    }

    pub fn preview_scan(&mut self, scanner_id: u32) -> Result<Vec<u8>, &'static str> {
        let mut settings = ScanSettings::default();
        settings.resolution = 75;
        settings.format = ImageFormat::JPEG;
        
        self.backend.preview_scan(scanner_id, settings)
    }

    fn generate_job_id(&self) -> u32 {
        let mut counter = self.job_counter.write();
        *counter += 1;
        *counter
    }

    pub fn set_scanner_option(&mut self, scanner_id: u32, option: &str, value: &str) -> Result<(), &'static str> {
        self.backend.set_option(scanner_id, option, value)
    }

    pub fn get_scanner_option(&self, scanner_id: u32, option: &str) -> Result<String, &'static str> {
        self.backend.get_option(scanner_id, option)
    }

    pub fn calibrate_scanner(&mut self, scanner_id: u32) -> Result<(), &'static str> {
        self.backend.calibrate(scanner_id)
    }
}

impl Default for ScanSettings {
    fn default() -> Self {
        Self {
            source: ScanSource::Flatbed,
            mode: ScanMode::Color,
            resolution: 300,
            bit_depth: 8,
            format: ImageFormat::PDF,
            area: ScanArea {
                x: 0.0,
                y: 0.0,
                width: 210.0,
                height: 297.0,
            },
            brightness: 0,
            contrast: 0,
            gamma: 1.0,
            threshold: 128,
            enable_ocr: false,
            enable_deskew: true,
            enable_autocrop: true,
            enable_blank_detection: false,
            multi_page: false,
            compression_quality: 85,
        }
    }
}

static SCAN_SUBSYSTEM: RwLock<Option<ScanSubsystem>> = RwLock::new(None);

pub fn init() -> Result<(), &'static str> {
    let mut subsystem = ScanSubsystem::new();
    subsystem.init()?;
    *SCAN_SUBSYSTEM.write() = Some(subsystem);
    Ok(())
}

pub fn get_subsystem() -> &'static RwLock<Option<ScanSubsystem>> {
    &SCAN_SUBSYSTEM
}