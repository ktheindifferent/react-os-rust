// NTFS (New Technology File System) Implementation
pub mod boot_sector;
pub mod mft;
pub mod attributes;
pub mod index;
pub mod security;
pub mod journal;
pub mod write_ops;
pub mod advanced;

use alloc::vec::Vec;
use alloc::string::String;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use alloc::vec;
use spin::Mutex;
use crate::drivers::disk::DiskDriver;
use self::journal::JournalManager;

// NTFS Constants
pub const NTFS_SIGNATURE: &[u8; 8] = b"NTFS    ";
pub const SECTOR_SIZE: usize = 512;
pub const MFT_ENTRY_SIZE: usize = 1024;

// NTFS System Files (first 16 MFT entries)
pub const MFT_ENTRY_MFT: u64 = 0;        // $MFT
pub const MFT_ENTRY_MFTMIRR: u64 = 1;    // $MFTMirr
pub const MFT_ENTRY_LOGFILE: u64 = 2;    // $LogFile
pub const MFT_ENTRY_VOLUME: u64 = 3;     // $Volume
pub const MFT_ENTRY_ATTRDEF: u64 = 4;    // $AttrDef
pub const MFT_ENTRY_ROOT: u64 = 5;       // . (root directory)
pub const MFT_ENTRY_BITMAP: u64 = 6;     // $Bitmap
pub const MFT_ENTRY_BOOT: u64 = 7;       // $Boot
pub const MFT_ENTRY_BADCLUS: u64 = 8;    // $BadClus
pub const MFT_ENTRY_SECURE: u64 = 9;     // $Secure
pub const MFT_ENTRY_UPCASE: u64 = 10;    // $UpCase
pub const MFT_ENTRY_EXTEND: u64 = 11;    // $Extend

// File Attributes
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileAttributes {
    ReadOnly = 0x0001,
    Hidden = 0x0002,
    System = 0x0004,
    Archive = 0x0020,
    Device = 0x0040,
    Normal = 0x0080,
    Temporary = 0x0100,
    SparseFile = 0x0200,
    ReparsePoint = 0x0400,
    Compressed = 0x0800,
    Offline = 0x1000,
    NotContentIndexed = 0x2000,
    Encrypted = 0x4000,
    Directory = 0x10000000,
    IndexView = 0x20000000,
}

// NTFS File System Structure
pub struct NtfsFileSystem {
    disk: Box<dyn DiskDriver>,
    boot_sector: boot_sector::NtfsBootSector,
    mft: mft::MasterFileTable,
    cluster_size: u32,
    mft_start_lcn: u64,
    volume_info: VolumeInfo,
    journal: Option<Box<JournalManager>>,
    cluster_bitmap: Mutex<ClusterBitmap>,
}

// Cluster allocation bitmap
pub struct ClusterBitmap {
    data: Vec<u8>,
    total_clusters: u64,
    free_clusters: u64,
}

impl ClusterBitmap {
    pub fn new(total_clusters: u64) -> Self {
        let bytes_needed = (total_clusters + 7) / 8;
        Self {
            data: vec![0u8; bytes_needed as usize],
            total_clusters,
            free_clusters: total_clusters,
        }
    }
    
    pub fn allocate_clusters(&mut self, count: u64) -> Option<Vec<u64>> {
        if count > self.free_clusters {
            return None;
        }
        
        let mut allocated = Vec::new();
        let mut current = 0u64;
        
        while allocated.len() < count as usize && current < self.total_clusters {
            let byte_idx = (current / 8) as usize;
            let bit_idx = (current % 8) as u8;
            
            if byte_idx < self.data.len() {
                if (self.data[byte_idx] & (1 << bit_idx)) == 0 {
                    // Cluster is free, allocate it
                    self.data[byte_idx] |= 1 << bit_idx;
                    allocated.push(current);
                    self.free_clusters -= 1;
                }
            }
            current += 1;
        }
        
        if allocated.len() == count as usize {
            Some(allocated)
        } else {
            // Rollback if we couldn't allocate enough
            for cluster in &allocated {
                let byte_idx = (cluster / 8) as usize;
                let bit_idx = (cluster % 8) as u8;
                self.data[byte_idx] &= !(1 << bit_idx);
                self.free_clusters += 1;
            }
            None
        }
    }
    
    pub fn deallocate_clusters(&mut self, clusters: &[u64]) {
        for cluster in clusters {
            let byte_idx = (cluster / 8) as usize;
            let bit_idx = (cluster % 8) as u8;
            
            if byte_idx < self.data.len() {
                if (self.data[byte_idx] & (1 << bit_idx)) != 0 {
                    self.data[byte_idx] &= !(1 << bit_idx);
                    self.free_clusters += 1;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct VolumeInfo {
    pub volume_name: String,
    pub serial_number: u64,
    pub total_sectors: u64,
    pub free_clusters: u64,
    pub total_clusters: u64,
}

impl NtfsFileSystem {
    pub fn new(mut disk: Box<dyn DiskDriver>) -> Result<Self, &'static str> {
        // Read boot sector
        let mut boot_data = vec![0u8; SECTOR_SIZE];
        disk.read_sectors(0, 1, &mut boot_data)
            .map_err(|_| "Failed to read boot sector")?;
        
        let boot_sector = boot_sector::NtfsBootSector::parse(&boot_data)?;
        
        // Verify NTFS signature
        if &boot_sector.oem_id != NTFS_SIGNATURE {
            return Err("Not an NTFS file system");
        }
        
        // Calculate cluster size
        let cluster_size = boot_sector.sectors_per_cluster as u32 * boot_sector.bytes_per_sector as u32;
        
        // Calculate MFT location
        let mft_start_lcn = boot_sector.mft_lcn;
        let mft_start_sector = mft_start_lcn * boot_sector.sectors_per_cluster as u64;
        
        // Load MFT
        let mft = mft::MasterFileTable::new(&mut *disk, mft_start_sector, &boot_sector)?;
        
        // Get volume information
        let volume_info = Self::read_volume_info(&mut *disk, &mft)?;
        
        // Initialize cluster bitmap
        let total_clusters = boot_sector.total_sectors / boot_sector.sectors_per_cluster as u64;
        let cluster_bitmap = Mutex::new(ClusterBitmap::new(total_clusters));
        
        // Journal initialization would go here
        let journal = None;
        
        Ok(Self {
            disk,
            boot_sector,
            mft,
            cluster_size,
            mft_start_lcn,
            volume_info,
            journal,
            cluster_bitmap,
        })
    }
    
    fn read_volume_info(disk: &mut dyn DiskDriver, mft: &mft::MasterFileTable) -> Result<VolumeInfo, &'static str> {
        // Read $Volume entry
        let volume_entry = mft.read_entry(MFT_ENTRY_VOLUME)?;
        
        // Parse volume attributes
        // Simplified for now
        Ok(VolumeInfo {
            volume_name: String::from("NTFS Volume"),
            serial_number: 0,
            total_sectors: 0,
            free_clusters: 0,
            total_clusters: 0,
        })
    }
    
    pub fn read_file(&mut self, path: &str) -> Result<Vec<u8>, &'static str> {
        // Parse path
        let components: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        
        if components.is_empty() {
            return Err("Invalid path");
        }
        
        // Start from root directory
        let mut current_entry = MFT_ENTRY_ROOT;
        
        // Navigate through path
        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            
            // Read current directory
            let entry = self.mft.read_entry(current_entry)?;
            
            if !is_last {
                // Find subdirectory
                current_entry = self.find_in_directory(&entry, component, true)?;
            } else {
                // Find file
                current_entry = self.find_in_directory(&entry, component, false)?;
            }
        }
        
        // Read file data
        self.read_file_data(current_entry)
    }
    
    fn find_in_directory(&mut self, dir_entry: &mft::MftEntry, name: &str, is_dir: bool) -> Result<u64, &'static str> {
        // Read index entries from directory
        let index_entries = self.read_directory_entries(dir_entry)?;
        
        // Search for matching name
        for entry in index_entries {
            if entry.name.eq_ignore_ascii_case(name) {
                if is_dir && !entry.is_directory {
                    return Err("Not a directory");
                }
                if !is_dir && entry.is_directory {
                    return Err("Is a directory");
                }
                return Ok(entry.mft_reference);
            }
        }
        
        Err("File not found")
    }
    
    fn read_directory_entries(&mut self, dir_entry: &mft::MftEntry) -> Result<Vec<DirectoryEntry>, &'static str> {
        let mut entries = Vec::new();
        
        // Read INDEX_ROOT attribute
        if let Some(index_root) = dir_entry.get_attribute(attributes::ATTR_TYPE_INDEX_ROOT) {
            // Parse index entries
            let index_data = self.read_attribute_data(index_root)?;
            entries.extend(self.parse_index_entries(&index_data)?);
        }
        
        // Read INDEX_ALLOCATION attribute for large directories
        if let Some(index_alloc) = dir_entry.get_attribute(attributes::ATTR_TYPE_INDEX_ALLOCATION) {
            let alloc_data = self.read_attribute_data(index_alloc)?;
            entries.extend(self.parse_index_allocation(&alloc_data)?);
        }
        
        Ok(entries)
    }
    
    fn read_file_data(&mut self, mft_entry_num: u64) -> Result<Vec<u8>, &'static str> {
        let entry = self.mft.read_entry(mft_entry_num)?;
        
        // Find DATA attribute
        let data_attr = entry.get_attribute(attributes::ATTR_TYPE_DATA)
            .ok_or("No data attribute")?;
        
        self.read_attribute_data(data_attr)
    }
    
    fn read_attribute_data(&mut self, attr: &attributes::Attribute) -> Result<Vec<u8>, &'static str> {
        match &attr.content {
            attributes::AttributeContent::Resident(data) => {
                Ok(data.clone())
            }
            attributes::AttributeContent::NonResident(non_res) => {
                self.read_non_resident_data(non_res)
            }
        }
    }
    
    fn read_non_resident_data(&mut self, non_res: &attributes::NonResidentAttribute) -> Result<Vec<u8>, &'static str> {
        let mut data = Vec::with_capacity(non_res.real_size as usize);
        
        // Read data runs
        for run in &non_res.data_runs {
            let start_cluster = run.start_lcn;
            let cluster_count = run.length;
            
            // Read clusters
            for i in 0..cluster_count {
                let cluster_data = self.read_cluster(start_cluster + i)?;
                data.extend_from_slice(&cluster_data);
            }
        }
        
        // Truncate to real size
        data.truncate(non_res.real_size as usize);
        Ok(data)
    }
    
    fn read_cluster(&mut self, lcn: u64) -> Result<Vec<u8>, &'static str> {
        let mut data = vec![0u8; self.cluster_size as usize];
        let start_sector = lcn * self.boot_sector.sectors_per_cluster as u64;
        
        self.disk.read_sectors(
            start_sector,
            self.boot_sector.sectors_per_cluster as u32,
            &mut data
        ).map_err(|_| "Failed to read cluster")?;
        
        Ok(data)
    }
    
    fn parse_index_entries(&self, data: &[u8]) -> Result<Vec<DirectoryEntry>, &'static str> {
        // Simplified index parsing
        Ok(Vec::new())
    }
    
    fn parse_index_allocation(&self, data: &[u8]) -> Result<Vec<DirectoryEntry>, &'static str> {
        // Simplified index allocation parsing
        Ok(Vec::new())
    }
    
    pub fn list_directory(&mut self, path: &str) -> Result<Vec<DirectoryEntry>, &'static str> {
        // Navigate to directory
        let components: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        
        let mut current_entry = MFT_ENTRY_ROOT;
        
        for component in components {
            let entry = self.mft.read_entry(current_entry)?;
            current_entry = self.find_in_directory(&entry, component, true)?;
        }
        
        // Read directory entries
        let entry = self.mft.read_entry(current_entry)?;
        self.read_directory_entries(&entry)
    }
    
    pub fn get_file_info(&mut self, path: &str) -> Result<FileInfo, &'static str> {
        // Parse path and find file
        let components: Vec<&str> = path.split('\\').filter(|s| !s.is_empty()).collect();
        
        if components.is_empty() {
            return Err("Invalid path");
        }
        
        let mut current_entry = MFT_ENTRY_ROOT;
        
        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            let entry = self.mft.read_entry(current_entry)?;
            
            current_entry = self.find_in_directory(&entry, component, !is_last)?;
        }
        
        // Read file entry
        let entry = self.mft.read_entry(current_entry)?;
        
        Ok(FileInfo {
            name: String::from(*components.last().unwrap()),
            size: self.get_file_size(&entry)?,
            is_directory: entry.is_directory(),
            created: entry.created_time,
            modified: entry.modified_time,
            accessed: entry.accessed_time,
            attributes: entry.file_attributes,
        })
    }
    
    fn get_file_size(&self, entry: &mft::MftEntry) -> Result<u64, &'static str> {
        if let Some(data_attr) = entry.get_attribute(attributes::ATTR_TYPE_DATA) {
            match &data_attr.content {
                attributes::AttributeContent::Resident(data) => Ok(data.len() as u64),
                attributes::AttributeContent::NonResident(non_res) => Ok(non_res.real_size),
            }
        } else {
            Ok(0)
        }
    }
}

#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub mft_reference: u64,
    pub is_directory: bool,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub is_directory: bool,
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub attributes: u32,
}

// VFS Integration
use super::{FileSystem, FileSystemError, FileType, FileInfo as VfsFileInfo};

impl FileSystem for NtfsFileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FileSystemError> {
        // Create a temporary mutable clone for read operations
        // In production, this would use interior mutability or refactor the trait
        Err(FileSystemError::NotSupported)
    }
    
    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), FileSystemError> {
        self.write_file_impl(path, data)
            .map_err(|e| FileSystemError::IoError(String::from(e)))
    }
    
    fn create_directory(&mut self, path: &str) -> Result<(), FileSystemError> {
        self.create_directory_impl(path)
            .map_err(|e| FileSystemError::IoError(String::from(e)))
    }
    
    fn list_directory(&self, path: &str) -> Result<Vec<VfsFileInfo>, FileSystemError> {
        // Need to handle the mutability issue
        // For now, return not supported
        Err(FileSystemError::NotSupported)
    }
    
    fn delete(&mut self, path: &str) -> Result<(), FileSystemError> {
        self.delete_file_impl(path)
            .map_err(|e| FileSystemError::IoError(String::from(e)))
    }
    
    fn get_file_info(&self, path: &str) -> Result<VfsFileInfo, FileSystemError> {
        // Need to handle the mutability issue
        // For now, return not supported
        Err(FileSystemError::NotSupported)
    }
}