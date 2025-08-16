pub mod postscript;
pub mod pcl;
pub mod escp;
pub mod pdf;
pub mod text;
pub mod image;
pub mod label;

use alloc::{collections::BTreeMap, string::String, vec::Vec, boxed::Box};
use spin::RwLock;

pub trait PrinterDriver: Send + Sync {
    fn name(&self) -> &str;
    fn supported_formats(&self) -> Vec<&str>;
    fn init(&mut self) -> Result<(), &'static str>;
    fn render(&self, data: &[u8], options: &super::PrintOptions) -> Result<Vec<u8>, &'static str>;
    fn send_command(&self, command: PrinterCommand) -> Result<(), &'static str>;
    fn get_status(&self) -> Result<super::PrinterStatus, &'static str>;
    fn get_capabilities(&self) -> super::PrinterCapabilities;
}

#[derive(Debug, Clone)]
pub enum PrinterCommand {
    Reset,
    FormFeed,
    LineFeed,
    SetFont(String),
    SetColor(u8, u8, u8),
    SetPosition(u32, u32),
    DrawLine(u32, u32, u32, u32),
    DrawRectangle(u32, u32, u32, u32),
    PrintText(String),
    PrintImage(Vec<u8>, u32, u32),
    SetMargins(u32, u32, u32, u32),
    StartPage,
    EndPage,
    StartDocument,
    EndDocument,
    SetDuplex(bool),
    SetCopies(u32),
    SetPaperSize(super::PaperSize),
    SetOrientation(super::Orientation),
    SetQuality(super::PrintQuality),
    CleanPrintHead,
    AlignPrintHead,
    GetSupplyLevels,
}

pub struct DriverManager {
    drivers: RwLock<BTreeMap<String, Box<dyn PrinterDriver>>>,
}

impl DriverManager {
    pub fn new() -> Self {
        Self {
            drivers: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn load_builtin_drivers(&mut self) -> Result<(), &'static str> {
        self.register_driver("postscript", Box::new(postscript::PostScriptDriver::new()))?;
        self.register_driver("pcl5", Box::new(pcl::PCL5Driver::new()))?;
        self.register_driver("pcl6", Box::new(pcl::PCL6Driver::new()))?;
        self.register_driver("escp", Box::new(escp::ESCPDriver::new()))?;
        self.register_driver("pdf", Box::new(pdf::PDFDriver::new()))?;
        self.register_driver("text", Box::new(text::TextDriver::new()))?;
        self.register_driver("image", Box::new(image::ImageDriver::new()))?;
        self.register_driver("label", Box::new(label::LabelDriver::new()))?;
        Ok(())
    }

    pub fn register_driver(&mut self, name: &str, mut driver: Box<dyn PrinterDriver>) -> Result<(), &'static str> {
        driver.init()?;
        self.drivers.write().insert(String::from(name), driver);
        Ok(())
    }

    pub fn unregister_driver(&mut self, name: &str) -> Result<(), &'static str> {
        self.drivers.write().remove(name);
        Ok(())
    }

    pub fn get_driver(&self, name: &str) -> Option<&dyn PrinterDriver> {
        self.drivers.read().get(name).map(|d| d.as_ref())
    }

    pub fn list_drivers(&self) -> Vec<String> {
        self.drivers.read().keys().cloned().collect()
    }

    pub fn send_to_printer(&self, printer_id: u32, data: Vec<u8>) -> Result<(), &'static str> {
        let subsystem = super::get_subsystem().read();
        if let Some(subsystem) = subsystem.as_ref() {
            if let Some(printer) = subsystem.get_printer(printer_id) {
                if let Some(driver) = self.drivers.read().get(&printer.capabilities.driver) {
                    driver.send_command(PrinterCommand::StartDocument)?;
                    let rendered = driver.render(&data, &super::PrintOptions::default())?;
                    self.send_to_device(printer_id, rendered)?;
                    driver.send_command(PrinterCommand::EndDocument)?;
                    Ok(())
                } else {
                    Err("Driver not found")
                }
            } else {
                Err("Printer not found")
            }
        } else {
            Err("Print subsystem not initialized")
        }
    }

    fn send_to_device(&self, printer_id: u32, data: Vec<u8>) -> Result<(), &'static str> {
        super::protocols::send_data(printer_id, data)
    }

    pub fn auto_detect_driver(&self, printer_info: &PrinterInfo) -> Option<String> {
        if printer_info.manufacturer.contains("HP") {
            if printer_info.supports_pcl6 {
                Some(String::from("pcl6"))
            } else if printer_info.supports_pcl5 {
                Some(String::from("pcl5"))
            } else {
                Some(String::from("postscript"))
            }
        } else if printer_info.manufacturer.contains("Epson") {
            Some(String::from("escp"))
        } else if printer_info.manufacturer.contains("Canon") {
            Some(String::from("postscript"))
        } else if printer_info.is_virtual {
            Some(String::from("pdf"))
        } else {
            Some(String::from("text"))
        }
    }
}

#[derive(Debug, Clone)]
pub struct PrinterInfo {
    pub manufacturer: String,
    pub model: String,
    pub serial: String,
    pub firmware_version: String,
    pub supports_postscript: bool,
    pub supports_pcl5: bool,
    pub supports_pcl6: bool,
    pub supports_pdf: bool,
    pub is_virtual: bool,
}

impl Default for super::PrintOptions {
    fn default() -> Self {
        Self {
            copies: 1,
            color_mode: super::ColorMode::Color,
            paper_size: super::PaperSize::Letter,
            orientation: super::Orientation::Portrait,
            quality: super::PrintQuality::Normal,
            duplex: false,
            collate: false,
            staple: false,
            resolution: (600, 600),
            page_range: None,
            n_up: 1,
            watermark: None,
            secure_pin: None,
        }
    }
}