// NTFS Index Implementation (B+ Tree for directories)
use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use core::cmp::Ordering;

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

// Index Header
pub struct IndexHeader {
    pub entries_offset: u32,
    pub index_length: u32,
    pub allocated_size: u32,
    pub flags: u8,
}

// Index Node Header (for non-leaf nodes)
pub struct IndexNodeHeader {
    pub header: IndexHeader,
    pub vcn: u64,  // Virtual Cluster Number for child node
}

// File Name Index Entry
#[derive(Clone)]
pub struct FileNameIndexEntry {
    pub file_reference: u64,
    pub parent_reference: u64,
    pub file_name: String,
    pub file_size: u64,
    pub file_attributes: u32,
    pub created_time: u64,
    pub modified_time: u64,
    pub flags: u16,
}

// Index tree node
pub struct IndexNode {
    pub entries: Vec<FileNameIndexEntry>,
    pub children: Vec<u64>,  // VCNs of child nodes
    pub is_leaf: bool,
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
    
    pub fn create_empty() -> Vec<u8> {
        let mut data = vec![0u8; 64];
        
        // Index type (0x30 = file name)
        data[0..4].copy_from_slice(&0x30u32.to_le_bytes());
        // Collation rule (1 = file name)
        data[4..8].copy_from_slice(&1u32.to_le_bytes());
        // Index block size
        data[8..12].copy_from_slice(&4096u32.to_le_bytes());
        // Clusters per index block
        data[12] = 1;
        
        // Index header
        let header_offset = 16;
        // Entries offset (relative to header)
        data[header_offset..header_offset + 4].copy_from_slice(&24u32.to_le_bytes());
        // Index length
        data[header_offset + 4..header_offset + 8].copy_from_slice(&32u32.to_le_bytes());
        // Allocated size
        data[header_offset + 8..header_offset + 12].copy_from_slice(&48u32.to_le_bytes());
        // Flags (0 = small index)
        data[header_offset + 12] = 0;
        
        data
    }
}

impl IndexEntry {
    pub fn parse(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < 16 {
            return Err("Index entry too small");
        }
        
        let file_reference = u64::from_le_bytes([
            data[0], data[1], data[2], data[3],
            data[4], data[5], data[6], data[7],
        ]);
        
        let length = u16::from_le_bytes([data[8], data[9]]);
        let key_length = u16::from_le_bytes([data[10], data[11]]);
        let flags = u16::from_le_bytes([data[12], data[13]]);
        
        let mut key = Vec::new();
        if key_length > 0 && 16 + key_length as usize <= data.len() {
            key.extend_from_slice(&data[16..16 + key_length as usize]);
        }
        
        Ok(Self {
            file_reference,
            length,
            key_length,
            flags,
            key,
        })
    }
    
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // File reference
        data.extend_from_slice(&self.file_reference.to_le_bytes());
        // Length
        data.extend_from_slice(&self.length.to_le_bytes());
        // Key length
        data.extend_from_slice(&self.key_length.to_le_bytes());
        // Flags
        data.extend_from_slice(&self.flags.to_le_bytes());
        // Key data
        data.extend_from_slice(&self.key);
        
        // Pad to 8-byte alignment
        while data.len() % 8 != 0 {
            data.push(0);
        }
        
        data
    }
}

impl FileNameIndexEntry {
    pub fn new(
        file_reference: u64,
        parent_reference: u64,
        file_name: String,
        file_size: u64,
        file_attributes: u32,
    ) -> Self {
        let timestamp = 0x01D7C4F0A0000000u64; // Placeholder timestamp
        
        Self {
            file_reference,
            parent_reference,
            file_name,
            file_size,
            file_attributes,
            created_time: timestamp,
            modified_time: timestamp,
            flags: 0,
        }
    }
    
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Parent reference
        data.extend_from_slice(&self.parent_reference.to_le_bytes());
        // Creation time
        data.extend_from_slice(&self.created_time.to_le_bytes());
        // Modification time
        data.extend_from_slice(&self.modified_time.to_le_bytes());
        // MFT change time
        data.extend_from_slice(&self.modified_time.to_le_bytes());
        // Access time
        data.extend_from_slice(&self.modified_time.to_le_bytes());
        // Allocated size
        data.extend_from_slice(&self.file_size.to_le_bytes());
        // Real size
        data.extend_from_slice(&self.file_size.to_le_bytes());
        // File attributes
        data.extend_from_slice(&self.file_attributes.to_le_bytes());
        // EA size and reparse tag
        data.extend_from_slice(&0u32.to_le_bytes());
        // File name length
        data.push(self.file_name.len() as u8);
        // File name type (1 = Windows)
        data.push(1);
        
        // File name as UTF-16
        for ch in self.file_name.chars() {
            let utf16 = ch as u16;
            data.extend_from_slice(&utf16.to_le_bytes());
        }
        
        data
    }
    
    pub fn compare(&self, other: &Self) -> Ordering {
        // Case-insensitive comparison for NTFS
        self.file_name.to_uppercase().cmp(&other.file_name.to_uppercase())
    }
}

// B+ Tree implementation for directory index
pub struct DirectoryIndexTree {
    root: IndexNode,
    max_entries_per_node: usize,
}

impl DirectoryIndexTree {
    pub fn new() -> Self {
        Self {
            root: IndexNode {
                entries: Vec::new(),
                children: Vec::new(),
                is_leaf: true,
            },
            max_entries_per_node: 10, // Simplified value
        }
    }
    
    pub fn insert(&mut self, entry: FileNameIndexEntry) -> Result<(), &'static str> {
        // Simplified B+ tree insertion
        // Real implementation would handle node splits and rebalancing
        
        // Find insertion point
        let mut insert_pos = 0;
        for (i, existing) in self.root.entries.iter().enumerate() {
            if entry.compare(existing) == Ordering::Less {
                insert_pos = i;
                break;
            }
            insert_pos = i + 1;
        }
        
        // Insert entry
        self.root.entries.insert(insert_pos, entry);
        
        // Check if node needs splitting
        if self.root.entries.len() > self.max_entries_per_node {
            self.split_node()?;
        }
        
        Ok(())
    }
    
    pub fn remove(&mut self, file_name: &str) -> Result<(), &'static str> {
        // Find and remove entry
        let mut found_index = None;
        for (i, entry) in self.root.entries.iter().enumerate() {
            if entry.file_name.eq_ignore_ascii_case(file_name) {
                found_index = Some(i);
                break;
            }
        }
        
        if let Some(index) = found_index {
            self.root.entries.remove(index);
            
            // Check if node needs merging (underflow)
            if self.root.entries.len() < self.max_entries_per_node / 2 {
                self.merge_node()?;
            }
            
            Ok(())
        } else {
            Err("Entry not found")
        }
    }
    
    pub fn find(&self, file_name: &str) -> Option<&FileNameIndexEntry> {
        for entry in &self.root.entries {
            if entry.file_name.eq_ignore_ascii_case(file_name) {
                return Some(entry);
            }
        }
        None
    }
    
    pub fn list_all(&self) -> Vec<FileNameIndexEntry> {
        self.root.entries.clone()
    }
    
    fn split_node(&mut self) -> Result<(), &'static str> {
        // Simplified node splitting
        // Real implementation would create new nodes and update parent
        if self.root.entries.len() <= self.max_entries_per_node {
            return Ok(());
        }
        
        // For now, just truncate (not a proper B+ tree split)
        self.root.entries.truncate(self.max_entries_per_node);
        
        Ok(())
    }
    
    fn merge_node(&mut self) -> Result<(), &'static str> {
        // Simplified node merging
        // Real implementation would merge with siblings and update parent
        Ok(())
    }
    
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Serialize all entries
        for entry in &self.root.entries {
            let entry_data = entry.serialize();
            data.extend_from_slice(&entry_data);
        }
        
        data
    }
}