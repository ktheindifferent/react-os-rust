use alloc::{string::String, vec::Vec};
use super::{PrinterDriver, PrinterCommand};

pub struct ESCPDriver {
    mode: ESCPMode,
    current_font: u8,
    line_spacing: u8,
}

#[derive(Debug, Clone, Copy)]
enum ESCPMode {
    ESCP,
    ESCP2,
}

impl ESCPDriver {
    pub fn new() -> Self {
        Self {
            mode: ESCPMode::ESCP2,
            current_font: 0,
            line_spacing: 60,
        }
    }

    fn generate_escp(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut escp = Vec::new();
        
        escp.extend_from_slice(b"\x1B@");
        
        escp.extend_from_slice(b"\x1BU\x00");
        
        let quality = match options.quality {
            crate::printing::PrintQuality::Draft => b"\x1Bx\x00",
            crate::printing::PrintQuality::Normal => b"\x1Bx\x01",
            _ => b"\x1Bx\x01",
        };
        escp.extend_from_slice(quality);
        
        escp.extend_from_slice(b"\x1B3");
        escp.push(self.line_spacing);
        
        if options.orientation == crate::printing::Orientation::Landscape {
            escp.extend_from_slice(b"\x1B\x0F");
        }
        
        let paper_code = self.get_paper_code(options.paper_size);
        escp.extend_from_slice(b"\x1BC");
        escp.push(paper_code);
        
        let margin = 10;
        escp.extend_from_slice(b"\x1Bl");
        escp.push(margin);
        escp.extend_from_slice(b"\x1BQ");
        escp.push(80 - margin);
        
        if matches!(self.mode, ESCPMode::ESCP2) {
            let resolution = match options.quality {
                crate::printing::PrintQuality::Draft => 180,
                crate::printing::PrintQuality::Normal => 360,
                crate::printing::PrintQuality::High => 720,
                crate::printing::PrintQuality::Photo => 1440,
            };
            
            escp.extend_from_slice(b"\x1B(U\x05\x00");
            escp.push(resolution as u8);
            escp.push((resolution >> 8) as u8);
            escp.push(resolution as u8);
            escp.push((resolution >> 8) as u8);
            escp.push(0x01);
        }
        
        escp.extend_from_slice(b"\x1Bk");
        escp.push(self.current_font);
        
        let text = String::from_utf8_lossy(data);
        for line in text.lines() {
            escp.extend_from_slice(line.as_bytes());
            escp.extend_from_slice(b"\r\n");
        }
        
        escp.push(0x0C);
        
        escp.extend_from_slice(b"\x1B@");
        
        escp
    }

    fn get_paper_code(&self, size: crate::printing::PaperSize) -> u8 {
        match size {
            crate::printing::PaperSize::Letter => 22,
            crate::printing::PaperSize::Legal => 32,
            crate::printing::PaperSize::A4 => 22,
            crate::printing::PaperSize::A3 => 30,
            crate::printing::PaperSize::A5 => 15,
            _ => 22,
        }
    }

    fn set_color_mode(&self, mode: crate::printing::ColorMode) -> Vec<u8> {
        let mut cmds = Vec::new();
        
        if matches!(self.mode, ESCPMode::ESCP2) {
            match mode {
                crate::printing::ColorMode::Color => {
                    cmds.extend_from_slice(b"\x1B(c\x04\x00\x00\x00\x01\x04");
                }
                crate::printing::ColorMode::Monochrome => {
                    cmds.extend_from_slice(b"\x1B(c\x04\x00\x00\x00\x01\x01");
                }
                _ => {}
            }
        }
        
        cmds
    }

    fn set_ink_type(&self, cmyk: bool) -> Vec<u8> {
        let mut cmds = Vec::new();
        
        if matches!(self.mode, ESCPMode::ESCP2) {
            if cmyk {
                cmds.extend_from_slice(b"\x1B(i\x01\x00\x01");
            } else {
                cmds.extend_from_slice(b"\x1B(i\x01\x00\x00");
            }
        }
        
        cmds
    }

    fn print_graphics(&self, width: u16, height: u16, data: &[u8]) -> Vec<u8> {
        let mut cmds = Vec::new();
        
        cmds.extend_from_slice(b"\x1B*");
        cmds.push(39);
        
        cmds.push((width & 0xFF) as u8);
        cmds.push((width >> 8) as u8);
        
        for row in 0..height {
            let row_start = (row as usize) * (width as usize / 8);
            let row_end = row_start + (width as usize / 8);
            
            if row_end <= data.len() {
                cmds.extend_from_slice(&data[row_start..row_end]);
            }
            
            cmds.extend_from_slice(b"\r\n");
        }
        
        cmds
    }
}

impl PrinterDriver for ESCPDriver {
    fn name(&self) -> &str {
        "ESC/P"
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["text/plain", "image/bmp"]
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        Ok(self.generate_escp(data, options))
    }

    fn send_command(&self, command: PrinterCommand) -> Result<(), &'static str> {
        match command {
            PrinterCommand::Reset => Ok(()),
            PrinterCommand::FormFeed => Ok(()),
            PrinterCommand::LineFeed => Ok(()),
            PrinterCommand::CleanPrintHead => Ok(()),
            PrinterCommand::AlignPrintHead => Ok(()),
            _ => Ok(()),
        }
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("ESC/P Printer"),
            driver: String::from("escp"),
            printer_type: crate::printing::PrinterType::Inkjet,
            color_modes: vec![
                crate::printing::ColorMode::Monochrome,
                crate::printing::ColorMode::Color,
            ],
            paper_sizes: vec![
                crate::printing::PaperSize::Letter,
                crate::printing::PaperSize::Legal,
                crate::printing::PaperSize::A4,
                crate::printing::PaperSize::A3,
                crate::printing::PaperSize::A5,
            ],
            duplex: false,
            stapler: false,
            collate: true,
            max_copies: 99,
            resolution_dpi: vec![(180, 180), (360, 360), (720, 720), (1440, 720)],
            print_qualities: vec![
                crate::printing::PrintQuality::Draft,
                crate::printing::PrintQuality::Normal,
                crate::printing::PrintQuality::High,
                crate::printing::PrintQuality::Photo,
            ],
            supports_postscript: false,
            supports_pcl: false,
            supports_pdf: false,
        }
    }
}