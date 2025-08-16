use alloc::{vec::Vec, string::String};
use super::{Scanner, ScanSettings};

pub struct TWAINBackend {
    initialized: bool,
}

impl TWAINBackend {
    pub fn new() -> Self {
        Self {
            initialized: false,
        }
    }

    pub fn init(&mut self) -> Result<(), &'static str> {
        self.initialized = true;
        Ok(())
    }

    pub fn get_devices(&self) -> Result<Vec<Scanner>, &'static str> {
        Ok(Vec::new())
    }
}