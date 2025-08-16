use alloc::{string::String, vec::Vec, format};
use super::{PrinterDriver, PrinterCommand};

pub struct LabelDriver {
    label_width: u32,
    label_height: u32,
    language: LabelLanguage,
}

#[derive(Debug, Clone, Copy)]
enum LabelLanguage {
    ZPL,
    EPL,
    TSPL,
}

impl LabelDriver {
    pub fn new() -> Self {
        Self {
            label_width: 4,
            label_height: 6,
            language: LabelLanguage::ZPL,
        }
    }

    fn generate_zpl(&self, data: &[u8], _options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut zpl = Vec::new();
        
        zpl.extend_from_slice(b"^XA\n");
        
        zpl.extend_from_slice(b"^MD10\n");
        
        zpl.extend_from_slice(b"^PW812\n");
        
        zpl.extend_from_slice(b"^LL1218\n");
        
        zpl.extend_from_slice(b"^FO50,50\n");
        zpl.extend_from_slice(b"^A0N,50,50\n");
        
        let text = String::from_utf8_lossy(data);
        for (i, line) in text.lines().enumerate() {
            let y = 50 + (i as u32 * 60);
            zpl.extend_from_slice(format!("^FO50,{}\n", y).as_bytes());
            zpl.extend_from_slice(b"^A0N,30,30\n");
            zpl.extend_from_slice(format!("^FD{}^FS\n", line).as_bytes());
        }
        
        zpl.extend_from_slice(b"^XZ\n");
        
        zpl
    }

    fn generate_epl(&self, data: &[u8], _options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut epl = Vec::new();
        
        epl.extend_from_slice(b"N\n");
        
        epl.extend_from_slice(b"q816\n");
        epl.extend_from_slice(b"Q1218,24\n");
        
        let text = String::from_utf8_lossy(data);
        for (i, line) in text.lines().enumerate() {
            let y = 50 + (i as u32 * 50);
            epl.extend_from_slice(format!("A50,{},0,3,1,1,N,\"{}\"\n", y, line).as_bytes());
        }
        
        epl.extend_from_slice(b"P1\n");
        
        epl
    }

    fn generate_barcode_zpl(&self, barcode_type: &str, data: &str, x: u32, y: u32) -> Vec<u8> {
        let mut zpl = Vec::new();
        
        zpl.extend_from_slice(format!("^FO{},{}\n", x, y).as_bytes());
        
        match barcode_type {
            "CODE128" => {
                zpl.extend_from_slice(b"^BCN,100,Y,N,N\n");
            }
            "QR" => {
                zpl.extend_from_slice(b"^BQN,2,10\n");
            }
            "CODE39" => {
                zpl.extend_from_slice(b"^B3N,N,100,Y,N\n");
            }
            "EAN13" => {
                zpl.extend_from_slice(b"^BEN,100,Y,N\n");
            }
            _ => {
                zpl.extend_from_slice(b"^BCN,100,Y,N,N\n");
            }
        }
        
        zpl.extend_from_slice(format!("^FD{}^FS\n", data).as_bytes());
        
        zpl
    }
}

impl PrinterDriver for LabelDriver {
    fn name(&self) -> &str {
        "Label Printer"
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["text/plain", "application/zpl", "application/epl"]
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        match self.language {
            LabelLanguage::ZPL => Ok(self.generate_zpl(data, options)),
            LabelLanguage::EPL => Ok(self.generate_epl(data, options)),
            _ => Ok(self.generate_zpl(data, options)),
        }
    }

    fn send_command(&self, _command: PrinterCommand) -> Result<(), &'static str> {
        Ok(())
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("Label Printer"),
            driver: String::from("label"),
            printer_type: crate::printing::PrinterType::Label,
            color_modes: vec![crate::printing::ColorMode::Monochrome],
            paper_sizes: vec![
                crate::printing::PaperSize::Custom(4, 6),
                crate::printing::PaperSize::Custom(4, 4),
                crate::printing::PaperSize::Custom(2, 1),
            ],
            duplex: false,
            stapler: false,
            collate: false,
            max_copies: 999,
            resolution_dpi: vec![(203, 203), (300, 300), (600, 600)],
            print_qualities: vec![
                crate::printing::PrintQuality::Normal,
                crate::printing::PrintQuality::High,
            ],
            supports_postscript: false,
            supports_pcl: false,
            supports_pdf: false,
        }
    }
}