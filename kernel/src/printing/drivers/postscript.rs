use alloc::{string::{String, ToString}, vec::Vec, format};
use super::{PrinterDriver, PrinterCommand};

pub struct PostScriptDriver {
    version: String,
    current_font: String,
    current_size: f32,
    current_color: (f32, f32, f32),
    page_count: u32,
}

impl PostScriptDriver {
    pub fn new() -> Self {
        Self {
            version: String::from("3.0"),
            current_font: String::from("Helvetica"),
            current_size: 12.0,
            current_color: (0.0, 0.0, 0.0),
            page_count: 0,
        }
    }

    fn generate_header(&self) -> String {
        format!(
            "%!PS-Adobe-{}\n\
            %%Creator: OS Print Subsystem\n\
            %%Pages: (atend)\n\
            %%BoundingBox: 0 0 612 792\n\
            %%EndComments\n\n",
            self.version
        )
    }

    fn generate_prolog(&self) -> String {
        String::from(
            "/inch {72 mul} def\n\
            /cm {28.35 mul} def\n\
            /mm {2.835 mul} def\n\
            /pica {12 mul} def\n\
            /point {1 mul} def\n\n\
            /setFont {\n\
            /fontName exch def\n\
            /fontSize exch def\n\
            fontName findfont fontSize scalefont setfont\n\
            } def\n\n\
            /centerText {\n\
            /str exch def\n\
            /y exch def\n\
            /x exch def\n\
            gsave\n\
            x y moveto\n\
            str stringwidth pop 2 div neg 0 rmoveto\n\
            str show\n\
            grestore\n\
            } def\n\n"
        )
    }

    fn generate_page_setup(&self, page_num: u32, options: &crate::printing::PrintOptions) -> String {
        let (width, height) = self.get_page_dimensions(options.paper_size);
        
        format!(
            "%%Page: {} {}\n\
            gsave\n\
            {} {} translate\n\
            {} rotate\n",
            page_num, page_num,
            if options.orientation == crate::printing::Orientation::Landscape { height } else { 0 },
            if options.orientation == crate::printing::Orientation::Landscape { 0 } else { 0 },
            if options.orientation == crate::printing::Orientation::Landscape { 90 } else { 0 }
        )
    }

    fn get_page_dimensions(&self, size: crate::printing::PaperSize) -> (f32, f32) {
        match size {
            crate::printing::PaperSize::Letter => (612.0, 792.0),
            crate::printing::PaperSize::Legal => (612.0, 1008.0),
            crate::printing::PaperSize::A4 => (595.0, 842.0),
            crate::printing::PaperSize::A3 => (842.0, 1191.0),
            crate::printing::PaperSize::A5 => (420.0, 595.0),
            crate::printing::PaperSize::Envelope => (297.0, 684.0),
            crate::printing::PaperSize::Custom(w, h) => (w as f32, h as f32),
        }
    }

    fn convert_to_postscript(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Vec<u8> {
        let mut ps = String::new();
        
        ps.push_str(&self.generate_header());
        ps.push_str(&self.generate_prolog());
        
        ps.push_str(&self.generate_page_setup(1, options));
        
        ps.push_str(&format!("{} {} setFont\n", self.current_size, self.current_font));
        ps.push_str(&format!("{} {} {} setrgbcolor\n", 
            self.current_color.0, self.current_color.1, self.current_color.2));
        
        let text = String::from_utf8_lossy(data);
        let lines: Vec<&str> = text.lines().collect();
        let mut y = 720.0;
        let line_height = self.current_size * 1.2;
        
        for line in lines {
            if y < 72.0 {
                ps.push_str("grestore\nshowpage\n");
                ps.push_str(&self.generate_page_setup(self.page_count + 2, options));
                y = 720.0;
            }
            
            ps.push_str(&format!("72 {} moveto\n", y));
            ps.push_str(&format!("({}) show\n", self.escape_string(line)));
            y -= line_height;
        }
        
        ps.push_str("grestore\nshowpage\n");
        ps.push_str("%%Trailer\n");
        ps.push_str(&format!("%%Pages: {}\n", self.page_count + 1));
        ps.push_str("%%EOF\n");
        
        ps.into_bytes()
    }

    fn escape_string(&self, s: &str) -> String {
        s.chars()
            .map(|c| match c {
                '(' => "\\(".to_string(),
                ')' => "\\)".to_string(),
                '\\' => "\\\\".to_string(),
                '\n' => "\\n".to_string(),
                '\r' => "\\r".to_string(),
                '\t' => "\\t".to_string(),
                _ => c.to_string(),
            })
            .collect()
    }

    fn render_image(&self, image_data: &[u8], width: u32, height: u32, x: f32, y: f32) -> String {
        format!(
            "gsave\n\
            {} {} translate\n\
            {} {} scale\n\
            /DeviceRGB setcolorspace\n\
            << /ImageType 1\n\
               /Width {}\n\
               /Height {}\n\
               /BitsPerComponent 8\n\
               /Decode [0 1 0 1 0 1]\n\
               /ImageMatrix [{} 0 0 {} 0 0]\n\
               /DataSource currentfile /ASCIIHexDecode filter\n\
            >> image\n",
            x, y, width as f32, height as f32,
            width, height, width, -(height as i32)
        )
    }
}

impl PrinterDriver for PostScriptDriver {
    fn name(&self) -> &str {
        "PostScript"
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["text/plain", "application/postscript", "image/jpeg", "image/png"]
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        Ok(self.convert_to_postscript(data, options))
    }

    fn send_command(&self, command: PrinterCommand) -> Result<(), &'static str> {
        match command {
            PrinterCommand::Reset => Ok(()),
            PrinterCommand::FormFeed => Ok(()),
            PrinterCommand::LineFeed => Ok(()),
            _ => Ok(()),
        }
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("PostScript Printer"),
            driver: String::from("postscript"),
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
            stapler: false,
            collate: true,
            max_copies: 999,
            resolution_dpi: vec![(300, 300), (600, 600), (1200, 1200)],
            print_qualities: vec![
                crate::printing::PrintQuality::Draft,
                crate::printing::PrintQuality::Normal,
                crate::printing::PrintQuality::High,
            ],
            supports_postscript: true,
            supports_pcl: false,
            supports_pdf: false,
        }
    }
}