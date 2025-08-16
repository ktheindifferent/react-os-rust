// NTFS Master File Table (MFT) Implementation
use alloc::vec::Vec;
use alloc::string::String;
use alloc::vec;
use crate::drivers::disk::DiskDriver;
use super::boot_sector::NtfsBootSector;
use super::attributes::{Attribute, parse_attributes};

// MFT Entry Header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MftEntryHeader {
    pub signature: [u8; 4],      // "FILE" or "BAAD"
    pub update_seq_offset: u16,
    pub update_seq_size: u16,
    pub log_seq_number: u64,
    pub sequence_number: u16,
    pub hard_link_count: u16,
    pub first_attr_offset: u16,
    pub flags: u16,
    pub used_size: u32,
    pub allocated_size: u32,
    pub file_reference: u64,
    pub next_attr_id: u16,
    pub reserved: u16,
    pub record_number: u32,
}

// MFT Entry Flags
pub const MFT_ENTRY_IN_USE: u16 = 0x0001;
pub const MFT_ENTRY_IS_DIRECTORY: u16 = 0x0002;
pub const MFT_ENTRY_IS_EXTENSION: u16 = 0x0004;
pub const MFT_ENTRY_HAS_VIEW_INDEX: u16 = 0x0008;

// MFT Entry
#[derive(Clone)]
pub struct MftEntry {
    pub header: MftEntryHeader,
    pub attributes: Vec<Attribute>,
    pub created_time: u64,
    pub modified_time: u64,
    pub accessed_time: u64,
    pub file_attributes: u32,
    raw_data: Vec<u8>,
}

impl MftEntry {
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 48 {
            return Err("MFT entry too small");
        }
        
        // Check signature
        if &data[0..4] != b"FILE" {
            if &data[0..4] == b"BAAD" {
                return Err("Bad MFT entry");
            }
            return Err("Invalid MFT entry signature");
        }
        
        // Parse header
        let header = MftEntryHeader {
            signature: [data[0], data[1], data[2], data[3]],
            update_seq_offset: u16::from_le_bytes([data[4], data[5]]),
            update_seq_size: u16::from_le_bytes([data[6], data[7]]),
            log_seq_number: u64::from_le_bytes([
                data[8], data[9], data[10], data[11],
                data[12], data[13], data[14], data[15],
            ]),
            sequence_number: u16::from_le_bytes([data[16], data[17]]),
            hard_link_count: u16::from_le_bytes([data[18], data[19]]),
            first_attr_offset: u16::from_le_bytes([data[20], data[21]]),
            flags: u16::from_le_bytes([data[22], data[23]]),
            used_size: u32::from_le_bytes([data[24], data[25], data[26], data[27]]),
            allocated_size: u32::from_le_bytes([data[28], data[29], data[30], data[31]]),
            file_reference: u64::from_le_bytes([
                data[32], data[33], data[34], data[35],
                data[36], data[37], data[38], data[39],
            ]),
            next_attr_id: u16::from_le_bytes([data[40], data[41]]),
            reserved: u16::from_le_bytes([data[42], data[43]]),
            record_number: u32::from_le_bytes([data[44], data[45], data[46], data[47]]),
        };
        
        // Apply fixup if needed
        let fixed_data = Self::apply_fixup(data, &header)?;
        
        // Parse attributes
        let attr_offset = header.first_attr_offset as usize;
        let attributes = parse_attributes(&fixed_data[attr_offset..])?;
        
        // Extract standard information
        let mut created_time = 0;
        let mut modified_time = 0;
        let mut accessed_time = 0;
        let mut file_attributes = 0;
        
        for attr in &attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_STANDARD_INFO {
                // Parse standard info attribute
                if let super::attributes::AttributeContent::Resident(data) = &attr.content {
                    if data.len() >= 48 {
                        created_time = u64::from_le_bytes([
                            data[0], data[1], data[2], data[3],
                            data[4], data[5], data[6], data[7],
                        ]);
                        modified_time = u64::from_le_bytes([
                            data[8], data[9], data[10], data[11],
                            data[12], data[13], data[14], data[15],
                        ]);
                        accessed_time = u64::from_le_bytes([
                            data[24], data[25], data[26], data[27],
                            data[28], data[29], data[30], data[31],
                        ]);
                        file_attributes = u32::from_le_bytes([
                            data[32], data[33], data[34], data[35],
                        ]);
                    }
                }
                break;
            }
        }
        
        Ok(Self {
            header,
            attributes,
            created_time,
            modified_time,
            accessed_time,
            file_attributes,
            raw_data: fixed_data,
        })
    }
    
    fn apply_fixup(data: &[u8], header: &MftEntryHeader) -> Result<Vec<u8>, &'static str> {
        let mut fixed = data.to_vec();
        
        if header.update_seq_size == 0 {
            return Ok(fixed);
        }
        
        let update_seq_offset = header.update_seq_offset as usize;
        let update_seq_size = header.update_seq_size as usize;
        
        if update_seq_offset + update_seq_size * 2 > data.len() {
            return Err("Invalid update sequence");
        }
        
        // Get update sequence number
        let usn = u16::from_le_bytes([
            data[update_seq_offset],
            data[update_seq_offset + 1],
        ]);
        
        // Apply fixup values
        for i in 1..update_seq_size {
            let fixup_offset = 510 + (i - 1) * 512;
            if fixup_offset + 1 < fixed.len() {
                let fixup_value_offset = update_seq_offset + i * 2;
                fixed[fixup_offset] = data[fixup_value_offset];
                fixed[fixup_offset + 1] = data[fixup_value_offset + 1];
            }
        }
        
        Ok(fixed)
    }
    
    pub fn is_in_use(&self) -> bool {
        self.header.flags & MFT_ENTRY_IN_USE != 0
    }
    
    pub fn is_directory(&self) -> bool {
        self.header.flags & MFT_ENTRY_IS_DIRECTORY != 0
    }
    
    pub fn get_attribute(&self, type_code: u32) -> Option<&Attribute> {
        self.attributes.iter().find(|attr| attr.type_code == type_code)
    }
    
    pub fn get_file_name(&self) -> Option<String> {
        for attr in &self.attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_FILE_NAME {
                if let super::attributes::AttributeContent::Resident(data) = &attr.content {
                    // Parse file name attribute
                    if data.len() >= 66 {
                        let name_len = data[64] as usize;
                        let name_type = data[65];
                        
                        if data.len() >= 66 + name_len * 2 {
                            // Convert UTF-16 to String
                            let mut name = String::new();
                            for i in 0..name_len {
                                let offset = 66 + i * 2;
                                let ch = u16::from_le_bytes([data[offset], data[offset + 1]]);
                                if let Some(c) = char::from_u32(ch as u32) {
                                    name.push(c);
                                }
                            }
                            return Some(name);
                        }
                    }
                }
            }
        }
        None
    }
}

// Master File Table
pub struct MasterFileTable {
    mft_start_sector: u64,
    entry_size: u32,
    sectors_per_entry: u64,
    cache: Vec<Option<MftEntry>>,
}

impl MasterFileTable {
    pub fn new(disk: &mut dyn DiskDriver, mft_start_sector: u64, boot_sector: &NtfsBootSector) -> Result<Self, &'static str> {
        let entry_size = boot_sector.get_mft_record_size();
        let sectors_per_entry = (entry_size / boot_sector.bytes_per_sector as u32) as u64;
        
        Ok(Self {
            mft_start_sector,
            entry_size,
            sectors_per_entry,
            cache: Vec::new(),
        })
    }
    
    pub fn read_entry(&self, entry_num: u64) -> Result<MftEntry, &'static str> {
        // Check cache first
        if entry_num < self.cache.len() as u64 {
            if let Some(ref entry) = self.cache[entry_num as usize] {
                return Ok(entry.clone());
            }
        }
        
        // Read from disk
        let sector = self.mft_start_sector + entry_num * self.sectors_per_entry;
        let mut data = vec![0u8; self.entry_size as usize];
        
        // Note: Would need disk reference here
        // For now, return error
        Err("MFT entry reading not fully implemented")
    }
    
    pub fn cache_entry(&mut self, entry_num: u64, entry: MftEntry) {
        // Extend cache if needed
        while self.cache.len() <= entry_num as usize {
            self.cache.push(None);
        }
        self.cache[entry_num as usize] = Some(entry);
    }
}