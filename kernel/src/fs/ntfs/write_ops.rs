// NTFS Write Operations Implementation
use alloc::vec::Vec;
use alloc::string::String;
use super::{NtfsFileSystem, MFT_ENTRY_ROOT, DirectoryEntry};
use super::mft::{MftEntry, MFT_ENTRY_IS_DIRECTORY};
use super::attributes::{
    self, Attribute, AttributeContent, NonResidentAttribute, DataRun,
    ATTR_TYPE_DATA, ATTR_TYPE_INDEX_ROOT, ATTR_TYPE_STANDARD_INFO,
    create_standard_info_attribute, create_file_name_attribute,
    create_data_attribute, update_attribute_data,
};
use crate::drivers::disk::DiskDriver;

impl NtfsFileSystem {
    // Main write operation entry point
    pub fn write_file_impl(&mut self, path: &str, data: &[u8]) -> Result<(), &'static str> {
        // Parse path
        let components: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid path");
        }
        
        let file_name = components.last().unwrap();
        let parent_path = &components[..components.len() - 1];
        
        // Find parent directory
        let parent_entry_num = if parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&parent_path.join("\\"))?
        };
        
        // Check if file exists
        match self.find_file_in_directory(parent_entry_num, file_name) {
            Ok(entry_num) => {
                // Update existing file
                self.update_file_data(entry_num, data)
            }
            Err(_) => {
                // Create new file
                self.create_file(parent_entry_num, file_name, data)
            }
        }
    }
    
    // Create a new file
    pub fn create_file(&mut self, parent_entry: u64, name: &str, data: &[u8]) -> Result<(), &'static str> {
        // Begin transaction if journal is available
        let transaction_id = if let Some(ref journal) = self.journal {
            Some(journal.begin_transaction())
        } else {
            None
        };
        
        // Allocate new MFT entry
        let (entry_num, mut entry) = self.mft.allocate_entry(&mut *self.disk)?;
        
        // Get current timestamp
        let timestamp = Self::get_current_timestamp();
        
        // Add standard information attribute
        let std_info = create_standard_info_attribute(
            timestamp,
            timestamp,
            timestamp,
            0x80, // Normal file
        );
        entry.attributes.push(std_info);
        
        // Add file name attribute
        let file_name_attr = create_file_name_attribute(
            parent_entry,
            name,
            false,
        );
        entry.attributes.push(file_name_attr);
        
        // Add data attribute
        if data.len() > 0 {
            // Determine if data should be resident or non-resident
            if data.len() <= 700 {
                // Resident data
                let data_attr = create_data_attribute(None, data.to_vec());
                entry.attributes.push(data_attr);
            } else {
                // Non-resident data - allocate clusters
                let clusters_needed = (data.len() + self.cluster_size as usize - 1) / self.cluster_size as usize;
                let allocated_clusters = self.allocate_clusters(clusters_needed as u64)?;
                
                // Write data to clusters
                self.write_clusters(&allocated_clusters, data)?;
                
                // Create non-resident data attribute
                let data_attr = self.create_non_resident_attribute(&allocated_clusters, data.len());
                entry.attributes.push(data_attr);
            }
        }
        
        // Update entry metadata
        entry.created_time = timestamp;
        entry.modified_time = timestamp;
        entry.accessed_time = timestamp;
        entry.file_attributes = 0x80;
        
        // Write MFT entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        // Add to parent directory index
        self.add_to_directory_index(parent_entry, entry_num, name)?;
        
        // Commit transaction if journal is available
        if let Some(ref journal) = self.journal {
            if let Some(tid) = transaction_id {
                journal.commit_transaction(tid)?;
            }
        }
        
        Ok(())
    }
    
    // Update existing file data
    pub fn update_file_data(&mut self, entry_num: u64, new_data: &[u8]) -> Result<(), &'static str> {
        // Begin transaction
        let transaction_id = if let Some(ref journal) = self.journal {
            Some(journal.begin_transaction())
        } else {
            None
        };
        
        // Read existing entry
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Find and update data attribute
        let mut found = false;
        for attr in &mut entry.attributes {
            if attr.type_code == ATTR_TYPE_DATA {
                // Update the data
                update_attribute_data(attr, new_data.to_vec())?;
                
                // Handle cluster allocation for non-resident data
                if let AttributeContent::NonResident(ref mut non_res) = attr.content {
                    let clusters_needed = (new_data.len() + self.cluster_size as usize - 1) / self.cluster_size as usize;
                    
                    // Deallocate old clusters if shrinking
                    if clusters_needed < non_res.data_runs[0].length as usize {
                        let clusters_to_free: Vec<u64> = ((clusters_needed as u64)..non_res.data_runs[0].length)
                            .map(|i| non_res.data_runs[0].start_lcn + i)
                            .collect();
                        self.deallocate_clusters(&clusters_to_free)?;
                    }
                    // Allocate new clusters if growing
                    else if clusters_needed > non_res.data_runs[0].length as usize {
                        let additional = clusters_needed as u64 - non_res.data_runs[0].length;
                        let new_clusters = self.allocate_clusters(additional)?;
                        // Update data runs
                        // This is simplified - real implementation would handle complex runs
                    }
                    
                    // Write data to clusters
                    let clusters: Vec<u64> = (0..clusters_needed as u64)
                        .map(|i| non_res.data_runs[0].start_lcn + i)
                        .collect();
                    self.write_clusters(&clusters, new_data)?;
                }
                
                found = true;
                break;
            }
        }
        
        if !found {
            // No data attribute, create one
            let data_attr = create_data_attribute(None, new_data.to_vec());
            entry.attributes.push(data_attr);
        }
        
        // Update timestamps
        let timestamp = Self::get_current_timestamp();
        entry.modified_time = timestamp;
        entry.accessed_time = timestamp;
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        // Commit transaction
        if let Some(ref journal) = self.journal {
            if let Some(tid) = transaction_id {
                journal.commit_transaction(tid)?;
            }
        }
        
        Ok(())
    }
    
    // Create a new directory
    pub fn create_directory_impl(&mut self, path: &str) -> Result<(), &'static str> {
        // Parse path
        let components: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid path");
        }
        
        let dir_name = components.last().unwrap();
        let parent_path = &components[..components.len() - 1];
        
        // Find parent directory
        let parent_entry_num = if parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&parent_path.join("\\"))?
        };
        
        // Check if directory already exists
        if self.find_file_in_directory(parent_entry_num, dir_name).is_ok() {
            return Err("Directory already exists");
        }
        
        // Allocate new MFT entry
        let (entry_num, mut entry) = self.mft.allocate_entry(&mut *self.disk)?;
        
        // Set directory flag
        entry.header.flags |= MFT_ENTRY_IS_DIRECTORY;
        
        // Get current timestamp
        let timestamp = Self::get_current_timestamp();
        
        // Add standard information attribute
        let std_info = create_standard_info_attribute(
            timestamp,
            timestamp,
            timestamp,
            0x10000000, // Directory
        );
        entry.attributes.push(std_info);
        
        // Add file name attribute
        let file_name_attr = create_file_name_attribute(
            parent_entry_num,
            dir_name,
            true,
        );
        entry.attributes.push(file_name_attr);
        
        // Add index root attribute for directory entries
        let index_root = self.create_index_root_attribute()?;
        entry.attributes.push(index_root);
        
        // Update entry metadata
        entry.created_time = timestamp;
        entry.modified_time = timestamp;
        entry.accessed_time = timestamp;
        entry.file_attributes = 0x10000000; // Directory
        
        // Write MFT entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        // Add to parent directory index
        self.add_to_directory_index(parent_entry_num, entry_num, dir_name)?;
        
        Ok(())
    }
    
    // Delete a file or directory
    pub fn delete_file_impl(&mut self, path: &str) -> Result<(), &'static str> {
        // Parse path
        let components: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid path");
        }
        
        let file_name = components.last().unwrap();
        let parent_path = &components[..components.len() - 1];
        
        // Find parent directory
        let parent_entry_num = if parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&parent_path.join("\\"))?
        };
        
        // Find file in directory
        let entry_num = self.find_file_in_directory(parent_entry_num, file_name)?;
        
        // Read entry to deallocate resources
        let entry = self.mft.read_entry(entry_num)?;
        
        // If it's a directory, ensure it's empty
        if entry.is_directory() {
            let entries = self.read_directory_entries(&entry)?;
            if !entries.is_empty() {
                return Err("Directory not empty");
            }
        }
        
        // Deallocate clusters if non-resident data
        for attr in &entry.attributes {
            if attr.type_code == ATTR_TYPE_DATA {
                if let AttributeContent::NonResident(ref non_res) = attr.content {
                    // Deallocate all data clusters
                    let mut clusters_to_free = Vec::new();
                    for run in &non_res.data_runs {
                        for i in 0..run.length {
                            clusters_to_free.push(run.start_lcn + i);
                        }
                    }
                    self.deallocate_clusters(&clusters_to_free)?;
                }
            }
        }
        
        // Remove from parent directory index
        self.remove_from_directory_index(parent_entry_num, entry_num, file_name)?;
        
        // Deallocate MFT entry
        self.mft.deallocate_entry(&mut *self.disk, entry_num)?;
        
        Ok(())
    }
    
    // Rename a file or directory
    pub fn rename_file(&mut self, old_path: &str, new_name: &str) -> Result<(), &'static str> {
        // Parse old path
        let components: Vec<&str> = old_path.split('\\').filter(|s| !s.is_empty()).collect();
        if components.is_empty() {
            return Err("Invalid path");
        }
        
        let old_name = components.last().unwrap();
        let parent_path = &components[..components.len() - 1];
        
        // Find parent directory
        let parent_entry_num = if parent_path.is_empty() {
            MFT_ENTRY_ROOT
        } else {
            self.find_entry_by_path(&parent_path.join("\\"))?
        };
        
        // Find file in directory
        let entry_num = self.find_file_in_directory(parent_entry_num, old_name)?;
        
        // Check if new name already exists
        if self.find_file_in_directory(parent_entry_num, new_name).is_ok() {
            return Err("Target name already exists");
        }
        
        // Read and update entry
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Store is_directory before borrowing mutably
        let is_dir = entry.is_directory();
        
        // Update file name attribute
        for attr in &mut entry.attributes {
            if attr.type_code == attributes::ATTR_TYPE_FILE_NAME {
                // Update the file name in the attribute
                *attr = create_file_name_attribute(
                    parent_entry_num,
                    new_name,
                    is_dir,
                );
                break;
            }
        }
        
        // Update modification time
        let timestamp = Self::get_current_timestamp();
        entry.modified_time = timestamp;
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        // Update parent directory index
        self.remove_from_directory_index(parent_entry_num, entry_num, old_name)?;
        self.add_to_directory_index(parent_entry_num, entry_num, new_name)?;
        
        Ok(())
    }
    
    // Helper function to find entry by path
    pub fn find_entry_by_path(&mut self, path: &str) -> Result<u64, &'static str> {
        let components: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        
        let mut current_entry = MFT_ENTRY_ROOT;
        
        for component in components {
            let entry = self.mft.read_entry(current_entry)?;
            current_entry = self.find_in_directory(&entry, component, true)?;
        }
        
        Ok(current_entry)
    }
    
    // Helper function to find file in directory
    pub fn find_file_in_directory(&mut self, dir_entry: u64, name: &str) -> Result<u64, &'static str> {
        let entry = self.mft.read_entry(dir_entry)?;
        self.find_in_directory(&entry, name, false)
    }
    
    // Allocate clusters for data storage
    pub fn allocate_clusters(&mut self, count: u64) -> Result<Vec<u64>, &'static str> {
        let mut bitmap = self.cluster_bitmap.lock();
        bitmap.allocate_clusters(count)
            .ok_or("Not enough free clusters")
    }
    
    // Deallocate clusters
    pub fn deallocate_clusters(&mut self, clusters: &[u64]) -> Result<(), &'static str> {
        let mut bitmap = self.cluster_bitmap.lock();
        bitmap.deallocate_clusters(clusters);
        Ok(())
    }
    
    // Write data to clusters
    pub fn write_clusters(&mut self, clusters: &[u64], data: &[u8]) -> Result<(), &'static str> {
        let cluster_size = self.cluster_size as usize;
        let sectors_per_cluster = self.boot_sector.sectors_per_cluster as u32;
        
        for (i, cluster) in clusters.iter().enumerate() {
            let offset = i * cluster_size;
            let end = core::cmp::min(offset + cluster_size, data.len());
            
            let mut cluster_data = vec![0u8; cluster_size];
            cluster_data[..end - offset].copy_from_slice(&data[offset..end]);
            
            let sector = cluster * sectors_per_cluster as u64;
            self.disk.write_sectors(sector, sectors_per_cluster, &cluster_data)
                .map_err(|_| "Failed to write cluster")?;
        }
        
        Ok(())
    }
    
    // Create non-resident attribute for large data
    pub fn create_non_resident_attribute(&self, clusters: &[u64], data_size: usize) -> Attribute {
        // Create data runs from allocated clusters
        let mut data_runs = Vec::new();
        
        if !clusters.is_empty() {
            // Simplified: assume contiguous clusters
            // Real implementation would handle fragmentation
            data_runs.push(DataRun {
                length: clusters.len() as u64,
                start_lcn: clusters[0],
            });
        }
        
        let non_res = NonResidentAttribute {
            start_vcn: 0,
            last_vcn: clusters.len() as u64 - 1,
            allocated_size: (clusters.len() * self.cluster_size as usize) as u64,
            real_size: data_size as u64,
            initialized_size: data_size as u64,
            data_runs,
        };
        
        Attribute {
            type_code: ATTR_TYPE_DATA,
            name: String::new(),
            flags: 0,
            content: AttributeContent::NonResident(non_res),
        }
    }
    
    // Create index root attribute for directories
    pub fn create_index_root_attribute(&self) -> Result<Attribute, &'static str> {
        // Create INDEX_ROOT attribute for directory
        let mut data = vec![0u8; 64];
        
        // Index type (0x30 = file name)
        data[0..4].copy_from_slice(&0x30u32.to_le_bytes());
        // Collation rule
        data[4..8].copy_from_slice(&1u32.to_le_bytes());
        // Index block size
        data[8..12].copy_from_slice(&4096u32.to_le_bytes());
        // Clusters per index block
        data[12] = 1;
        
        Ok(Attribute {
            type_code: ATTR_TYPE_INDEX_ROOT,
            name: String::from("$I30"),
            flags: 0,
            content: AttributeContent::Resident(data),
        })
    }
    
    // Add entry to directory index
    pub fn add_to_directory_index(&mut self, dir_entry: u64, file_entry: u64, name: &str) -> Result<(), &'static str> {
        // This is a simplified implementation
        // Real implementation would update the B+ tree index structure
        
        // Read directory entry
        let mut dir = self.mft.read_entry(dir_entry)?;
        
        // Find INDEX_ROOT attribute
        for attr in &mut dir.attributes {
            if attr.type_code == ATTR_TYPE_INDEX_ROOT {
                // Add index entry to the attribute
                // This would involve complex B+ tree operations
                break;
            }
        }
        
        // Write updated directory entry
        self.mft.write_entry(&mut *self.disk, dir_entry, &dir)?;
        
        Ok(())
    }
    
    // Remove entry from directory index
    pub fn remove_from_directory_index(&mut self, dir_entry: u64, file_entry: u64, name: &str) -> Result<(), &'static str> {
        // This is a simplified implementation
        // Real implementation would update the B+ tree index structure
        
        // Read directory entry
        let mut dir = self.mft.read_entry(dir_entry)?;
        
        // Find INDEX_ROOT attribute
        for attr in &mut dir.attributes {
            if attr.type_code == ATTR_TYPE_INDEX_ROOT {
                // Remove index entry from the attribute
                // This would involve complex B+ tree operations
                break;
            }
        }
        
        // Write updated directory entry
        self.mft.write_entry(&mut *self.disk, dir_entry, &dir)?;
        
        Ok(())
    }
    
    // Get current Windows timestamp
    pub fn get_current_timestamp() -> u64 {
        // This is a placeholder - real implementation would get system time
        // Windows timestamps are 100-nanosecond intervals since 1601-01-01
        0x01D7C4F0A0000000u64
    }
    
    // Set file attributes
    pub fn set_file_attributes(&mut self, path: &str, attributes: u32) -> Result<(), &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        
        // Read and update entry
        let mut entry = self.mft.read_entry(entry_num)?;
        entry.file_attributes = attributes;
        
        // Update standard information attribute
        for attr in &mut entry.attributes {
            if attr.type_code == ATTR_TYPE_STANDARD_INFO {
                if let AttributeContent::Resident(ref mut data) = attr.content {
                    if data.len() >= 36 {
                        data[32..36].copy_from_slice(&attributes.to_le_bytes());
                    }
                }
                break;
            }
        }
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        Ok(())
    }
    
    // Truncate file to specified size
    pub fn truncate_file(&mut self, path: &str, new_size: u64) -> Result<(), &'static str> {
        // Find file
        let entry_num = self.find_entry_by_path(path)?;
        
        // Read entry
        let mut entry = self.mft.read_entry(entry_num)?;
        
        // Find and update data attribute
        for attr in &mut entry.attributes {
            if attr.type_code == ATTR_TYPE_DATA {
                attributes::resize_attribute(attr, new_size)?;
                
                // Handle cluster deallocation if shrinking
                if let AttributeContent::NonResident(ref non_res) = attr.content {
                    let old_clusters = ((non_res.allocated_size + self.cluster_size as u64 - 1) / self.cluster_size as u64) as usize;
                    let new_clusters = ((new_size + self.cluster_size as u64 - 1) / self.cluster_size as u64) as usize;
                    
                    if new_clusters < old_clusters {
                        // Deallocate excess clusters
                        let clusters_to_free: Vec<u64> = ((new_clusters as u64)..old_clusters as u64)
                            .map(|i| non_res.data_runs[0].start_lcn + i)
                            .collect();
                        self.deallocate_clusters(&clusters_to_free)?;
                    }
                }
                break;
            }
        }
        
        // Update modification time
        let timestamp = Self::get_current_timestamp();
        entry.modified_time = timestamp;
        
        // Write updated entry
        self.mft.write_entry(&mut *self.disk, entry_num, &entry)?;
        
        Ok(())
    }
}