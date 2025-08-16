use alloc::{string::String, vec::Vec};
use super::{PrinterDriver, PrinterCommand};

pub struct TextDriver {
    chars_per_line: usize,
    lines_per_page: usize,
}

impl TextDriver {
    pub fn new() -> Self {
        Self {
            chars_per_line: 80,
            lines_per_page: 66,
        }
    }

    fn format_text(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut output = Vec::new();
        let text = String::from_utf8_lossy(data);
        
        let chars_per_line = if options.orientation == crate::printing::Orientation::Landscape {
            132
        } else {
            self.chars_per_line
        };
        
        let mut current_line = 0;
        let mut current_page = 1;
        
        for line in text.lines() {
            if current_line >= self.lines_per_page {
                output.push(0x0C);
                current_line = 0;
                current_page += 1;
            }
            
            if line.len() > chars_per_line {
                let mut pos = 0;
                while pos < line.len() {
                    let end = (pos + chars_per_line).min(line.len());
                    output.extend_from_slice(line[pos..end].as_bytes());
                    output.extend_from_slice(b"\r\n");
                    current_line += 1;
                    pos = end;
                }
            } else {
                output.extend_from_slice(line.as_bytes());
                output.extend_from_slice(b"\r\n");
                current_line += 1;
            }
        }
        
        if current_line > 0 {
            output.push(0x0C);
        }
        
        output
    }
}

impl PrinterDriver for TextDriver {
    fn name(&self) -> &str {
        "Generic Text"
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["text/plain"]
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        Ok(self.format_text(data, options))
    }

    fn send_command(&self, _command: PrinterCommand) -> Result<(), &'static str> {
        Ok(())
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("Generic Text Printer"),
            driver: String::from("text"),
            printer_type: crate::printing::PrinterType::DotMatrix,
            color_modes: vec![crate::printing::ColorMode::Monochrome],
            paper_sizes: vec![
                crate::printing::PaperSize::Letter,
                crate::printing::PaperSize::Legal,
                crate::printing::PaperSize::A4,
            ],
            duplex: false,
            stapler: false,
            collate: false,
            max_copies: 1,
            resolution_dpi: vec![(72, 72)],
            print_qualities: vec![crate::printing::PrintQuality::Normal],
            supports_postscript: false,
            supports_pcl: false,
            supports_pdf: false,
        }
    }
}