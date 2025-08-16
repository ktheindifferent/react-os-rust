use alloc::{string::String, vec::Vec, format};
use super::{PrinterDriver, PrinterCommand};

pub struct PCL5Driver {
    current_font: u8,
    current_pitch: f32,
    page_count: u32,
}

impl PCL5Driver {
    pub fn new() -> Self {
        Self {
            current_font: 0,
            current_pitch: 10.0,
            page_count: 0,
        }
    }

    fn generate_pcl5(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut pcl = Vec::new();
        
        pcl.extend_from_slice(b"\x1B%-12345X");
        pcl.extend_from_slice(b"\x1BE");
        
        let orientation = match options.orientation {
            crate::printing::Orientation::Portrait => b"\x1B&l0O",
            crate::printing::Orientation::Landscape => b"\x1B&l1O",
            _ => b"\x1B&l0O",
        };
        pcl.extend_from_slice(orientation);
        
        let paper_size = self.get_paper_size_code(options.paper_size);
        pcl.extend_from_slice(format!("\x1B&l{}A", paper_size).as_bytes());
        
        pcl.extend_from_slice(format!("\x1B&l{}X", options.copies).as_bytes());
        
        if options.duplex {
            pcl.extend_from_slice(b"\x1B&l1S");
        }
        
        let resolution = match options.quality {
            crate::printing::PrintQuality::Draft => 150,
            crate::printing::PrintQuality::Normal => 300,
            crate::printing::PrintQuality::High => 600,
            crate::printing::PrintQuality::Photo => 1200,
        };
        pcl.extend_from_slice(format!("\x1B*t{}R", resolution).as_bytes());
        
        pcl.extend_from_slice(b"\x1B&l6D");
        pcl.extend_from_slice(b"\x1B(s0P");
        pcl.extend_from_slice(b"\x1B(s10H");
        pcl.extend_from_slice(b"\x1B(s0S");
        pcl.extend_from_slice(b"\x1B(s0B");
        pcl.extend_from_slice(b"\x1B(s4099T");
        
        let text = String::from_utf8_lossy(data);
        for line in text.lines() {
            pcl.extend_from_slice(line.as_bytes());
            pcl.extend_from_slice(b"\r\n");
        }
        
        pcl.extend_from_slice(b"\x0C");
        pcl.extend_from_slice(b"\x1BE");
        pcl.extend_from_slice(b"\x1B%-12345X");
        
        pcl
    }

    fn get_paper_size_code(&self, size: crate::printing::PaperSize) -> u8 {
        match size {
            crate::printing::PaperSize::Letter => 2,
            crate::printing::PaperSize::Legal => 3,
            crate::printing::PaperSize::A4 => 26,
            crate::printing::PaperSize::A3 => 27,
            crate::printing::PaperSize::A5 => 25,
            crate::printing::PaperSize::Envelope => 81,
            _ => 2,
        }
    }
}

impl PrinterDriver for PCL5Driver {
    fn name(&self) -> &str {
        "PCL5"
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["text/plain", "application/vnd.hp-pcl"]
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        Ok(self.generate_pcl5(data, options))
    }

    fn send_command(&self, command: PrinterCommand) -> Result<(), &'static str> {
        match command {
            PrinterCommand::Reset => Ok(()),
            PrinterCommand::FormFeed => Ok(()),
            _ => Ok(()),
        }
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("PCL5 Printer"),
            driver: String::from("pcl5"),
            printer_type: crate::printing::PrinterType::Laser,
            color_modes: vec![
                crate::printing::ColorMode::Monochrome,
                crate::printing::ColorMode::Grayscale,
            ],
            paper_sizes: vec![
                crate::printing::PaperSize::Letter,
                crate::printing::PaperSize::Legal,
                crate::printing::PaperSize::A4,
            ],
            duplex: true,
            stapler: false,
            collate: true,
            max_copies: 999,
            resolution_dpi: vec![(150, 150), (300, 300), (600, 600)],
            print_qualities: vec![
                crate::printing::PrintQuality::Draft,
                crate::printing::PrintQuality::Normal,
                crate::printing::PrintQuality::High,
            ],
            supports_postscript: false,
            supports_pcl: true,
            supports_pdf: false,
        }
    }
}

pub struct PCL6Driver {
    stream_header: Vec<u8>,
}

impl PCL6Driver {
    pub fn new() -> Self {
        Self {
            stream_header: vec![0x29, 0x20, 0x48, 0x50, 0x2D, 0x50, 0x43, 0x4C, 0x20, 0x58, 0x4C],
        }
    }

    fn generate_pclxl(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut pclxl = Vec::new();
        
        pclxl.extend_from_slice(&self.stream_header);
        pclxl.extend_from_slice(b"\x0A");
        
        pclxl.push(0x41);
        pclxl.push(0xC0);
        pclxl.push(0x00);
        
        pclxl.push(0x48);
        pclxl.push(self.get_orientation_value(options.orientation));
        pclxl.push(0xF8);
        pclxl.push(0x26);
        
        pclxl.push(0x48);
        pclxl.push(self.get_media_size_value(options.paper_size));
        pclxl.push(0xF8);
        pclxl.push(0x25);
        
        if options.duplex {
            pclxl.push(0x48);
            pclxl.push(0x01);
            pclxl.push(0xF8);
            pclxl.push(0x34);
        }
        
        pclxl.push(0x43);
        
        let text = String::from_utf8_lossy(data);
        for line in text.lines() {
            let line_bytes = line.as_bytes();
            pclxl.push(0xD5);
            pclxl.push(line_bytes.len() as u8);
            pclxl.push(0x00);
            pclxl.extend_from_slice(line_bytes);
            pclxl.push(0xF8);
            pclxl.push(0xA8);
        }
        
        pclxl.push(0x44);
        pclxl.push(0x49);
        
        pclxl
    }

    fn get_orientation_value(&self, orientation: crate::printing::Orientation) -> u8 {
        match orientation {
            crate::printing::Orientation::Portrait => 0x00,
            crate::printing::Orientation::Landscape => 0x01,
            crate::printing::Orientation::ReversePortrait => 0x02,
            crate::printing::Orientation::ReverseLandscape => 0x03,
        }
    }

    fn get_media_size_value(&self, size: crate::printing::PaperSize) -> u8 {
        match size {
            crate::printing::PaperSize::Letter => 0x02,
            crate::printing::PaperSize::Legal => 0x03,
            crate::printing::PaperSize::A4 => 0x1A,
            crate::printing::PaperSize::A3 => 0x1B,
            crate::printing::PaperSize::A5 => 0x19,
            _ => 0x02,
        }
    }
}

impl PrinterDriver for PCL6Driver {
    fn name(&self) -> &str {
        "PCL6/XL"
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["text/plain", "application/vnd.hp-pclxl", "image/jpeg"]
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        Ok(self.generate_pclxl(data, options))
    }

    fn send_command(&self, command: PrinterCommand) -> Result<(), &'static str> {
        match command {
            PrinterCommand::Reset => Ok(()),
            _ => Ok(()),
        }
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("PCL6/XL Printer"),
            driver: String::from("pcl6"),
            printer_type: crate::printing::PrinterType::Laser,
            color_modes: vec![
                crate::printing::ColorMode::Monochrome,
                crate::printing::ColorMode::Grayscale,
                crate::printing::ColorMode::Color,
            ],
            paper_sizes: vec![
                crate::printing::PaperSize::Letter,
                crate::printing::PaperSize::Legal,
                crate::printing::PaperSize::A4,
                crate::printing::PaperSize::A3,
            ],
            duplex: true,
            stapler: true,
            collate: true,
            max_copies: 999,
            resolution_dpi: vec![(300, 300), (600, 600), (1200, 1200)],
            print_qualities: vec![
                crate::printing::PrintQuality::Draft,
                crate::printing::PrintQuality::Normal,
                crate::printing::PrintQuality::High,
                crate::printing::PrintQuality::Photo,
            ],
            supports_postscript: false,
            supports_pcl: true,
            supports_pdf: false,
        }
    }
}