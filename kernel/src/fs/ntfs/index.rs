// NTFS Index Implementation (B+ Tree for directories)
use alloc::vec::Vec;
use alloc::string::String;

// Index structures for directory entries
pub struct IndexRoot {
    pub index_type: u32,
    pub collation_rule: u32,
    pub index_block_size: u32,
    pub clusters_per_index_block: u8,
}

pub struct IndexEntry {
    pub file_reference: u64,
    pub length: u16,
    pub key_length: u16,
    pub flags: u16,
    pub key: Vec<u8>,
}

impl IndexRoot {
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 16 {
            return Err("Index root too small");
        }
        
        Ok(Self {
            index_type: u32::from_le_bytes([data[0], data[1], data[2], data[3]]),
            collation_rule: u32::from_le_bytes([data[4], data[5], data[6], data[7]]),
            index_block_size: u32::from_le_bytes([data[8], data[9], data[10], data[11]]),
            clusters_per_index_block: data[12],
        })
    }
}