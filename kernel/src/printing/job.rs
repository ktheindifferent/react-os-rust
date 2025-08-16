use alloc::{string::String, vec::Vec};
use crate::fs::File;
use super::PrintOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Processing,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobPriority {
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone)]
pub struct PrintJob {
    pub id: u32,
    pub printer_id: u32,
    pub user: String,
    pub title: String,
    pub file: File,
    pub options: PrintOptions,
    pub status: JobStatus,
    pub priority: JobPriority,
    pub total_pages: u32,
    pub pages_printed: u32,
    pub size_bytes: u64,
    pub queued_time: u64,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub error_message: Option<String>,
    pub secure_pin: Option<String>,
}

impl PrintJob {
    pub fn new(id: u32, printer_id: u32, file: File, options: PrintOptions) -> Self {
        let total_pages = Self::calculate_pages(&file, &options);
        let size_bytes = file.size();
        
        Self {
            id,
            printer_id,
            user: String::from("user"),
            title: file.name.clone(),
            file,
            options: options.clone(),
            status: JobStatus::Queued,
            priority: JobPriority::Normal,
            total_pages,
            pages_printed: 0,
            size_bytes,
            queued_time: 0,
            start_time: None,
            end_time: None,
            error_message: None,
            secure_pin: options.secure_pin,
        }
    }

    fn calculate_pages(file: &File, options: &PrintOptions) -> u32 {
        let base_pages = file.size() / 4096 + 1;
        
        let pages = match &options.page_range {
            Some(super::PageRange::Range(start, end)) => end - start + 1,
            Some(super::PageRange::Pages(pages)) => pages.len() as u32,
            _ => base_pages as u32,
        };
        
        if options.n_up > 1 {
            (pages + options.n_up - 1) / options.n_up
        } else {
            pages
        }
    }

    pub fn estimated_print_time(&self) -> u64 {
        let base_time = 2000;
        let per_page = 500;
        base_time + (self.total_pages as u64 * per_page)
    }

    pub fn is_secure(&self) -> bool {
        self.secure_pin.is_some()
    }

    pub fn verify_pin(&self, pin: &str) -> bool {
        match &self.secure_pin {
            Some(stored_pin) => stored_pin == pin,
            None => true,
        }
    }

    pub fn get_duration(&self) -> Option<u64> {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }

    pub fn set_priority(&mut self, priority: JobPriority) {
        self.priority = priority;
    }

    pub fn update_progress(&mut self, pages_printed: u32) {
        self.pages_printed = pages_printed.min(self.total_pages);
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled)
    }

    pub fn can_cancel(&self) -> bool {
        !self.is_completed()
    }

    pub fn can_pause(&self) -> bool {
        matches!(self.status, JobStatus::Queued | JobStatus::Processing)
    }

    pub fn can_resume(&self) -> bool {
        self.status == JobStatus::Paused
    }
}