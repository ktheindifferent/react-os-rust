// FAT32 File System Implementation
use super::{FileSystem, FileSystemError, FileInfo, FileType};
use alloc::{vec::{self, Vec}, string::String};
use crate::drivers::disk::{DiskDriver, DISK_MANAGER, SECTOR_SIZE};

// FAT32 constants
const FAT32_SIGNATURE: u16 = 0xAA55;
const BYTES_PER_DIR_ENTRY: usize = 32;
const FAT_ENTRY_SIZE: u32 = 4;
const END_OF_CLUSTER_CHAIN: u32 = 0x0FFFFFFF;
const BAD_CLUSTER: u32 = 0x0FFFFFF7;

// FAT32 Boot Sector structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Fat32BootSector {
    jump_boot: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sector_count: u16,
    num_fats: u8,
    root_entry_count: u16,  // Must be 0 for FAT32
    total_sectors_16: u16,   // Must be 0 for FAT32
    media: u8,
    fat_size_16: u16,        // Must be 0 for FAT32
    sectors_per_track: u16,
    number_of_heads: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,
    // FAT32 specific
    fat_size_32: u32,
    ext_flags: u16,
    fs_version: u16,
    root_cluster: u32,
    fs_info: u16,
    backup_boot_sector: u16,
    reserved: [u8; 12],
    drive_number: u8,
    reserved1: u8,
    boot_signature: u8,
    volume_id: u32,
    volume_label: [u8; 11],
    fs_type: [u8; 8],
}

// FAT32 Directory Entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Fat32DirEntry {
    name: [u8; 11],
    attributes: u8,
    nt_reserved: u8,
    creation_time_tenth: u8,
    creation_time: u16,
    creation_date: u16,
    last_access_date: u16,
    first_cluster_high: u16,
    write_time: u16,
    write_date: u16,
    first_cluster_low: u16,
    file_size: u32,
}

// Directory entry attributes
const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN: u8 = 0x02;
const ATTR_SYSTEM: u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE: u8 = 0x20;
const ATTR_LONG_NAME: u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;

pub struct Fat32FileSystem {
    disk_index: usize,
    boot_sector: Fat32BootSector,
    fat_start_sector: u32,
    data_start_sector: u32,
    sectors_per_cluster: u32,
    root_dir_cluster: u32,
}

impl Fat32FileSystem {
    pub fn new(disk_index: usize) -> Result<Self, FileSystemError> {
        use crate::serial_println;
        
        // Read boot sector
        let mut boot_sector_data = Vec::with_capacity(SECTOR_SIZE);
        boot_sector_data.resize(SECTOR_SIZE, 0u8);
        
        {
            let mut disk_manager = DISK_MANAGER.lock();
            if let Some(disk) = disk_manager.get_disk(disk_index) {
                serial_println!("Reading boot sector from disk {}...", disk_index);
                match disk.read_sectors(0, 1, &mut boot_sector_data) {
                    Ok(_) => serial_println!("Boot sector read successfully"),
                    Err(e) => {
                        serial_println!("Failed to read boot sector: {:?}", e);
                        return Err(FileSystemError::IoError);
                    }
                }
            } else {
                serial_println!("Disk {} not found", disk_index);
                return Err(FileSystemError::NotFound);
            }
        }
        
        // Parse boot sector
        let boot_sector = unsafe {
            *(boot_sector_data.as_ptr() as *const Fat32BootSector)
        };
        
        // Validate FAT32 signature
        let signature = u16::from_le_bytes([boot_sector_data[510], boot_sector_data[511]]);
        if signature != FAT32_SIGNATURE {
            return Err(FileSystemError::InvalidPath);
        }
        
        // Calculate important sectors
        let fat_start_sector = boot_sector.reserved_sector_count as u32;
        let fat_size = boot_sector.fat_size_32;
        let data_start_sector = fat_start_sector + (boot_sector.num_fats as u32 * fat_size);
        
        Ok(Self {
            disk_index,
            boot_sector,
            fat_start_sector,
            data_start_sector,
            sectors_per_cluster: boot_sector.sectors_per_cluster as u32,
            root_dir_cluster: boot_sector.root_cluster,
        })
    }
    
    // Convert cluster number to sector number
    fn cluster_to_sector(&self, cluster: u32) -> u32 {
        self.data_start_sector + ((cluster - 2) * self.sectors_per_cluster)
    }
    
    // Read a cluster from disk
    fn read_cluster(&self, cluster: u32) -> Result<Vec<u8>, FileSystemError> {
        let sector = self.cluster_to_sector(cluster);
        let mut data = Vec::with_capacity(self.sectors_per_cluster as usize * SECTOR_SIZE);
        data.resize(self.sectors_per_cluster as usize * SECTOR_SIZE, 0u8);
        
        let mut disk_manager = DISK_MANAGER.lock();
        if let Some(disk) = disk_manager.get_disk(self.disk_index) {
            disk.read_sectors(sector as u64, self.sectors_per_cluster, &mut data)
                .map_err(|_| FileSystemError::IoError)?;
        }
        
        Ok(data)
    }
    
    // Get next cluster from FAT
    fn get_next_cluster(&self, cluster: u32) -> Result<u32, FileSystemError> {
        let fat_offset = cluster * FAT_ENTRY_SIZE;
        let fat_sector = self.fat_start_sector + (fat_offset / SECTOR_SIZE as u32);
        let entry_offset = (fat_offset % SECTOR_SIZE as u32) as usize;
        
        let mut sector_data = Vec::with_capacity(SECTOR_SIZE);
        sector_data.resize(SECTOR_SIZE, 0u8);
        
        let mut disk_manager = DISK_MANAGER.lock();
        if let Some(disk) = disk_manager.get_disk(self.disk_index) {
            disk.read_sectors(fat_sector as u64, 1, &mut sector_data)
                .map_err(|_| FileSystemError::IoError)?;
        }
        
        let next_cluster = u32::from_le_bytes([
            sector_data[entry_offset],
            sector_data[entry_offset + 1],
            sector_data[entry_offset + 2],
            sector_data[entry_offset + 3],
        ]) & 0x0FFFFFFF;
        
        Ok(next_cluster)
    }
    
    // Read entire cluster chain
    fn read_cluster_chain(&self, start_cluster: u32) -> Result<Vec<u8>, FileSystemError> {
        let mut data = Vec::new();
        let mut current_cluster = start_cluster;
        
        while current_cluster < END_OF_CLUSTER_CHAIN && current_cluster != BAD_CLUSTER {
            let cluster_data = self.read_cluster(current_cluster)?;
            data.extend_from_slice(&cluster_data);
            current_cluster = self.get_next_cluster(current_cluster)?;
        }
        
        Ok(data)
    }
    
    // Parse short filename (8.3 format)
    fn parse_short_name(name: &[u8; 11]) -> String {
        let mut result = String::new();
        
        // First 8 bytes are the name
        for i in 0..8 {
            if name[i] == 0x20 || name[i] == 0 {
                break;
            }
            result.push(name[i] as char);
        }
        
        // Last 3 bytes are the extension
        if name[8] != 0x20 && name[8] != 0 {
            result.push('.');
            for i in 8..11 {
                if name[i] == 0x20 || name[i] == 0 {
                    break;
                }
                result.push(name[i] as char);
            }
        }
        
        result
    }
    
    // List directory entries in a cluster
    fn list_dir_cluster(&self, cluster: u32) -> Result<Vec<FileInfo>, FileSystemError> {
        let mut files = Vec::new();
        let data = self.read_cluster_chain(cluster)?;
        
        let entries_per_cluster = (self.sectors_per_cluster as usize * SECTOR_SIZE) / BYTES_PER_DIR_ENTRY;
        
        for i in 0..entries_per_cluster {
            let offset = i * BYTES_PER_DIR_ENTRY;
            if offset + BYTES_PER_DIR_ENTRY > data.len() {
                break;
            }
            
            let entry = unsafe {
                *(data[offset..].as_ptr() as *const Fat32DirEntry)
            };
            
            // Skip empty entries
            if entry.name[0] == 0x00 {
                break;  // No more entries
            }
            if entry.name[0] == 0xE5 {
                continue;  // Deleted entry
            }
            
            // Skip long name entries for now
            if entry.attributes == ATTR_LONG_NAME {
                continue;
            }
            
            // Skip volume label
            if entry.attributes & ATTR_VOLUME_ID != 0 {
                continue;
            }
            
            let name = Self::parse_short_name(&entry.name);
            let file_type = if entry.attributes & ATTR_DIRECTORY != 0 {
                FileType::Directory
            } else {
                FileType::Regular
            };
            
            files.push(FileInfo {
                name,
                size: entry.file_size as u64,
                file_type,
                permissions: 0o755,
            });
        }
        
        Ok(files)
    }
    
    // Find a file in a directory
    fn find_in_directory(&self, dir_cluster: u32, name: &str) -> Result<Fat32DirEntry, FileSystemError> {
        let data = self.read_cluster_chain(dir_cluster)?;
        let name_upper = name.to_uppercase();
        
        let entries_per_cluster = (self.sectors_per_cluster as usize * SECTOR_SIZE) / BYTES_PER_DIR_ENTRY;
        
        for i in 0..entries_per_cluster {
            let offset = i * BYTES_PER_DIR_ENTRY;
            if offset + BYTES_PER_DIR_ENTRY > data.len() {
                break;
            }
            
            let entry = unsafe {
                *(data[offset..].as_ptr() as *const Fat32DirEntry)
            };
            
            if entry.name[0] == 0x00 {
                break;
            }
            if entry.name[0] == 0xE5 || entry.attributes == ATTR_LONG_NAME {
                continue;
            }
            
            let entry_name = Self::parse_short_name(&entry.name).to_uppercase();
            if entry_name == name_upper {
                return Ok(entry);
            }
        }
        
        Err(FileSystemError::NotFound)
    }
}

impl FileSystem for Fat32FileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FileSystemError> {
        // Parse path and navigate to file
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        if parts.is_empty() {
            return Err(FileSystemError::InvalidPath);
        }
        
        let mut current_cluster = self.root_dir_cluster;
        
        // Navigate through directories
        for (i, part) in parts.iter().enumerate() {
            let entry = self.find_in_directory(current_cluster, part)?;
            
            if i == parts.len() - 1 {
                // This should be the file
                if entry.attributes & ATTR_DIRECTORY != 0 {
                    return Err(FileSystemError::InvalidPath);
                }
                
                let file_cluster = (entry.first_cluster_high as u32) << 16 | 
                                  entry.first_cluster_low as u32;
                let mut data = self.read_cluster_chain(file_cluster)?;
                data.truncate(entry.file_size as usize);
                return Ok(data);
            } else {
                // This should be a directory
                if entry.attributes & ATTR_DIRECTORY == 0 {
                    return Err(FileSystemError::InvalidPath);
                }
                current_cluster = (entry.first_cluster_high as u32) << 16 | 
                                 entry.first_cluster_low as u32;
            }
        }
        
        Err(FileSystemError::NotFound)
    }
    
    fn write_file(&mut self, _path: &str, _data: &[u8]) -> Result<(), FileSystemError> {
        // Writing to FAT32 is complex - would need to:
        // 1. Find or create directory entry
        // 2. Allocate clusters in FAT
        // 3. Write data to clusters
        // 4. Update directory entry
        Err(FileSystemError::IoError)  // Not implemented yet
    }
    
    fn create_directory(&mut self, _path: &str) -> Result<(), FileSystemError> {
        // Would need to:
        // 1. Allocate a cluster
        // 2. Create . and .. entries
        // 3. Add directory entry in parent
        Err(FileSystemError::IoError)  // Not implemented yet
    }
    
    fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>, FileSystemError> {
        if path == "/" || path.is_empty() {
            return self.list_dir_cluster(self.root_dir_cluster);
        }
        
        // Navigate to directory
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        let mut current_cluster = self.root_dir_cluster;
        
        for part in parts {
            let entry = self.find_in_directory(current_cluster, part)?;
            
            if entry.attributes & ATTR_DIRECTORY == 0 {
                return Err(FileSystemError::InvalidPath);
            }
            
            current_cluster = (entry.first_cluster_high as u32) << 16 | 
                             entry.first_cluster_low as u32;
        }
        
        self.list_dir_cluster(current_cluster)
    }
    
    fn delete(&mut self, _path: &str) -> Result<(), FileSystemError> {
        // Would need to:
        // 1. Find directory entry
        // 2. Mark clusters as free in FAT
        // 3. Mark directory entry as deleted (0xE5)
        Err(FileSystemError::IoError)  // Not implemented yet
    }
    
    fn get_file_info(&self, path: &str) -> Result<FileInfo, FileSystemError> {
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
        
        if parts.is_empty() {
            return Ok(FileInfo {
                name: String::from("/"),
                size: 0,
                file_type: FileType::Directory,
                permissions: 0o755,
            });
        }
        
        let mut current_cluster = self.root_dir_cluster;
        
        for (i, part) in parts.iter().enumerate() {
            let entry = self.find_in_directory(current_cluster, part)?;
            
            if i == parts.len() - 1 {
                // This is the target
                return Ok(FileInfo {
                    name: Self::parse_short_name(&entry.name),
                    size: entry.file_size as u64,
                    file_type: if entry.attributes & ATTR_DIRECTORY != 0 {
                        FileType::Directory
                    } else {
                        FileType::Regular
                    },
                    permissions: if entry.attributes & ATTR_READ_ONLY != 0 {
                        0o555
                    } else {
                        0o755
                    },
                });
            }
            
            if entry.attributes & ATTR_DIRECTORY == 0 {
                return Err(FileSystemError::InvalidPath);
            }
            
            current_cluster = (entry.first_cluster_high as u32) << 16 | 
                             entry.first_cluster_low as u32;
        }
        
        Err(FileSystemError::NotFound)
    }
}