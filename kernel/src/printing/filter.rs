use alloc::{vec::Vec, string::String, boxed::Box};
use super::job::PrintJob;

pub trait PrintFilter: Send + Sync {
    fn name(&self) -> &str;
    fn input_format(&self) -> &str;
    fn output_format(&self) -> &str;
    fn process(&self, input: &[u8], job: &PrintJob) -> Result<Vec<u8>, &'static str>;
}

pub struct FilterChain {
    filters: Vec<Box<dyn PrintFilter>>,
}

impl FilterChain {
    pub fn new() -> Self {
        let mut chain = Self {
            filters: Vec::new(),
        };
        chain.register_builtin_filters();
        chain
    }

    fn register_builtin_filters(&mut self) {
        self.filters.push(Box::new(TextToPostScriptFilter::new()));
        self.filters.push(Box::new(PostScriptToPDFFilter::new()));
        self.filters.push(Box::new(ImageRasterFilter::new()));
        self.filters.push(Box::new(PDFRasterFilter::new()));
        self.filters.push(Box::new(NUpFilter::new()));
        self.filters.push(Box::new(WatermarkFilter::new()));
    }

    pub fn process(&self, job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        let mut data = Vec::new();
        
        if let Ok(content) = job.file.read_all() {
            data = content;
        } else {
            return Err("Failed to read file");
        }
        
        let input_format = self.detect_format(&data);
        let target_format = self.get_target_format(job);
        
        if input_format != target_format {
            if let Some(filter) = self.find_filter(input_format, target_format) {
                data = filter.process(&data, job)?;
            }
        }
        
        if job.options.n_up > 1 {
            if let Some(filter) = self.filters.iter().find(|f| f.name() == "n-up") {
                data = filter.process(&data, job)?;
            }
        }
        
        if job.options.watermark.is_some() {
            if let Some(filter) = self.filters.iter().find(|f| f.name() == "watermark") {
                data = filter.process(&data, job)?;
            }
        }
        
        Ok(data)
    }

    fn detect_format(&self, data: &[u8]) -> &'static str {
        if data.starts_with(b"%PDF") {
            "application/pdf"
        } else if data.starts_with(b"%!PS") {
            "application/postscript"
        } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
            "image/jpeg"
        } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
            "image/png"
        } else {
            "text/plain"
        }
    }

    fn get_target_format(&self, job: &PrintJob) -> &'static str {
        "application/postscript"
    }

    fn find_filter(&self, input: &'static str, output: &'static str) -> Option<&Box<dyn PrintFilter>> {
        self.filters.iter().find(|f| f.input_format() == input && f.output_format() == output)
    }
}

pub struct TextToPostScriptFilter;

impl TextToPostScriptFilter {
    pub fn new() -> Self {
        Self
    }
}

impl PrintFilter for TextToPostScriptFilter {
    fn name(&self) -> &str {
        "text-to-ps"
    }

    fn input_format(&self) -> &str {
        "text/plain"
    }

    fn output_format(&self) -> &str {
        "application/postscript"
    }

    fn process(&self, input: &[u8], _job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        let text = String::from_utf8_lossy(input);
        let mut ps = String::new();
        
        ps.push_str("%!PS-Adobe-3.0\n");
        ps.push_str("%%Pages: 1\n");
        ps.push_str("%%EndComments\n\n");
        ps.push_str("/Courier findfont 10 scalefont setfont\n");
        ps.push_str("72 720 moveto\n");
        
        for line in text.lines() {
            ps.push_str(&format!("({}) show\n", line));
            ps.push_str("72 currentpoint exch pop 12 sub moveto\n");
        }
        
        ps.push_str("showpage\n");
        ps.push_str("%%EOF\n");
        
        Ok(ps.into_bytes())
    }
}

pub struct PostScriptToPDFFilter;

impl PostScriptToPDFFilter {
    pub fn new() -> Self {
        Self
    }
}

impl PrintFilter for PostScriptToPDFFilter {
    fn name(&self) -> &str {
        "ps-to-pdf"
    }

    fn input_format(&self) -> &str {
        "application/postscript"
    }

    fn output_format(&self) -> &str {
        "application/pdf"
    }

    fn process(&self, input: &[u8], _job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        Ok(input.to_vec())
    }
}

pub struct ImageRasterFilter;

impl ImageRasterFilter {
    pub fn new() -> Self {
        Self
    }
}

impl PrintFilter for ImageRasterFilter {
    fn name(&self) -> &str {
        "image-raster"
    }

    fn input_format(&self) -> &str {
        "image/*"
    }

    fn output_format(&self) -> &str {
        "application/vnd.cups-raster"
    }

    fn process(&self, input: &[u8], _job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        Ok(input.to_vec())
    }
}

pub struct PDFRasterFilter;

impl PDFRasterFilter {
    pub fn new() -> Self {
        Self
    }
}

impl PrintFilter for PDFRasterFilter {
    fn name(&self) -> &str {
        "pdf-raster"
    }

    fn input_format(&self) -> &str {
        "application/pdf"
    }

    fn output_format(&self) -> &str {
        "application/vnd.cups-raster"
    }

    fn process(&self, input: &[u8], _job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        Ok(input.to_vec())
    }
}

pub struct NUpFilter;

impl NUpFilter {
    pub fn new() -> Self {
        Self
    }
}

impl PrintFilter for NUpFilter {
    fn name(&self) -> &str {
        "n-up"
    }

    fn input_format(&self) -> &str {
        "*"
    }

    fn output_format(&self) -> &str {
        "*"
    }

    fn process(&self, input: &[u8], job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        if job.options.n_up <= 1 {
            return Ok(input.to_vec());
        }
        
        Ok(input.to_vec())
    }
}

pub struct WatermarkFilter;

impl WatermarkFilter {
    pub fn new() -> Self {
        Self
    }
}

impl PrintFilter for WatermarkFilter {
    fn name(&self) -> &str {
        "watermark"
    }

    fn input_format(&self) -> &str {
        "*"
    }

    fn output_format(&self) -> &str {
        "*"
    }

    fn process(&self, input: &[u8], job: &PrintJob) -> Result<Vec<u8>, &'static str> {
        if job.options.watermark.is_none() {
            return Ok(input.to_vec());
        }
        
        Ok(input.to_vec())
    }
}