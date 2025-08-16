// NTFS Filesystem Tests
use crate::fs::ntfs::{NtfsFileSystem, DirectoryEntry, FileInfo};
use crate::drivers::disk::DiskDriver;
use alloc::vec::Vec;
use alloc::string::String;

// Mock disk driver for testing
pub struct MockDiskDriver {
    data: Vec<u8>,
    sector_size: usize,
}

impl MockDiskDriver {
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0u8; size],
            sector_size: 512,
        }
    }
    
    pub fn setup_ntfs_boot_sector(&mut self) {
        // Create a minimal NTFS boot sector
        let boot_sector = &mut self.data[0..512];
        
        // Jump instruction
        boot_sector[0] = 0xEB;
        boot_sector[1] = 0x52;
        boot_sector[2] = 0x90;
        
        // OEM ID
        boot_sector[3..11].copy_from_slice(b"NTFS    ");
        
        // Bytes per sector
        boot_sector[11..13].copy_from_slice(&512u16.to_le_bytes());
        
        // Sectors per cluster
        boot_sector[13] = 8;
        
        // Reserved sectors
        boot_sector[14..16].copy_from_slice(&0u16.to_le_bytes());
        
        // Media descriptor
        boot_sector[21] = 0xF8;
        
        // Sectors per track
        boot_sector[24..26].copy_from_slice(&63u16.to_le_bytes());
        
        // Number of heads
        boot_sector[26..28].copy_from_slice(&255u16.to_le_bytes());
        
        // Total sectors
        boot_sector[40..48].copy_from_slice(&1048576u64.to_le_bytes());
        
        // MFT logical cluster number
        boot_sector[48..56].copy_from_slice(&4u64.to_le_bytes());
        
        // MFT mirror logical cluster number
        boot_sector[56..64].copy_from_slice(&1u64.to_le_bytes());
        
        // Clusters per MFT record
        boot_sector[64] = 0xF6; // -10 = 1024 bytes
        
        // Clusters per index block
        boot_sector[68] = 1;
        
        // Volume serial number
        boot_sector[72..80].copy_from_slice(&0x1234567890ABCDEFu64.to_le_bytes());
        
        // Boot signature
        boot_sector[510] = 0x55;
        boot_sector[511] = 0xAA;
    }
}

impl DiskDriver for MockDiskDriver {
    fn read_sectors(&mut self, start: u64, count: u32, buffer: &mut [u8]) -> Result<(), crate::drivers::disk::DiskError> {
        let start_byte = start as usize * self.sector_size;
        let end_byte = start_byte + (count as usize * self.sector_size);
        
        if end_byte <= self.data.len() {
            buffer.copy_from_slice(&self.data[start_byte..end_byte]);
            Ok(())
        } else {
            Err(crate::drivers::disk::DiskError::InvalidSector)
        }
    }
    
    fn write_sectors(&mut self, start: u64, count: u32, data: &[u8]) -> Result<(), crate::drivers::disk::DiskError> {
        let start_byte = start as usize * self.sector_size;
        let end_byte = start_byte + (count as usize * self.sector_size);
        
        if end_byte <= self.data.len() && data.len() == (count as usize * self.sector_size) {
            self.data[start_byte..end_byte].copy_from_slice(data);
            Ok(())
        } else {
            Err(crate::drivers::disk::DiskError::InvalidSector)
        }
    }
    
    fn get_sector_size(&self) -> usize {
        self.sector_size
    }
    
    fn get_total_sectors(&self) -> u64 {
        (self.data.len() / self.sector_size) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::FileSystem;
    
    #[test]
    fn test_ntfs_initialization() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024); // 100MB
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let result = NtfsFileSystem::new(disk_box);
        
        assert!(result.is_ok(), "Failed to initialize NTFS filesystem");
    }
    
    #[test]
    fn test_file_creation() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Test file creation
        let test_data = b"Hello, NTFS!";
        let result = ntfs.write_file("test.txt", test_data);
        
        assert!(result.is_ok(), "Failed to create file");
        
        // Test reading the file back
        let read_result = ntfs.read_file("test.txt");
        assert!(read_result.is_ok(), "Failed to read file");
        
        let read_data = read_result.unwrap();
        assert_eq!(read_data, test_data, "File data mismatch");
    }
    
    #[test]
    fn test_directory_creation() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Test directory creation
        let result = ntfs.create_directory("testdir");
        assert!(result.is_ok(), "Failed to create directory");
        
        // Test creating file in directory
        let file_result = ntfs.write_file("testdir\\file.txt", b"Content");
        assert!(file_result.is_ok(), "Failed to create file in directory");
    }
    
    #[test]
    fn test_file_deletion() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create a file
        ntfs.write_file("delete_me.txt", b"Temporary").unwrap();
        
        // Delete the file
        let delete_result = ntfs.delete("delete_me.txt");
        assert!(delete_result.is_ok(), "Failed to delete file");
        
        // Try to read deleted file
        let read_result = ntfs.read_file("delete_me.txt");
        assert!(read_result.is_err(), "Deleted file still exists");
    }
    
    #[test]
    fn test_large_file() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create a large file (non-resident data)
        let large_data = vec![0x42u8; 10000]; // 10KB of data
        let result = ntfs.write_file("large.bin", &large_data);
        assert!(result.is_ok(), "Failed to create large file");
        
        // Read it back
        let read_result = ntfs.read_file("large.bin");
        assert!(read_result.is_ok(), "Failed to read large file");
        
        let read_data = read_result.unwrap();
        assert_eq!(read_data.len(), large_data.len(), "Large file size mismatch");
        assert_eq!(read_data, large_data, "Large file data mismatch");
    }
    
    #[test]
    fn test_file_update() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create initial file
        ntfs.write_file("update.txt", b"Initial content").unwrap();
        
        // Update file with new content
        let new_content = b"Updated content with more data";
        let update_result = ntfs.write_file("update.txt", new_content);
        assert!(update_result.is_ok(), "Failed to update file");
        
        // Read updated file
        let read_result = ntfs.read_file("update.txt").unwrap();
        assert_eq!(read_result, new_content, "Updated file content mismatch");
    }
    
    #[test]
    fn test_nested_directories() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create nested directory structure
        ntfs.create_directory("level1").unwrap();
        ntfs.create_directory("level1\\level2").unwrap();
        ntfs.create_directory("level1\\level2\\level3").unwrap();
        
        // Create file in nested directory
        let file_path = "level1\\level2\\level3\\deep.txt";
        let result = ntfs.write_file(file_path, b"Deep file");
        assert!(result.is_ok(), "Failed to create file in nested directory");
        
        // Read file from nested directory
        let read_result = ntfs.read_file(file_path);
        assert!(read_result.is_ok(), "Failed to read file from nested directory");
    }
    
    #[test]
    fn test_hard_links() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create original file
        ntfs.write_file("original.txt", b"Shared content").unwrap();
        
        // Create hard link
        let link_result = ntfs.create_hard_link("original.txt", "hardlink.txt");
        assert!(link_result.is_ok(), "Failed to create hard link");
        
        // Read through hard link
        let read_result = ntfs.read_file("hardlink.txt");
        assert!(read_result.is_ok(), "Failed to read through hard link");
        assert_eq!(read_result.unwrap(), b"Shared content");
        
        // Update through hard link
        ntfs.write_file("hardlink.txt", b"Modified content").unwrap();
        
        // Read through original
        let original_read = ntfs.read_file("original.txt").unwrap();
        assert_eq!(original_read, b"Modified content", "Hard link update not reflected");
    }
    
    #[test]
    fn test_symbolic_links() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create target file
        ntfs.write_file("target.txt", b"Target content").unwrap();
        
        // Create symbolic link
        let symlink_result = ntfs.create_symbolic_link("symlink.txt", "target.txt");
        assert!(symlink_result.is_ok(), "Failed to create symbolic link");
        
        // Read symbolic link target
        let target_result = ntfs.read_symbolic_link("symlink.txt");
        assert!(target_result.is_ok(), "Failed to read symbolic link");
        assert_eq!(target_result.unwrap(), "target.txt");
    }
    
    #[test]
    fn test_file_attributes() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create file
        ntfs.write_file("attrs.txt", b"Test").unwrap();
        
        // Set file attributes
        let attrs = 0x01 | 0x02 | 0x20; // ReadOnly | Hidden | Archive
        let result = ntfs.set_file_attributes("attrs.txt", attrs);
        assert!(result.is_ok(), "Failed to set file attributes");
        
        // Get file info
        let info = ntfs.get_file_info("attrs.txt");
        assert!(info.is_ok(), "Failed to get file info");
        
        let file_info = info.unwrap();
        assert_eq!(file_info.attributes & attrs, attrs, "File attributes mismatch");
    }
    
    #[test]
    fn test_sparse_files() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create sparse file
        ntfs.write_file("sparse.dat", &[]).unwrap();
        
        // Mark as sparse
        let sparse_result = ntfs.set_sparse("sparse.dat");
        assert!(sparse_result.is_ok(), "Failed to mark file as sparse");
        
        // Allocate sparse range
        let alloc_result = ntfs.allocate_sparse_range("sparse.dat", 1024 * 1024, 4096);
        assert!(alloc_result.is_ok(), "Failed to allocate sparse range");
    }
    
    #[test]
    fn test_compression() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create file
        ntfs.write_file("compress.txt", b"Compressible content").unwrap();
        
        // Enable compression
        let compress_result = ntfs.enable_compression("compress.txt");
        assert!(compress_result.is_ok(), "Failed to enable compression");
        
        // Disable compression
        let decompress_result = ntfs.disable_compression("compress.txt");
        assert!(decompress_result.is_ok(), "Failed to disable compression");
    }
    
    #[test]
    fn test_extended_attributes() {
        let mut mock_disk = MockDiskDriver::new(100 * 1024 * 1024);
        mock_disk.setup_ntfs_boot_sector();
        
        let disk_box = Box::new(mock_disk);
        let mut ntfs = NtfsFileSystem::new(disk_box).unwrap();
        
        // Create file
        ntfs.write_file("ea_test.txt", b"Content").unwrap();
        
        // Set extended attribute
        let ea_result = ntfs.set_extended_attribute("ea_test.txt", "custom.attr", b"custom value");
        assert!(ea_result.is_ok(), "Failed to set extended attribute");
        
        // Get extended attribute
        let get_result = ntfs.get_extended_attribute("ea_test.txt", "custom.attr");
        assert!(get_result.is_ok(), "Failed to get extended attribute");
        assert_eq!(get_result.unwrap(), b"custom value");
    }
}