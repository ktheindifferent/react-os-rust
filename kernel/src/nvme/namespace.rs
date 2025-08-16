// NVMe Namespace Management
use super::*;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

impl NvmeNamespace {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            size: 0,
            block_size: 512,
            capacity: 0,
            features: 0,
        }
    }
    
    pub fn update_from_identify(&mut self, identify: &NvmeIdentifyNamespace) {
        self.size = identify.nsze;
        self.features = identify.nsfeat;
        
        // Get LBA format
        let lba_format = identify.lbaf[(identify.flbas & 0x0F) as usize];
        self.block_size = 1u32 << lba_format.lbads;
        self.capacity = self.size * self.block_size as u64;
    }
    
    pub fn get_optimal_io_size(&self) -> u32 {
        // Return optimal I/O size based on namespace characteristics
        // Default to 128KB for good performance
        128 * 1024
    }
    
    pub fn supports_deallocate(&self) -> bool {
        // Check if namespace supports TRIM/deallocate
        (self.features & 0x01) != 0
    }
    
    pub fn supports_write_zeroes(&self) -> bool {
        // Check if namespace supports write zeroes command
        (self.features & 0x08) != 0
    }
    
    pub fn get_info_string(&self) -> String {
        format!("Namespace {}: {} blocks x {} bytes = {} MB",
                self.id,
                self.size,
                self.block_size,
                self.capacity / (1024 * 1024))
    }
}

// Namespace utilities
pub struct NvmeNamespaceManager {
    namespaces: Vec<NvmeNamespace>,
}

impl NvmeNamespaceManager {
    pub fn new() -> Self {
        Self {
            namespaces: Vec::new(),
        }
    }
    
    pub fn add_namespace(&mut self, ns: NvmeNamespace) {
        self.namespaces.push(ns);
    }
    
    pub fn get_namespace(&self, id: u32) -> Option<&NvmeNamespace> {
        self.namespaces.iter().find(|ns| ns.id == id)
    }
    
    pub fn get_namespace_mut(&mut self, id: u32) -> Option<&mut NvmeNamespace> {
        self.namespaces.iter_mut().find(|ns| ns.id == id)
    }
    
    pub fn list_namespaces(&self) -> Vec<u32> {
        self.namespaces.iter().map(|ns| ns.id).collect()
    }
    
    pub fn total_capacity(&self) -> u64 {
        self.namespaces.iter().map(|ns| ns.capacity).sum()
    }
    
    pub fn active_count(&self) -> usize {
        self.namespaces.len()
    }
}

// Namespace I/O operations
pub struct NvmeIoRequest {
    pub namespace_id: u32,
    pub opcode: u8,
    pub lba: u64,
    pub count: u32,
    pub buffer: Vec<u8>,
    pub metadata: Option<Vec<u8>>,
}

impl NvmeIoRequest {
    pub fn read(namespace_id: u32, lba: u64, count: u32) -> Self {
        Self {
            namespace_id,
            opcode: NVME_IO_READ,
            lba,
            count,
            buffer: Vec::new(),
            metadata: None,
        }
    }
    
    pub fn write(namespace_id: u32, lba: u64, data: Vec<u8>) -> Self {
        let count = data.len() as u32 / 512; // Assuming 512 byte blocks
        Self {
            namespace_id,
            opcode: NVME_IO_WRITE,
            lba,
            count,
            buffer: data,
            metadata: None,
        }
    }
    
    pub fn flush(namespace_id: u32) -> Self {
        Self {
            namespace_id,
            opcode: NVME_IO_FLUSH,
            lba: 0,
            count: 0,
            buffer: Vec::new(),
            metadata: None,
        }
    }
    
    pub fn trim(namespace_id: u32, lba: u64, count: u32) -> Self {
        Self {
            namespace_id,
            opcode: NVME_IO_DSM,
            lba,
            count,
            buffer: Vec::new(),
            metadata: None,
        }
    }
}

// Namespace statistics
pub struct NvmeNamespaceStats {
    pub read_commands: u64,
    pub write_commands: u64,
    pub read_blocks: u64,
    pub write_blocks: u64,
    pub read_errors: u32,
    pub write_errors: u32,
    pub media_errors: u32,
}

impl NvmeNamespaceStats {
    pub fn new() -> Self {
        Self {
            read_commands: 0,
            write_commands: 0,
            read_blocks: 0,
            write_blocks: 0,
            read_errors: 0,
            write_errors: 0,
            media_errors: 0,
        }
    }
    
    pub fn record_read(&mut self, blocks: u32) {
        self.read_commands += 1;
        self.read_blocks += blocks as u64;
    }
    
    pub fn record_write(&mut self, blocks: u32) {
        self.write_commands += 1;
        self.write_blocks += blocks as u64;
    }
    
    pub fn record_error(&mut self, is_read: bool) {
        if is_read {
            self.read_errors += 1;
        } else {
            self.write_errors += 1;
        }
    }
    
    pub fn reset(&mut self) {
        *self = Self::new();
    }
}