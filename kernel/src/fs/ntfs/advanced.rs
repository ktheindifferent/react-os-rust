// NTFS Advanced Features Implementation
use alloc::vec::Vec;
use alloc::string::String;
use super::{NtfsFileSystem, MFT_ENTRY_ROOT};
use super::mft::MftEntry;
use super::attributes::{
    Attribute, AttributeContent, ATTR_TYPE_REPARSE_POINT,
    ATTR_TYPE_FILE_NAME, ATTR_TYPE_STANDARD_INFO,
};

// Reparse Point Tags
pub const IO_REPARSE_TAG_MOUNT_POINT: u32 = 0xA0000003;
pub const IO_REPARSE_TAG_SYMLINK: u32 = 0xA000000C;
pub const IO_REPARSE_TAG_DEDUP: u32 = 0x80000013;
pub const IO_REPARSE_TAG_NFS: u32 = 0x80000014;

// Reparse Point Structure
#[derive(Debug, Clone)]
pub struct ReparsePoint {
    pub tag: u32,
    pub data_length: u16,
    pub reserved: u16,
    pub data: Vec<u8>,
}

// Symbolic Link Data
#[derive(Debug, Clone)]
pub struct SymbolicLinkData {
    pub target_path: String,
    pub print_name: String,
    pub flags: u32,
}

// Hard Link Support
impl NtfsFileSystem {
    // Create a hard link to an existing file
    pub fn create_hard_link(&mut self, existing_path: &str, new_link_path: &str) -> Result<(), &'static str> {
        // Parse paths
        let existing_components: Vec<&str> = existing_path.split('\\').filter(|s| !s.is_empty()).collect();
        let new_components: Vec<&str> = new_link_path.split('\\').filter(|s| !s.is_empty()).collect();
        
        if existing_components.is_empty() || new_components.is_empty() {
            return Err("Invalid path");
        }
        
        // Find existing file
        let existing_entry_num = self.find_entry_by_path(existing_path)?;
        let mut existing_entry = self.mft.read_entry(existing_entry_num)?;
        
        // Ensure it's not a directory
        if existing_entry.is_directory() {
            return Err("Cannot create hard link to directory");
        }
        
        // Get parent directory for new link
        let new_parent_path = &new_components[..new_components.len() - 1];
        let new_name = new_components.last().unwrap();
        
        let new_parent_entry = if new_parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&new_parent_path.join("\\"))?
        };
        
        // Check if new name already exists
        if self.find_file_in_directory(new_parent_entry, new_name).is_ok() {
            return Err("Target name already exists");
        }
        
        // Create new file name attribute for the hard link
        let timestamp = Self::get_current_timestamp();
        let file_name_attr = super::attributes::create_file_name_attribute(
            new_parent_entry,
            new_name,
            false,
        );
        
        // Add the new file name attribute to the existing entry
        existing_entry.attributes.push(file_name_attr);
        
        // Increment hard link count
        existing_entry.header.hard_link_count += 1;
        
        // Update modification time
        existing_entry.modified_time = timestamp;
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, existing_entry_num, &existing_entry)?;
        
        // Add to new parent directory index
        self.add_to_directory_index(new_parent_entry, existing_entry_num, new_name)?;
        
        Ok(())
    }
    
    // Remove a hard link
    pub fn remove_hard_link(&mut self, link_path: &str) -> Result<(), &'static str> {
        // Parse path
        let components: Vec<&str> = link_path.split('\\').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid path");
        }
        
        let parent_path = &components[..components.len() - 1];
        let link_name = components.last().unwrap();
        
        // Find parent directory
        let parent_entry_num = if parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&parent_path.join("\\"))?
        };
        
        // Find the linked file
        let entry_num = self.find_file_in_directory(parent_entry_num, link_name)?;
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Check hard link count
        if entry.header.hard_link_count <= 1 {
            // This is the last link, delete the file
            return self.delete_file_impl(link_path);
        }
        
        // Find and remove the specific file name attribute
        let mut removed = false;
        entry.attributes.retain(|attr| {
            if attr.type_code == ATTR_TYPE_FILE_NAME {
                if let AttributeContent::Resident(data) = &attr.content {
                    // Parse parent reference from file name attribute
                    if data.len() >= 8 {
                        let parent_ref = u64::from_le_bytes([
                            data[0], data[1], data[2], data[3],
                            data[4], data[5], data[6], data[7],
                        ]);
                        
                        if parent_ref == parent_entry_num {
                            removed = true;
                            return false; // Remove this attribute
                        }
                    }
                }
            }
            true // Keep other attributes
        });
        
        if !removed {
            return Err("Link not found");
        }
        
        // Decrement hard link count
        entry.header.hard_link_count -= 1;
        
        // Update modification time
        let timestamp = Self::get_current_timestamp();
        entry.modified_time = timestamp;
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        // Remove from parent directory index
        self.remove_from_directory_index(parent_entry_num, entry_num, link_name)?;
        
        Ok(())
    }
}

// Symbolic Link Support
impl NtfsFileSystem {
    // Create a symbolic link
    pub fn create_symbolic_link(&mut self, link_path: &str, target_path: &str) -> Result<(), &'static str> {
        // Parse link path
        let components: Vec<&str> = link_path.split('\\').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid link path");
        }
        
        let parent_path = &components[..components.len() - 1];
        let link_name = components.last().unwrap();
        
        // Find parent directory
        let parent_entry_num = if parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&parent_path.join("\\"))?
        };
        
        // Check if name already exists
        if self.find_file_in_directory(parent_entry_num, link_name).is_ok() {
            return Err("Link name already exists");
        }
        
        // Allocate new MFT entry for the symlink
        let (entry_num, mut entry) = self.mft.allocate_entry(&mut *self.disk)?;
        
        // Get current timestamp
        let timestamp = Self::get_current_timestamp();
        
        // Add standard information attribute
        let std_info = super::attributes::create_standard_info_attribute(
            timestamp,
            timestamp,
            timestamp,
            0x400, // Reparse point attribute
        );
        entry.attributes.push(std_info);
        
        // Add file name attribute
        let file_name_attr = super::attributes::create_file_name_attribute(
            parent_entry_num,
            link_name,
            false,
        );
        entry.attributes.push(file_name_attr);
        
        // Create reparse point attribute
        let reparse_attr = self.create_symlink_reparse_attribute(target_path)?;
        entry.attributes.push(reparse_attr);
        
        // Update entry metadata
        entry.created_time = timestamp;
        entry.modified_time = timestamp;
        entry.accessed_time = timestamp;
        entry.file_attributes = 0x400; // Reparse point
        
        // Write MFT entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        // Add to parent directory index
        self.add_to_directory_index(parent_entry_num, entry_num, link_name)?;
        
        Ok(())
    }
    
    // Create a junction (directory symbolic link)
    pub fn create_junction(&mut self, junction_path: &str, target_path: &str) -> Result<(), &'static str> {
        // Similar to symbolic link but with different reparse tag
        // Parse junction path
        let components: Vec<&str> = junction_path.split('\\').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid junction path");
        }
        
        let parent_path = &components[..components.len() - 1];
        let junction_name = components.last().unwrap();
        
        // Find parent directory
        let parent_entry_num = if parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&parent_path.join("\\"))?
        };
        
        // Allocate new MFT entry
        let (entry_num, mut entry) = self.mft.allocate_entry(&mut *self.disk)?;
        
        // Set directory flag
        entry.header.flags |= super::mft::MFT_ENTRY_IS_DIRECTORY;
        
        // Get current timestamp
        let timestamp = Self::get_current_timestamp();
        
        // Add standard information attribute
        let std_info = super::attributes::create_standard_info_attribute(
            timestamp,
            timestamp,
            timestamp,
            0x10000400, // Directory | Reparse point
        );
        entry.attributes.push(std_info);
        
        // Add file name attribute
        let file_name_attr = super::attributes::create_file_name_attribute(
            parent_entry_num,
            junction_name,
            true,
        );
        entry.attributes.push(file_name_attr);
        
        // Create mount point reparse attribute
        let reparse_attr = self.create_junction_reparse_attribute(target_path)?;
        entry.attributes.push(reparse_attr);
        
        // Add index root for directory
        let index_root = self.create_index_root_attribute()?;
        entry.attributes.push(index_root);
        
        // Update entry metadata
        entry.created_time = timestamp;
        entry.modified_time = timestamp;
        entry.accessed_time = timestamp;
        entry.file_attributes = 0x10000400;
        
        // Write MFT entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        // Add to parent directory index
        self.add_to_directory_index(parent_entry_num, entry_num, junction_name)?;
        
        Ok(())
    }
    
    // Read symbolic link target
    pub fn read_symbolic_link(&mut self, link_path: &str) -> Result<String, &'static str> {
        // Find the symlink entry
        let entry_num = self.find_entry_by_path(link_path)?;
        let entry = self.mft.read_entry(entry_num)?;
        
        // Check if it's a reparse point
        if (entry.file_attributes & 0x400) == 0 {
            return Err("Not a symbolic link");
        }
        
        // Find reparse point attribute
        let reparse_attr = entry.get_attribute(ATTR_TYPE_REPARSE_POINT)
            .ok_or("No reparse point attribute")?;
        
        // Parse reparse data
        if let AttributeContent::Resident(data) = &reparse_attr.content {
            let reparse_point = Self::parse_reparse_point(data)?;
            
            if reparse_point.tag == IO_REPARSE_TAG_SYMLINK {
                let symlink_data = Self::parse_symlink_data(&reparse_point.data)?;
                Ok(symlink_data.target_path)
            } else if reparse_point.tag == IO_REPARSE_TAG_MOUNT_POINT {
                let junction_data = Self::parse_junction_data(&reparse_point.data)?;
                Ok(junction_data)
            } else {
                Err("Unknown reparse point type")
            }
        } else {
            Err("Reparse point data is non-resident")
        }
    }
    
    // Helper functions for reparse points
    
    fn create_symlink_reparse_attribute(&self, target_path: &str) -> Result<Attribute, &'static str> {
        let symlink_data = Self::create_symlink_data(target_path);
        let reparse_point = ReparsePoint {
            tag: IO_REPARSE_TAG_SYMLINK,
            data_length: symlink_data.len() as u16,
            reserved: 0,
            data: symlink_data,
        };
        
        let reparse_data = Self::serialize_reparse_point(&reparse_point);
        
        Ok(Attribute {
            type_code: ATTR_TYPE_REPARSE_POINT,
            name: String::new(),
            flags: 0,
            content: AttributeContent::Resident(reparse_data),
        })
    }
    
    fn create_junction_reparse_attribute(&self, target_path: &str) -> Result<Attribute, &'static str> {
        let junction_data = Self::create_junction_data(target_path);
        let reparse_point = ReparsePoint {
            tag: IO_REPARSE_TAG_MOUNT_POINT,
            data_length: junction_data.len() as u16,
            reserved: 0,
            data: junction_data,
        };
        
        let reparse_data = Self::serialize_reparse_point(&reparse_point);
        
        Ok(Attribute {
            type_code: ATTR_TYPE_REPARSE_POINT,
            name: String::new(),
            flags: 0,
            content: AttributeContent::Resident(reparse_data),
        })
    }
    
    fn create_symlink_data(target_path: &str) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Substitute name offset
        data.extend_from_slice(&0u16.to_le_bytes());
        // Substitute name length
        let sub_name_len = (target_path.len() * 2) as u16;
        data.extend_from_slice(&sub_name_len.to_le_bytes());
        // Print name offset
        let print_offset = 4 + sub_name_len;
        data.extend_from_slice(&print_offset.to_le_bytes());
        // Print name length
        data.extend_from_slice(&sub_name_len.to_le_bytes());
        // Flags (0 = absolute path)
        data.extend_from_slice(&0u32.to_le_bytes());
        
        // Substitute name (UTF-16)
        for ch in target_path.chars() {
            data.extend_from_slice(&(ch as u16).to_le_bytes());
        }
        
        // Print name (same as substitute name)
        for ch in target_path.chars() {
            data.extend_from_slice(&(ch as u16).to_le_bytes());
        }
        
        data
    }
    
    fn create_junction_data(target_path: &str) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Substitute name offset
        data.extend_from_slice(&0u16.to_le_bytes());
        // Substitute name length
        let sub_name_len = (target_path.len() * 2) as u16;
        data.extend_from_slice(&sub_name_len.to_le_bytes());
        // Print name offset
        let print_offset = 4 + sub_name_len;
        data.extend_from_slice(&print_offset.to_le_bytes());
        // Print name length
        data.extend_from_slice(&sub_name_len.to_le_bytes());
        
        // Substitute name (UTF-16)
        for ch in target_path.chars() {
            data.extend_from_slice(&(ch as u16).to_le_bytes());
        }
        
        // Print name
        for ch in target_path.chars() {
            data.extend_from_slice(&(ch as u16).to_le_bytes());
        }
        
        data
    }
    
    fn serialize_reparse_point(reparse: &ReparsePoint) -> Vec<u8> {
        let mut data = Vec::new();
        
        // Reparse tag
        data.extend_from_slice(&reparse.tag.to_le_bytes());
        // Data length
        data.extend_from_slice(&reparse.data_length.to_le_bytes());
        // Reserved
        data.extend_from_slice(&reparse.reserved.to_le_bytes());
        // Data
        data.extend_from_slice(&reparse.data);
        
        data
    }
    
    fn parse_reparse_point(data: &[u8]) -> Result<ReparsePoint, &'static str> {
        if data.len() < 8 {
            return Err("Reparse point data too small");
        }
        
        let tag = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let data_length = u16::from_le_bytes([data[4], data[5]]);
        let reserved = u16::from_le_bytes([data[6], data[7]]);
        
        let mut reparse_data = Vec::new();
        if 8 + data_length as usize <= data.len() {
            reparse_data.extend_from_slice(&data[8..8 + data_length as usize]);
        }
        
        Ok(ReparsePoint {
            tag,
            data_length,
            reserved,
            data: reparse_data,
        })
    }
    
    fn parse_symlink_data(data: &[u8]) -> Result<SymbolicLinkData, &'static str> {
        if data.len() < 12 {
            return Err("Symlink data too small");
        }
        
        let sub_name_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
        let sub_name_length = u16::from_le_bytes([data[2], data[3]]) as usize;
        let print_name_offset = u16::from_le_bytes([data[4], data[5]]) as usize;
        let print_name_length = u16::from_le_bytes([data[6], data[7]]) as usize;
        let flags = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        
        // Parse substitute name (target path)
        let mut target_path = String::new();
        if 12 + sub_name_offset + sub_name_length <= data.len() {
            for i in (12 + sub_name_offset..12 + sub_name_offset + sub_name_length).step_by(2) {
                if i + 1 < data.len() {
                    let ch = u16::from_le_bytes([data[i], data[i + 1]]);
                    if let Some(c) = char::from_u32(ch as u32) {
                        target_path.push(c);
                    }
                }
            }
        }
        
        // Parse print name
        let mut print_name = String::new();
        if 12 + print_name_offset + print_name_length <= data.len() {
            for i in (12 + print_name_offset..12 + print_name_offset + print_name_length).step_by(2) {
                if i + 1 < data.len() {
                    let ch = u16::from_le_bytes([data[i], data[i + 1]]);
                    if let Some(c) = char::from_u32(ch as u32) {
                        print_name.push(c);
                    }
                }
            }
        }
        
        Ok(SymbolicLinkData {
            target_path,
            print_name,
            flags,
        })
    }
    
    fn parse_junction_data(data: &[u8]) -> Result<String, &'static str> {
        if data.len() < 8 {
            return Err("Junction data too small");
        }
        
        let sub_name_offset = u16::from_le_bytes([data[0], data[1]]) as usize;
        let sub_name_length = u16::from_le_bytes([data[2], data[3]]) as usize;
        
        // Parse substitute name (target path)
        let mut target_path = String::new();
        if 8 + sub_name_offset + sub_name_length <= data.len() {
            for i in (8 + sub_name_offset..8 + sub_name_offset + sub_name_length).step_by(2) {
                if i + 1 < data.len() {
                    let ch = u16::from_le_bytes([data[i], data[i + 1]]);
                    if let Some(c) = char::from_u32(ch as u32) {
                        target_path.push(c);
                    }
                }
            }
        }
        
        Ok(target_path)
    }
}

// Extended Attributes Support
impl NtfsFileSystem {
    // Set extended attribute
    pub fn set_extended_attribute(&mut self, path: &str, name: &str, value: &[u8]) -> Result<(), &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Create or update EA attribute
        let ea_attr = self.create_ea_attribute(name, value)?;
        
        // Find existing EA attribute
        let mut found = false;
        for attr in &mut entry.attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_EA {
                *attr = ea_attr.clone();
                found = true;
                break;
            }
        }
        
        if !found {
            entry.attributes.push(ea_attr);
        }
        
        // Update modification time
        let timestamp = Self::get_current_timestamp();
        entry.modified_time = timestamp;
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        Ok(())
    }
    
    // Get extended attribute
    pub fn get_extended_attribute(&mut self, path: &str, name: &str) -> Result<Vec<u8>, &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        let entry = self.mft.read_entry(entry_num)?;
        
        // Find EA attribute
        let ea_attr = entry.get_attribute(super::attributes::ATTR_TYPE_EA)
            .ok_or("No extended attributes")?;
        
        // Parse EA data
        if let AttributeContent::Resident(data) = &ea_attr.content {
            self.parse_ea_value(data, name)
        } else {
            Err("EA data is non-resident")
        }
    }
    
    fn create_ea_attribute(&self, name: &str, value: &[u8]) -> Result<Attribute, &'static str> {
        let mut data = Vec::new();
        
        // EA header
        data.extend_from_slice(&0u32.to_le_bytes()); // Next entry offset (0 = last)
        data.push(0); // Flags
        data.push(name.len() as u8); // Name length
        let value_len = value.len() as u16;
        data.extend_from_slice(&value_len.to_le_bytes()); // Value length
        
        // Name (ASCII)
        data.extend_from_slice(name.as_bytes());
        
        // Padding to align value
        while data.len() % 4 != 0 {
            data.push(0);
        }
        
        // Value
        data.extend_from_slice(value);
        
        Ok(Attribute {
            type_code: super::attributes::ATTR_TYPE_EA,
            name: String::new(),
            flags: 0,
            content: AttributeContent::Resident(data),
        })
    }
    
    fn parse_ea_value(&self, data: &[u8], name: &str) -> Result<Vec<u8>, &'static str> {
        let mut offset = 0;
        
        while offset + 8 <= data.len() {
            // Parse EA entry
            let next_offset = u32::from_le_bytes([
                data[offset], data[offset + 1],
                data[offset + 2], data[offset + 3],
            ]) as usize;
            
            let name_len = data[offset + 5] as usize;
            let value_len = u16::from_le_bytes([data[offset + 6], data[offset + 7]]) as usize;
            
            if offset + 8 + name_len <= data.len() {
                let ea_name = core::str::from_utf8(&data[offset + 8..offset + 8 + name_len])
                    .map_err(|_| "Invalid EA name")?;
                
                if ea_name == name {
                    // Found the EA
                    let value_offset = offset + 8 + name_len;
                    // Align to 4 bytes
                    let value_offset = (value_offset + 3) & !3;
                    
                    if value_offset + value_len <= data.len() {
                        return Ok(data[value_offset..value_offset + value_len].to_vec());
                    }
                }
            }
            
            if next_offset == 0 {
                break;
            }
            offset = next_offset;
        }
        
        Err("Extended attribute not found")
    }
}

// Compression Support
impl NtfsFileSystem {
    // Enable compression on a file
    pub fn enable_compression(&mut self, path: &str) -> Result<(), &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Set compression flag
        entry.file_attributes |= 0x800; // FILE_ATTRIBUTE_COMPRESSED
        
        // Update standard information attribute
        for attr in &mut entry.attributes {
            if attr.type_code == ATTR_TYPE_STANDARD_INFO {
                if let AttributeContent::Resident(ref mut data) = attr.content {
                    if data.len() >= 36 {
                        let mut attrs = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
                        attrs |= 0x800;
                        data[32..36].copy_from_slice(&attrs.to_le_bytes());
                    }
                }
                break;
            }
        }
        
        // Mark data attribute as compressed
        for attr in &mut entry.attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_DATA {
                attr.flags |= 0x0001; // ATTR_IS_COMPRESSED
                break;
            }
        }
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        Ok(())
    }
    
    // Disable compression on a file
    pub fn disable_compression(&mut self, path: &str) -> Result<(), &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Clear compression flag
        entry.file_attributes &= !0x800;
        
        // Update standard information attribute
        for attr in &mut entry.attributes {
            if attr.type_code == ATTR_TYPE_STANDARD_INFO {
                if let AttributeContent::Resident(ref mut data) = attr.content {
                    if data.len() >= 36 {
                        let mut attrs = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
                        attrs &= !0x800;
                        data[32..36].copy_from_slice(&attrs.to_le_bytes());
                    }
                }
                break;
            }
        }
        
        // Clear compression flag on data attribute
        for attr in &mut entry.attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_DATA {
                attr.flags &= !0x0001;
                break;
            }
        }
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        Ok(())
    }
}

// Sparse File Support
impl NtfsFileSystem {
    // Mark a file as sparse
    pub fn set_sparse(&mut self, path: &str) -> Result<(), &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Set sparse flag
        entry.file_attributes |= 0x200; // FILE_ATTRIBUTE_SPARSE_FILE
        
        // Update standard information attribute
        for attr in &mut entry.attributes {
            if attr.type_code == ATTR_TYPE_STANDARD_INFO {
                if let AttributeContent::Resident(ref mut data) = attr.content {
                    if data.len() >= 36 {
                        let mut attrs = u32::from_le_bytes([data[32], data[33], data[34], data[35]]);
                        attrs |= 0x200;
                        data[32..36].copy_from_slice(&attrs.to_le_bytes());
                    }
                }
                break;
            }
        }
        
        // Mark data attribute as sparse
        for attr in &mut entry.attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_DATA {
                attr.flags |= 0x8000; // ATTR_IS_SPARSE
                break;
            }
        }
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        Ok(())
    }
    
    // Allocate a range in a sparse file
    pub fn allocate_sparse_range(&mut self, path: &str, offset: u64, length: u64) -> Result<(), &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Find data attribute
        for attr in &mut entry.attributes {
            if attr.type_code == super::attributes::ATTR_TYPE_DATA {
                if let AttributeContent::NonResident(ref mut non_res) = attr.content {
                    // Calculate clusters needed
                    let start_cluster = offset / self.cluster_size as u64;
                    let end_cluster = (offset + length + self.cluster_size as u64 - 1) / self.cluster_size as u64;
                    let clusters_needed = end_cluster - start_cluster;
                    
                    // Allocate clusters
                    let allocated = self.allocate_clusters(clusters_needed)?;
                    
                    // Add to data runs
                    // This is simplified - real implementation would merge runs
                    non_res.data_runs.push(super::attributes::DataRun {
                        length: clusters_needed,
                        start_lcn: allocated[0],
                    });
                }
                break;
            }
        }
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        Ok(())
    }
}