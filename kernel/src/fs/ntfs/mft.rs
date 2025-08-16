// NTFS Master File Table (MFT) Implementation
use alloc::vec::Vec;
use alloc::string::String;
use alloc::vec;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use spin::Mutex;
use crate::drivers::disk::DiskDriver;
use super::boot_sector::NtfsBootSector;
use super::attributes::{Attribute, AttributeContent, parse_attributes, ATTR_TYPE_BITMAP};
use super::journal::{JournalManager, OperationType};

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
    bitmap: Mutex<MftBitmap>,
    next_free_entry: Mutex<u64>,
    journal: Option<Box<JournalManager>>,
}

// MFT Bitmap for tracking free entries
pub struct MftBitmap {
    data: Vec<u8>,
    total_entries: u64,
}

impl MftBitmap {
    pub fn new(total_entries: u64) -> Self {
        let bytes_needed = (total_entries + 7) / 8;
        let mut data = vec![0u8; bytes_needed as usize];
        
        // Mark system entries as used (first 16 entries)
        for i in 0..16 {
            Self::set_bit(&mut data, i);
        }
        
        Self {
            data,
            total_entries,
        }
    }
    
    fn set_bit(data: &mut [u8], index: u64) {
        let byte_index = (index / 8) as usize;
        let bit_index = (index % 8) as u8;
        if byte_index < data.len() {
            data[byte_index] |= 1 << bit_index;
        }
    }
    
    fn clear_bit(data: &mut [u8], index: u64) {
        let byte_index = (index / 8) as usize;
        let bit_index = (index % 8) as u8;
        if byte_index < data.len() {
            data[byte_index] &= !(1 << bit_index);
        }
    }
    
    fn is_bit_set(&self, index: u64) -> bool {
        let byte_index = (index / 8) as usize;
        let bit_index = (index % 8) as u8;
        if byte_index < self.data.len() {
            (self.data[byte_index] & (1 << bit_index)) != 0
        } else {
            false
        }
    }
    
    pub fn find_free_entry(&self, start_from: u64) -> Option<u64> {
        for i in start_from..self.total_entries {
            if !self.is_bit_set(i) {
                return Some(i);
            }
        }
        None
    }
    
    pub fn allocate_entry(&mut self, entry_num: u64) -> bool {
        if entry_num < self.total_entries && !self.is_bit_set(entry_num) {
            Self::set_bit(&mut self.data, entry_num);
            true
        } else {
            false
        }
    }
    
    pub fn deallocate_entry(&mut self, entry_num: u64) -> bool {
        if entry_num < self.total_entries && self.is_bit_set(entry_num) {
            Self::clear_bit(&mut self.data, entry_num);
            true
        } else {
            false
        }
    }
}

impl MasterFileTable {
    pub fn new(disk: &mut dyn DiskDriver, mft_start_sector: u64, boot_sector: &NtfsBootSector) -> Result<Self, &'static str> {
        let entry_size = boot_sector.get_mft_record_size();
        let sectors_per_entry = (entry_size / boot_sector.bytes_per_sector as u32) as u64;
        
        // Calculate total MFT entries based on MFT size
        let total_entries = 65536; // Default, should be calculated from MFT size
        
        Ok(Self {
            mft_start_sector,
            entry_size,
            sectors_per_entry,
            cache: Vec::new(),
            bitmap: Mutex::new(MftBitmap::new(total_entries)),
            next_free_entry: Mutex::new(16), // Start after system entries
            journal: None,
        })
    }
    
    pub fn set_journal(&mut self, journal: Box<JournalManager>) {
        self.journal = Some(journal);
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
    
    // Write Operations
    pub fn allocate_entry(&mut self, disk: &mut dyn DiskDriver) -> Result<(u64, MftEntry), &'static str> {
        let entry_num = {
            let mut bitmap = self.bitmap.lock();
            let mut next_free = self.next_free_entry.lock();
            
            // Find free entry
            let entry_num = bitmap.find_free_entry(*next_free)
                .ok_or("No free MFT entries available")?;
            
            // Allocate in bitmap
            if !bitmap.allocate_entry(entry_num) {
                return Err("Failed to allocate MFT entry");
            }
            
            // Update next free hint
            *next_free = entry_num + 1;
            entry_num
        }; // Drop locks
        
        // Create new MFT entry
        let mut entry = self.create_empty_entry(entry_num)?;
        
        // Write to disk
        self.write_entry(disk, entry_num, &entry)?;
        
        // Cache the entry
        self.cache_entry(entry_num, entry.clone());
        
        Ok((entry_num, entry))
    }
    
    pub fn deallocate_entry(&mut self, disk: &mut dyn DiskDriver, entry_num: u64) -> Result<(), &'static str> {
        {
            let mut bitmap = self.bitmap.lock();
            
            // Mark as free in bitmap
            if !bitmap.deallocate_entry(entry_num) {
                return Err("Entry was not allocated");
            }
        } // Drop lock
        
        // Read the entry
        let mut entry = self.read_entry_from_disk(disk, entry_num)?;
        
        // Mark entry as not in use
        entry.header.flags &= !MFT_ENTRY_IN_USE;
        entry.header.sequence_number += 1; // Increment sequence for reuse detection
        
        // Write back to disk
        self.write_entry(disk, entry_num, &entry)?;
        
        // Remove from cache
        if entry_num < self.cache.len() as u64 {
            self.cache[entry_num as usize] = None;
        }
        
        Ok(())
    }
    
    pub fn write_entry(&mut self, disk: &mut dyn DiskDriver, entry_num: u64, entry: &MftEntry) -> Result<(), &'static str> {
        let sector = self.mft_start_sector + entry_num * self.sectors_per_entry;
        let data = self.serialize_entry(entry)?;
        
        // Log operation if journal is available
        if let Some(ref journal) = self.journal {
            let transaction_id = journal.begin_transaction();
            
            // Read old data for undo
            let mut old_data = vec![0u8; self.entry_size as usize];
            disk.read_sectors(sector, self.sectors_per_entry as u32, &mut old_data)
                .map_err(|_| "Failed to read old MFT entry")?;
            
            // Log the operation
            journal.log_operation(
                transaction_id,
                OperationType::WriteData,
                0, // MFT attribute
                entry_num,
                Some(old_data),
                Some(data.clone()),
            ).map_err(|_| "Failed to log MFT operation")?;
            
            // Write new data
            disk.write_sectors(sector, self.sectors_per_entry as u32, &data)
                .map_err(|_| "Failed to write MFT entry")?;
            
            // Commit transaction
            journal.commit_transaction(transaction_id)
                .map_err(|_| "Failed to commit MFT transaction")?;
        } else {
            // Direct write without journaling
            disk.write_sectors(sector, self.sectors_per_entry as u32, &data)
                .map_err(|_| "Failed to write MFT entry")?;
        }
        
        Ok(())
    }
    
    fn create_empty_entry(&self, entry_num: u64) -> Result<MftEntry, &'static str> {
        let mut raw_data = vec![0u8; self.entry_size as usize];
        
        // Set up header
        raw_data[0..4].copy_from_slice(b"FILE");
        
        let header = MftEntryHeader {
            signature: *b"FILE",
            update_seq_offset: 48,
            update_seq_size: 3,
            log_seq_number: 0,
            sequence_number: 1,
            hard_link_count: 0,
            first_attr_offset: 56,
            flags: MFT_ENTRY_IN_USE,
            used_size: 56 + 8, // Header + end marker
            allocated_size: self.entry_size,
            file_reference: entry_num,
            next_attr_id: 0,
            reserved: 0,
            record_number: entry_num as u32,
        };
        
        Ok(MftEntry {
            header,
            attributes: Vec::new(),
            created_time: 0,
            modified_time: 0,
            accessed_time: 0,
            file_attributes: 0,
            raw_data,
        })
    }
    
    fn serialize_entry(&self, entry: &MftEntry) -> Result<Vec<u8>, &'static str> {
        let mut data = vec![0u8; self.entry_size as usize];
        
        // Write header
        data[0..4].copy_from_slice(&entry.header.signature);
        data[4..6].copy_from_slice(&entry.header.update_seq_offset.to_le_bytes());
        data[6..8].copy_from_slice(&entry.header.update_seq_size.to_le_bytes());
        data[8..16].copy_from_slice(&entry.header.log_seq_number.to_le_bytes());
        data[16..18].copy_from_slice(&entry.header.sequence_number.to_le_bytes());
        data[18..20].copy_from_slice(&entry.header.hard_link_count.to_le_bytes());
        data[20..22].copy_from_slice(&entry.header.first_attr_offset.to_le_bytes());
        data[22..24].copy_from_slice(&entry.header.flags.to_le_bytes());
        data[24..28].copy_from_slice(&entry.header.used_size.to_le_bytes());
        data[28..32].copy_from_slice(&entry.header.allocated_size.to_le_bytes());
        data[32..40].copy_from_slice(&entry.header.file_reference.to_le_bytes());
        data[40..42].copy_from_slice(&entry.header.next_attr_id.to_le_bytes());
        data[42..44].copy_from_slice(&entry.header.reserved.to_le_bytes());
        data[44..48].copy_from_slice(&entry.header.record_number.to_le_bytes());
        
        // Add update sequence array
        let update_seq_offset = entry.header.update_seq_offset as usize;
        if update_seq_offset + 2 * entry.header.update_seq_size as usize <= data.len() {
            // Write USN and fixup values
            let usn = 0x0001u16;
            data[update_seq_offset..update_seq_offset + 2].copy_from_slice(&usn.to_le_bytes());
        }
        
        // Write attributes
        let mut attr_offset = entry.header.first_attr_offset as usize;
        for attr in &entry.attributes {
            let attr_data = self.serialize_attribute(attr)?;
            if attr_offset + attr_data.len() <= data.len() {
                data[attr_offset..attr_offset + attr_data.len()].copy_from_slice(&attr_data);
                attr_offset += attr_data.len();
            }
        }
        
        // Write end marker
        if attr_offset + 4 <= data.len() {
            data[attr_offset..attr_offset + 4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
        }
        
        // Apply fixup
        self.apply_fixup(&mut data, &entry.header)?;
        
        Ok(data)
    }
    
    fn serialize_attribute(&self, attr: &Attribute) -> Result<Vec<u8>, &'static str> {
        // Serialize an attribute to bytes
        // This is a simplified version
        let mut data = Vec::new();
        
        // Write type
        data.extend_from_slice(&attr.type_code.to_le_bytes());
        
        // Calculate and write length
        let content_len = match &attr.content {
            AttributeContent::Resident(d) => 24 + d.len(),
            AttributeContent::NonResident(_) => 64,
        };
        
        let total_len = ((content_len + 7) & !7) as u32; // Align to 8 bytes
        data.extend_from_slice(&total_len.to_le_bytes());
        
        // Write non-resident flag
        match &attr.content {
            AttributeContent::Resident(_) => data.push(0),
            AttributeContent::NonResident(_) => data.push(1),
        }
        
        // Write name length and offset
        data.push(0); // name_length
        data.extend_from_slice(&[0u8; 2]); // name_offset
        
        // Write flags and attribute ID
        data.extend_from_slice(&attr.flags.to_le_bytes());
        data.extend_from_slice(&[0u8; 2]); // attribute_id
        
        // Write content
        match &attr.content {
            AttributeContent::Resident(content_data) => {
                // Write resident header
                data.extend_from_slice(&(content_data.len() as u32).to_le_bytes());
                data.extend_from_slice(&(24u16).to_le_bytes()); // value_offset
                data.extend_from_slice(&[0u8; 2]); // indexed_flag and padding
                
                // Write data
                data.extend_from_slice(content_data);
            }
            AttributeContent::NonResident(non_res) => {
                // Write non-resident header
                data.extend_from_slice(&non_res.start_vcn.to_le_bytes());
                data.extend_from_slice(&non_res.last_vcn.to_le_bytes());
                data.extend_from_slice(&(64u16).to_le_bytes()); // data_runs_offset
                data.extend_from_slice(&[0u8; 2]); // compression_unit_size
                data.extend_from_slice(&[0u8; 4]); // padding
                data.extend_from_slice(&non_res.allocated_size.to_le_bytes());
                data.extend_from_slice(&non_res.real_size.to_le_bytes());
                data.extend_from_slice(&non_res.initialized_size.to_le_bytes());
                
                // Write data runs
                for run in &non_res.data_runs {
                    // Simplified data run encoding
                    // Real implementation would properly encode runs
                }
            }
        }
        
        // Pad to 8-byte alignment
        while data.len() < total_len as usize {
            data.push(0);
        }
        
        Ok(data)
    }
    
    fn apply_fixup(&self, data: &mut [u8], header: &MftEntryHeader) -> Result<(), &'static str> {
        if header.update_seq_size == 0 {
            return Ok(());
        }
        
        let update_seq_offset = header.update_seq_offset as usize;
        let update_seq_size = header.update_seq_size as usize;
        
        // Read USN
        let usn = u16::from_le_bytes([
            data[update_seq_offset],
            data[update_seq_offset + 1],
        ]);
        
        // Apply fixup values to sector boundaries
        for i in 1..update_seq_size {
            let sector_offset = i * 512 - 2;
            if sector_offset + 1 < data.len() {
                // Save original values as fixup
                let fixup_offset = update_seq_offset + i * 2;
                if fixup_offset + 1 < data.len() {
                    data[fixup_offset] = data[sector_offset];
                    data[fixup_offset + 1] = data[sector_offset + 1];
                    
                    // Write USN at sector boundary
                    data[sector_offset] = (usn & 0xFF) as u8;
                    data[sector_offset + 1] = ((usn >> 8) & 0xFF) as u8;
                }
            }
        }
        
        Ok(())
    }
    
    fn read_entry_from_disk(&mut self, disk: &mut dyn DiskDriver, entry_num: u64) -> Result<MftEntry, &'static str> {
        let sector = self.mft_start_sector + entry_num * self.sectors_per_entry;
        let mut data = vec![0u8; self.entry_size as usize];
        
        disk.read_sectors(sector, self.sectors_per_entry as u32, &mut data)
            .map_err(|_| "Failed to read MFT entry")?;
        
        MftEntry::parse(&data)
    }
    
    pub fn update_entry_times(&mut self, disk: &mut dyn DiskDriver, entry_num: u64, created: Option<u64>, modified: Option<u64>, accessed: Option<u64>) -> Result<(), &'static str> {
        let mut entry = self.read_entry_from_disk(disk, entry_num)?;
        
        if let Some(time) = created {
            entry.created_time = time;
        }
        if let Some(time) = modified {
            entry.modified_time = time;
        }
        if let Some(time) = accessed {
            entry.accessed_time = time;
        }
        
        // Update standard information attribute
        for attr in &mut entry.attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_STANDARD_INFO {
                if let AttributeContent::Resident(ref mut data) = attr.content {
                    if data.len() >= 48 {
                        // Update times in attribute data
                        if let Some(time) = created {
                            data[0..8].copy_from_slice(&time.to_le_bytes());
                        }
                        if let Some(time) = modified {
                            data[8..16].copy_from_slice(&time.to_le_bytes());
                        }
                        if let Some(time) = accessed {
                            data[24..32].copy_from_slice(&time.to_le_bytes());
                        }
                    }
                }
                break;
            }
        }
        
        self.write_entry(disk, entry_num, &entry)?;
        Ok(())
    }
}