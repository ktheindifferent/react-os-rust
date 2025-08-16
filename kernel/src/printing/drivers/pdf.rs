use alloc::{string::{String, ToString}, vec::Vec, format, collections::BTreeMap};
use super::{PrinterDriver, PrinterCommand};

pub struct PDFDriver {
    version: String,
    objects: Vec<PDFObject>,
    current_obj_id: u32,
}

#[derive(Clone)]
struct PDFObject {
    id: u32,
    generation: u32,
    content: Vec<u8>,
    offset: usize,
}

impl PDFDriver {
    pub fn new() -> Self {
        Self {
            version: String::from("1.7"),
            objects: Vec::new(),
            current_obj_id: 0,
        }
    }

    fn generate_pdf(&mut self, data: &[u8], options: &crate::printing::PrintOptions) -> Vec<u8> {
        self.objects.clear();
        self.current_obj_id = 0;
        
        let catalog_id = self.add_catalog();
        let pages_id = self.add_pages();
        let page_id = self.add_page(pages_id, options);
        let font_id = self.add_font();
        let content_id = self.add_content(data, font_id, options);
        
        self.update_page_content(page_id, content_id);
        self.update_pages_kids(pages_id, vec![page_id]);
        
        self.build_pdf(catalog_id)
    }

    fn next_obj_id(&mut self) -> u32 {
        self.current_obj_id += 1;
        self.current_obj_id
    }

    fn add_object(&mut self, content: Vec<u8>) -> u32 {
        let id = self.next_obj_id();
        self.objects.push(PDFObject {
            id,
            generation: 0,
            content,
            offset: 0,
        });
        id
    }

    fn add_catalog(&mut self) -> u32 {
        let content = format!(
            "<< /Type /Catalog /Pages {} 0 R >>",
            self.current_obj_id + 2
        ).into_bytes();
        self.add_object(content)
    }

    fn add_pages(&mut self) -> u32 {
        let content = b"<< /Type /Pages /Kids [] /Count 0 >>".to_vec();
        self.add_object(content)
    }

    fn add_page(&mut self, parent_id: u32, options: &crate::printing::PrintOptions) -> u32 {
        let (width, height) = self.get_page_size(options.paper_size);
        let media_box = if options.orientation == crate::printing::Orientation::Landscape {
            format!("[0 0 {} {}]", height, width)
        } else {
            format!("[0 0 {} {}]", width, height)
        };
        
        let content = format!(
            "<< /Type /Page /Parent {} 0 R /MediaBox {} /Resources << /Font << >> >> /Contents {} 0 R >>",
            parent_id, media_box, self.current_obj_id + 1
        ).into_bytes();
        self.add_object(content)
    }

    fn add_font(&mut self) -> u32 {
        let content = b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec();
        self.add_object(content)
    }

    fn add_content(&mut self, data: &[u8], font_id: u32, options: &crate::printing::PrintOptions) -> u32 {
        let text = String::from_utf8_lossy(data);
        let mut stream = Vec::new();
        
        stream.extend_from_slice(b"BT\n");
        stream.extend_from_slice(format!("/F{} 12 Tf\n", font_id).as_bytes());
        stream.extend_from_slice(b"72 720 Td\n");
        stream.extend_from_slice(b"14 TL\n");
        
        for line in text.lines() {
            stream.extend_from_slice(b"(");
            stream.extend_from_slice(self.escape_pdf_string(line).as_bytes());
            stream.extend_from_slice(b") Tj\n");
            stream.extend_from_slice(b"T*\n");
        }
        
        stream.extend_from_slice(b"ET\n");
        
        let compressed = self.compress_stream(&stream);
        let content = format!(
            "<< /Length {} /Filter /FlateDecode >>\nstream\n{}\nendstream",
            compressed.len(),
            String::from_utf8_lossy(&compressed)
        ).into_bytes();
        
        self.add_object(content)
    }

    fn update_page_content(&mut self, page_id: u32, content_id: u32) {
        if let Some(page) = self.objects.iter_mut().find(|obj| obj.id == page_id) {
            let content_str = String::from_utf8_lossy(&page.content);
            let updated = content_str.replace(
                "/Contents 0 0 R",
                &format!("/Contents {} 0 R", content_id)
            );
            page.content = updated.into_bytes();
        }
    }

    fn update_pages_kids(&mut self, pages_id: u32, kids: Vec<u32>) {
        if let Some(pages) = self.objects.iter_mut().find(|obj| obj.id == pages_id) {
            let kids_str = kids.iter()
                .map(|id| format!("{} 0 R", id))
                .collect::<Vec<_>>()
                .join(" ");
            
            let content = format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                kids_str,
                kids.len()
            ).into_bytes();
            pages.content = content;
        }
    }

    fn get_page_size(&self, size: crate::printing::PaperSize) -> (f32, f32) {
        match size {
            crate::printing::PaperSize::Letter => (612.0, 792.0),
            crate::printing::PaperSize::Legal => (612.0, 1008.0),
            crate::printing::PaperSize::A4 => (595.28, 841.89),
            crate::printing::PaperSize::A3 => (841.89, 1190.55),
            crate::printing::PaperSize::A5 => (420.94, 595.28),
            crate::printing::PaperSize::Custom(w, h) => (w as f32, h as f32),
            _ => (612.0, 792.0),
        }
    }

    fn escape_pdf_string(&self, s: &str) -> String {
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

    fn compress_stream(&self, data: &[u8]) -> Vec<u8> {
        data.to_vec()
    }

    fn build_pdf(&mut self, catalog_id: u32) -> Vec<u8> {
        let mut pdf = Vec::new();
        
        pdf.extend_from_slice(format!("%PDF-{}\n", self.version).as_bytes());
        pdf.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
        
        let mut xref_offsets = Vec::new();
        
        for obj in &mut self.objects {
            obj.offset = pdf.len();
            xref_offsets.push(obj.offset);
            
            pdf.extend_from_slice(format!("{} {} obj\n", obj.id, obj.generation).as_bytes());
            pdf.extend_from_slice(&obj.content);
            pdf.extend_from_slice(b"\nendobj\n");
        }
        
        let xref_offset = pdf.len();
        
        pdf.extend_from_slice(b"xref\n");
        pdf.extend_from_slice(format!("0 {}\n", self.objects.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        
        for offset in xref_offsets {
            pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
        }
        
        pdf.extend_from_slice(b"trailer\n");
        pdf.extend_from_slice(format!(
            "<< /Size {} /Root {} 0 R >>\n",
            self.objects.len() + 1,
            catalog_id
        ).as_bytes());
        pdf.extend_from_slice(b"startxref\n");
        pdf.extend_from_slice(format!("{}\n", xref_offset).as_bytes());
        pdf.extend_from_slice(b"%%EOF\n");
        
        pdf
    }
}

impl PrinterDriver for PDFDriver {
    fn name(&self) -> &str {
        "PDF"
    }

    fn supported_formats(&self) -> Vec<&str> {
        vec!["text/plain", "text/html", "application/postscript"]
    }

    fn init(&mut self) -> Result<(), &'static str> {
        Ok(())
    }

    fn render(&self, data: &[u8], options: &crate::printing::PrintOptions) -> Result<Vec<u8>, &'static str> {
        let mut driver = Self::new();
        Ok(driver.generate_pdf(data, options))
    }

    fn send_command(&self, _command: PrinterCommand) -> Result<(), &'static str> {
        Ok(())
    }

    fn get_status(&self) -> Result<crate::printing::PrinterStatus, &'static str> {
        Ok(crate::printing::PrinterStatus::Idle)
    }

    fn get_capabilities(&self) -> crate::printing::PrinterCapabilities {
        crate::printing::PrinterCapabilities {
            name: String::from("PDF Printer"),
            driver: String::from("pdf"),
            printer_type: crate::printing::PrinterType::Virtual,
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
                crate::printing::PaperSize::A5,
            ],
            duplex: true,
            stapler: false,
            collate: true,
            max_copies: 1,
            resolution_dpi: vec![(72, 72), (150, 150), (300, 300), (600, 600)],
            print_qualities: vec![
                crate::printing::PrintQuality::Draft,
                crate::printing::PrintQuality::Normal,
                crate::printing::PrintQuality::High,
            ],
            supports_postscript: false,
            supports_pcl: false,
            supports_pdf: true,
        }
    }
}