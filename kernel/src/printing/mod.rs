pub mod spooler;
pub mod drivers;
pub mod protocols;
pub mod job;
pub mod ppd;
pub mod filter;
pub mod pdf;
pub mod manager;
pub mod queue;

use alloc::{collections::VecDeque, string::String, vec::Vec, boxed::Box};
use spin::RwLock;
use crate::fs::File;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterStatus {
    Idle,
    Printing,
    Paused,
    Error,
    Offline,
    OutOfPaper,
    OutOfToner,
    PaperJam,
    DoorOpen,
    Maintenance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrinterType {
    Laser,
    Inkjet,
    DotMatrix,
    Thermal,
    Label,
    Plotter,
    ThreeD,
    Virtual,
    Network,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorMode {
    Monochrome,
    Grayscale,
    Color,
    CMYK,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaperSize {
    Letter,
    Legal,
    A4,
    A3,
    A5,
    Envelope,
    Custom(u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Portrait,
    Landscape,
    ReversePortrait,
    ReverseLandscape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrintQuality {
    Draft,
    Normal,
    High,
    Photo,
}

#[derive(Debug, Clone)]
pub struct PrinterCapabilities {
    pub name: String,
    pub driver: String,
    pub printer_type: PrinterType,
    pub color_modes: Vec<ColorMode>,
    pub paper_sizes: Vec<PaperSize>,
    pub duplex: bool,
    pub stapler: bool,
    pub collate: bool,
    pub max_copies: u32,
    pub resolution_dpi: Vec<(u32, u32)>,
    pub print_qualities: Vec<PrintQuality>,
    pub supports_postscript: bool,
    pub supports_pcl: bool,
    pub supports_pdf: bool,
}

#[derive(Debug, Clone)]
pub struct PrintOptions {
    pub copies: u32,
    pub color_mode: ColorMode,
    pub paper_size: PaperSize,
    pub orientation: Orientation,
    pub quality: PrintQuality,
    pub duplex: bool,
    pub collate: bool,
    pub staple: bool,
    pub resolution: (u32, u32),
    pub page_range: Option<PageRange>,
    pub n_up: u32,
    pub watermark: Option<String>,
    pub secure_pin: Option<String>,
}

#[derive(Debug, Clone)]
pub enum PageRange {
    All,
    Range(u32, u32),
    Pages(Vec<u32>),
    Odd,
    Even,
}

#[derive(Debug)]
pub struct Printer {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub location: String,
    pub status: PrinterStatus,
    pub capabilities: PrinterCapabilities,
    pub job_queue: VecDeque<job::PrintJob>,
    pub current_job: Option<job::PrintJob>,
    pub total_pages_printed: u64,
    pub supply_levels: SupplyLevels,
    pub is_default: bool,
    pub is_shared: bool,
    pub access_control: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SupplyLevels {
    pub toner_black: Option<u8>,
    pub toner_cyan: Option<u8>,
    pub toner_magenta: Option<u8>,
    pub toner_yellow: Option<u8>,
    pub paper_trays: Vec<PaperTray>,
    pub maintenance_kit: Option<u8>,
    pub waste_toner: Option<u8>,
}

#[derive(Debug, Clone)]
pub struct PaperTray {
    pub id: u32,
    pub name: String,
    pub paper_size: PaperSize,
    pub capacity: u32,
    pub level: u32,
}

pub struct PrintSubsystem {
    printers: RwLock<Vec<Printer>>,
    spooler: spooler::PrintSpooler,
    drivers: drivers::DriverManager,
    protocols: protocols::ProtocolManager,
    job_counter: RwLock<u32>,
    default_printer: RwLock<Option<u32>>,
}

impl PrintSubsystem {
    pub fn new() -> Self {
        Self {
            printers: RwLock::new(Vec::new()),
            spooler: spooler::PrintSpooler::new(),
            drivers: drivers::DriverManager::new(),
            protocols: protocols::ProtocolManager::new(),
            job_counter: RwLock::new(0),
            default_printer: RwLock::new(None),
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.drivers.load_builtin_drivers()?;
        self.protocols.init_protocols()?;
        self.discover_printers()?;
        self.spooler.start()?;
        Ok(())
    }

    pub fn discover_printers(&mut self) -> Result<(), &'static str> {
        self.protocols.discover_usb_printers()?;
        self.protocols.discover_network_printers()?;
        self.add_pdf_printer()?;
        Ok(())
    }

    fn add_pdf_printer(&mut self) -> Result<(), &'static str> {
        let pdf_printer = Printer {
            id: self.generate_printer_id(),
            name: String::from("PDF Printer"),
            description: String::from("Virtual PDF printer"),
            location: String::from("Local"),
            status: PrinterStatus::Idle,
            capabilities: PrinterCapabilities {
                name: String::from("PDF Printer"),
                driver: String::from("pdf"),
                printer_type: PrinterType::Virtual,
                color_modes: vec![ColorMode::Color, ColorMode::Grayscale],
                paper_sizes: vec![
                    PaperSize::Letter,
                    PaperSize::Legal,
                    PaperSize::A4,
                    PaperSize::A3,
                ],
                duplex: true,
                stapler: false,
                collate: true,
                max_copies: 999,
                resolution_dpi: vec![(300, 300), (600, 600), (1200, 1200)],
                print_qualities: vec![
                    PrintQuality::Draft,
                    PrintQuality::Normal,
                    PrintQuality::High,
                ],
                supports_postscript: true,
                supports_pcl: false,
                supports_pdf: true,
            },
            job_queue: VecDeque::new(),
            current_job: None,
            total_pages_printed: 0,
            supply_levels: SupplyLevels {
                toner_black: None,
                toner_cyan: None,
                toner_magenta: None,
                toner_yellow: None,
                paper_trays: vec![],
                maintenance_kit: None,
                waste_toner: None,
            },
            is_default: true,
            is_shared: false,
            access_control: vec![],
        };

        self.add_printer(pdf_printer);
        Ok(())
    }

    fn generate_printer_id(&self) -> u32 {
        let mut counter = self.job_counter.write();
        *counter += 1;
        *counter
    }

    pub fn add_printer(&mut self, printer: Printer) {
        let mut printers = self.printers.write();
        if printer.is_default {
            *self.default_printer.write() = Some(printer.id);
        }
        printers.push(printer);
    }

    pub fn remove_printer(&mut self, id: u32) -> Result<(), &'static str> {
        let mut printers = self.printers.write();
        printers.retain(|p| p.id != id);
        Ok(())
    }

    pub fn get_printer(&self, id: u32) -> Option<Printer> {
        let printers = self.printers.read();
        printers.iter().find(|p| p.id == id).cloned()
    }

    pub fn list_printers(&self) -> Vec<Printer> {
        self.printers.read().clone()
    }

    pub fn set_default_printer(&mut self, id: u32) -> Result<(), &'static str> {
        let printers = self.printers.read();
        if printers.iter().any(|p| p.id == id) {
            *self.default_printer.write() = Some(id);
            Ok(())
        } else {
            Err("Printer not found")
        }
    }

    pub fn get_default_printer(&self) -> Option<u32> {
        *self.default_printer.read()
    }

    pub fn submit_job(
        &mut self,
        printer_id: u32,
        file: File,
        options: PrintOptions,
    ) -> Result<u32, &'static str> {
        let job = job::PrintJob::new(
            self.generate_job_id(),
            printer_id,
            file,
            options,
        );
        
        let job_id = job.id;
        self.spooler.add_job(job)?;
        Ok(job_id)
    }

    fn generate_job_id(&self) -> u32 {
        let mut counter = self.job_counter.write();
        *counter += 1;
        *counter
    }

    pub fn cancel_job(&mut self, job_id: u32) -> Result<(), &'static str> {
        self.spooler.cancel_job(job_id)
    }

    pub fn pause_printer(&mut self, printer_id: u32) -> Result<(), &'static str> {
        let mut printers = self.printers.write();
        if let Some(printer) = printers.iter_mut().find(|p| p.id == printer_id) {
            printer.status = PrinterStatus::Paused;
            Ok(())
        } else {
            Err("Printer not found")
        }
    }

    pub fn resume_printer(&mut self, printer_id: u32) -> Result<(), &'static str> {
        let mut printers = self.printers.write();
        if let Some(printer) = printers.iter_mut().find(|p| p.id == printer_id) {
            printer.status = PrinterStatus::Idle;
            Ok(())
        } else {
            Err("Printer not found")
        }
    }

    pub fn get_printer_status(&self, printer_id: u32) -> Option<PrinterStatus> {
        let printers = self.printers.read();
        printers.iter().find(|p| p.id == printer_id).map(|p| p.status)
    }

    pub fn get_supply_levels(&self, printer_id: u32) -> Option<SupplyLevels> {
        let printers = self.printers.read();
        printers.iter().find(|p| p.id == printer_id).map(|p| p.supply_levels.clone())
    }
}

static PRINT_SUBSYSTEM: RwLock<Option<PrintSubsystem>> = RwLock::new(None);

pub fn init() -> Result<(), &'static str> {
    let mut subsystem = PrintSubsystem::new();
    subsystem.init()?;
    *PRINT_SUBSYSTEM.write() = Some(subsystem);
    Ok(())
}

pub fn get_subsystem() -> &'static RwLock<Option<PrintSubsystem>> {
    &PRINT_SUBSYSTEM
}