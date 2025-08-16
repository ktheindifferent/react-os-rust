use alloc::{string::String, vec::Vec};
use super::{PrinterDriver, PrinterCommand};

pub struct ImageDriver {
    supported_formats: Vec<String>,
}

impl ImageDriver {
    pub fn new() -> Self {
        Self {
            supported_formats: vec![
                String::from("image/png"),
                String::from("image/jpeg"),
                String::from("image/bmp"),
                String::from("image/tiff"),
            ],
        }
    }

    fn render_image(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut output = Vec::new();
        
        output.extend_from_slice(b"\x1B*");
        output.push(3);
        
        let dpi = options.resolution.0;
        output.push((dpi & 0xFF) as u8);
        output.push((dpi >> 8) as u8);
        
        output.extend_from_slice(data);
        
        output.push(0x0C);
        
        output
    }
}

impl PrinterDriver for ImageDriver {
    fn name(&self) -> &str {
        "Image Printer"
    }

    fn supported_formats(&self) -> Vec<&str> {
        self.supported_formats.iter().map(|s| s.as_str()).collect()
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        Ok(self.render_image(data, options))
    }

    fn send_command(&self, _command: PrinterCommand) -> Result<(), &'static str> {
        Ok(())
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("Image Printer"),
            driver: String::from("image"),
            printer_type: crate::printing::PrinterType::Inkjet,
            color_modes: vec![
                crate::printing::ColorMode::Monochrome,
                crate::printing::ColorMode::Grayscale,
                crate::printing::ColorMode::Color,
                crate::printing::ColorMode::CMYK,
            ],
            paper_sizes: vec![
                crate::printing::PaperSize::Letter,
                crate::printing::PaperSize::A4,
                crate::printing::PaperSize::Custom(0, 0),
            ],
            duplex: false,
            stapler: false,
            collate: false,
            max_copies: 99,
            resolution_dpi: vec![(300, 300), (600, 600), (1200, 1200), (2400, 2400)],
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