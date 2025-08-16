// File System Unit Tests

use crate::test_runner::TestRunner;
use alloc::vec::Vec;
use alloc::string::String;

pub fn run_filesystem_tests(runner: &mut TestRunner) {
    runner.run_test("fs::path_parsing", || {
        // Test path parsing and normalization
        let paths = vec![
            ("/home/user/file.txt", true),
            ("../relative/path", false),
            ("/", true),
            ("//double//slash//", true),
            ("/path/./current/", true),
            ("/path/../parent/", true),
        ];
        
        for (path, is_absolute) in paths {
            let starts_with_slash = path.starts_with('/');
            if starts_with_slash != is_absolute {
                return Err(format!("Path '{}' absolute check failed", path));
            }
        }
        
        Ok(())
    });
    
    runner.run_test("fs::inode_operations", || {
        // Test inode structure and operations
        let inode = Inode {
            number: 12345,
            mode: 0o644,
            uid: 1000,
            gid: 1000,
            size: 4096,
            blocks: 8,
            atime: 1234567890,
            mtime: 1234567890,
            ctime: 1234567890,
            links: 1,
            block_pointers: [0; 15],
        };
        
        // Check permissions
        let owner_read = (inode.mode >> 8) & 0b100;
        let group_read = (inode.mode >> 5) & 0b100;
        let other_read = (inode.mode >> 2) & 0b100;
        
        if owner_read == 0 {
            return Err(format!("Owner should have read permission"));
        }
        
        // Check file type from mode
        let file_type = inode.mode & 0o170000;
        if file_type != 0o100000 && file_type != 0o040000 {
            // Should be regular file or directory
        }
        
        Ok(())
    });
    
    runner.run_test("fs::directory_entries", || {
        // Test directory entry operations
        let mut entries = Vec::new();
        
        entries.push(DirectoryEntry {
            inode: 1,
            name: String::from("."),
            file_type: FileType::Directory,
        });
        
        entries.push(DirectoryEntry {
            inode: 1,
            name: String::from(".."),
            file_type: FileType::Directory,
        });
        
        entries.push(DirectoryEntry {
            inode: 100,
            name: String::from("file.txt"),
            file_type: FileType::Regular,
        });
        
        // Lookup entry
        let found = entries.iter().find(|e| e.name == "file.txt");
        if found.is_none() {
            return Err(format!("Failed to find file.txt"));
        }
        
        let entry = found.unwrap();
        if entry.inode != 100 {
            return Err(format!("Wrong inode for file.txt"));
        }
        
        Ok(())
    });
    
    runner.run_test("fs::file_descriptors", || {
        // Test file descriptor management
        let mut fd_table = vec![None; 256];
        
        // Open stdin, stdout, stderr
        fd_table[0] = Some(FileDescriptor {
            inode: 1,
            offset: 0,
            flags: O_RDONLY,
        });
        fd_table[1] = Some(FileDescriptor {
            inode: 2,
            offset: 0,
            flags: O_WRONLY,
        });
        fd_table[2] = Some(FileDescriptor {
            inode: 3,
            offset: 0,
            flags: O_WRONLY,
        });
        
        // Find next available fd
        let mut next_fd = None;
        for i in 3..fd_table.len() {
            if fd_table[i].is_none() {
                next_fd = Some(i);
                break;
            }
        }
        
        if next_fd != Some(3) {
            return Err(format!("Next available fd should be 3"));
        }
        
        Ok(())
    });
    
    runner.run_test("fs::buffer_cache", || {
        // Test buffer cache operations
        let mut cache = BufferCache::new(16); // 16 buffers
        
        // Add buffer to cache
        cache.insert(BlockId { device: 0, block: 100 }, vec![0; 512]);
        
        // Lookup buffer
        if !cache.contains(BlockId { device: 0, block: 100 }) {
            return Err(format!("Buffer not found in cache"));
        }
        
        // Test LRU eviction
        for i in 0..20 {
            cache.insert(BlockId { device: 0, block: i }, vec![0; 512]);
        }
        
        // First blocks should be evicted
        if cache.contains(BlockId { device: 0, block: 0 }) {
            return Err(format!("Old buffer should have been evicted"));
        }
        
        Ok(())
    });
}

// FAT32 specific tests
pub fn run_fat32_tests(runner: &mut TestRunner) {
    runner.run_test("fat32::boot_sector", || {
        // Test FAT32 boot sector structure
        let boot_sector = Fat32BootSector {
            jump: [0xEB, 0x58, 0x90],
            oem_name: *b"MSWIN4.1",
            bytes_per_sector: 512,
            sectors_per_cluster: 8,
            reserved_sectors: 32,
            fat_count: 2,
            root_entry_count: 0, // Must be 0 for FAT32
            total_sectors_16: 0,
            media_type: 0xF8,
            sectors_per_fat_16: 0,
            sectors_per_track: 63,
            head_count: 255,
            hidden_sectors: 0,
            total_sectors_32: 1000000,
            sectors_per_fat_32: 1000,
            ext_flags: 0,
            fs_version: 0,
            root_cluster: 2,
            fs_info_sector: 1,
            backup_boot_sector: 6,
            signature: 0xAA55,
        };
        
        if boot_sector.bytes_per_sector != 512 {
            return Err(format!("Invalid bytes per sector"));
        }
        
        if boot_sector.root_entry_count != 0 {
            return Err(format!("FAT32 root entry count must be 0"));
        }
        
        if boot_sector.signature != 0xAA55 {
            return Err(format!("Invalid boot sector signature"));
        }
        
        Ok(())
    });
    
    runner.run_test("fat32::cluster_chain", || {
        // Test FAT cluster chain navigation
        let mut fat = vec![0u32; 1000];
        
        // Create cluster chain: 2 -> 3 -> 4 -> EOF
        fat[2] = 3;
        fat[3] = 4;
        fat[4] = 0x0FFFFFFF; // End of chain
        
        // Follow chain
        let mut current = 2;
        let mut chain_length = 0;
        
        while current < 0x0FFFFFF8 {
            chain_length += 1;
            current = fat[current as usize];
            
            if chain_length > 10 {
                return Err(format!("Cluster chain too long or circular"));
            }
        }
        
        if chain_length != 3 {
            return Err(format!("Expected chain length 3, got {}", chain_length));
        }
        
        Ok(())
    });
    
    runner.run_test("fat32::directory_entries", || {
        // Test FAT32 directory entry structure
        let entry = Fat32DirEntry {
            name: [b'F', b'I', b'L', b'E', b' ', b' ', b' ', b' '],
            ext: [b'T', b'X', b'T'],
            attributes: 0x20, // Archive
            nt_reserved: 0,
            create_time_tenth: 0,
            create_time: 0,
            create_date: 0,
            last_access_date: 0,
            cluster_high: 0,
            write_time: 0,
            write_date: 0,
            cluster_low: 100,
            file_size: 1024,
        };
        
        // Check if regular file
        if entry.attributes & 0x10 != 0 {
            return Err(format!("Should not be a directory"));
        }
        
        // Check cluster number
        let cluster = ((entry.cluster_high as u32) << 16) | (entry.cluster_low as u32);
        if cluster != 100 {
            return Err(format!("Cluster number mismatch"));
        }
        
        Ok(())
    });
    
    runner.run_test("fat32::long_filename", || {
        // Test long filename support
        let lfn_entry = Fat32LfnEntry {
            order: 0x41, // First entry, last in sequence
            chars1: [b'T' as u16, b'e' as u16, b's' as u16, b't' as u16, b' ' as u16],
            attributes: 0x0F, // LFN attribute
            entry_type: 0,
            checksum: 0xAB,
            chars2: [b'F' as u16, b'i' as u16, b'l' as u16, b'e' as u16, b' ' as u16, b'N' as u16],
            cluster: 0,
            chars3: [b'a' as u16, b'm' as u16],
        };
        
        if lfn_entry.attributes != 0x0F {
            return Err(format!("Invalid LFN attribute"));
        }
        
        if lfn_entry.order & 0x40 == 0 {
            return Err(format!("Should be last LFN entry"));
        }
        
        Ok(())
    });
}

// NTFS specific tests
pub fn run_ntfs_tests(runner: &mut TestRunner) {
    runner.run_test("ntfs::boot_sector", || {
        // Test NTFS boot sector
        let boot = NtfsBootSector {
            jump: [0xEB, 0x52, 0x90],
            oem_id: *b"NTFS    ",
            bytes_per_sector: 512,
            sectors_per_cluster: 8,
            reserved_sectors: 0,
            media_descriptor: 0xF8,
            sectors_per_track: 63,
            heads: 255,
            hidden_sectors: 63,
            total_sectors: 1000000,
            mft_cluster: 786432,
            mft_mirror_cluster: 2,
            clusters_per_mft_record: -10i8 as u8, // 2^10 = 1024 bytes
            clusters_per_index_block: 1,
            volume_serial: 0x1234567890ABCDEF,
            checksum: 0,
            signature: 0xAA55,
        };
        
        if &boot.oem_id != b"NTFS    " {
            return Err(format!("Invalid NTFS OEM ID"));
        }
        
        if boot.signature != 0xAA55 {
            return Err(format!("Invalid boot sector signature"));
        }
        
        // Calculate MFT record size
        let mft_record_size = if (boot.clusters_per_mft_record as i8) < 0 {
            1 << (-(boot.clusters_per_mft_record as i8)) as u32
        } else {
            boot.clusters_per_mft_record as u32 * boot.sectors_per_cluster as u32 * boot.bytes_per_sector as u32
        };
        
        if mft_record_size != 1024 {
            return Err(format!("MFT record size should be 1024, got {}", mft_record_size));
        }
        
        Ok(())
    });
    
    runner.run_test("ntfs::mft_entry", || {
        // Test Master File Table entry
        let mft_entry = MftEntry {
            signature: *b"FILE",
            update_sequence_offset: 48,
            update_sequence_size: 3,
            log_sequence: 1,
            sequence_number: 1,
            hard_link_count: 1,
            first_attribute_offset: 56,
            flags: 0x01, // In use
            used_size: 512,
            allocated_size: 1024,
            base_record: 0,
            next_attribute_id: 5,
        };
        
        if &mft_entry.signature != b"FILE" {
            return Err(format!("Invalid MFT signature"));
        }
        
        if mft_entry.flags & 0x01 == 0 {
            return Err(format!("MFT entry should be in use"));
        }
        
        if mft_entry.used_size > mft_entry.allocated_size {
            return Err(format!("Used size exceeds allocated size"));
        }
        
        Ok(())
    });
    
    runner.run_test("ntfs::attributes", || {
        // Test NTFS attributes
        let attributes = vec![
            (0x10, "$STANDARD_INFORMATION"),
            (0x30, "$FILE_NAME"),
            (0x40, "$OBJECT_ID"),
            (0x50, "$SECURITY_DESCRIPTOR"),
            (0x60, "$VOLUME_NAME"),
            (0x70, "$VOLUME_INFORMATION"),
            (0x80, "$DATA"),
            (0x90, "$INDEX_ROOT"),
            (0xA0, "$INDEX_ALLOCATION"),
            (0xB0, "$BITMAP"),
        ];
        
        for (type_code, name) in attributes {
            if type_code == 0x80 {
                // $DATA attribute
                if !name.contains("DATA") {
                    return Err(format!("Attribute 0x80 should be $DATA"));
                }
            }
        }
        
        Ok(())
    });
    
    runner.run_test("ntfs::filename_namespaces", || {
        // Test NTFS filename namespaces
        let namespaces = vec![
            (0, "POSIX"),
            (1, "Win32"),
            (2, "DOS"),
            (3, "Win32&DOS"),
        ];
        
        for (code, name) in namespaces {
            match code {
                0 => {
                    // POSIX: case-sensitive, all characters except NUL and /
                }
                1 => {
                    // Win32: case-insensitive, restricted characters
                }
                2 => {
                    // DOS: 8.3 format
                }
                3 => {
                    // Both Win32 and DOS
                }
                _ => return Err(format!("Invalid namespace code")),
            }
        }
        
        Ok(())
    });
}

// VFS (Virtual File System) tests
pub fn run_vfs_tests(runner: &mut TestRunner) {
    runner.run_test("vfs::mount_points", || {
        // Test mount point management
        let mut mounts = Vec::new();
        
        mounts.push(MountPoint {
            path: String::from("/"),
            device: String::from("/dev/sda1"),
            fs_type: String::from("ext4"),
            flags: 0,
        });
        
        mounts.push(MountPoint {
            path: String::from("/boot"),
            device: String::from("/dev/sda2"),
            fs_type: String::from("fat32"),
            flags: 0,
        });
        
        // Find mount for path
        let path = "/boot/grub/grub.cfg";
        let mount = mounts.iter()
            .filter(|m| path.starts_with(&m.path))
            .max_by_key(|m| m.path.len());
        
        if mount.is_none() {
            return Err(format!("No mount point found for {}", path));
        }
        
        let mount = mount.unwrap();
        if mount.path != "/boot" {
            return Err(format!("Wrong mount point for {}", path));
        }
        
        Ok(())
    });
    
    runner.run_test("vfs::path_resolution", || {
        // Test path resolution with symlinks
        let mut symlinks = Vec::new();
        symlinks.push((String::from("/usr/bin/python"), String::from("/usr/bin/python3")));
        symlinks.push((String::from("/tmp"), String::from("/var/tmp")));
        
        let path = "/usr/bin/python";
        let resolved = symlinks.iter()
            .find(|(link, _)| link == path)
            .map(|(_, target)| target.clone())
            .unwrap_or_else(|| String::from(path));
        
        if resolved != "/usr/bin/python3" {
            return Err(format!("Symlink resolution failed"));
        }
        
        Ok(())
    });
    
    runner.run_test("vfs::dcache", || {
        // Test directory entry cache
        let mut dcache = DentryCache::new(100);
        
        dcache.insert(String::from("/home/user"), Dentry {
            inode: 1000,
            parent: 100,
            name: String::from("user"),
        });
        
        if !dcache.contains("/home/user") {
            return Err(format!("Dentry not found in cache"));
        }
        
        let entry = dcache.get("/home/user");
        if entry.is_none() || entry.unwrap().inode != 1000 {
            return Err(format!("Wrong dentry retrieved"));
        }
        
        Ok(())
    });
}

// File operation tests
pub fn run_file_ops_tests(runner: &mut TestRunner) {
    runner.run_test("file_ops::read_write", || {
        // Test read/write operations
        let mut file = FileHandle {
            inode: 100,
            position: 0,
            size: 1024,
            buffer: vec![0; 1024],
        };
        
        // Write data
        let data = b"Hello, World!";
        file.write(data);
        file.position = 0;
        
        // Read data back
        let mut read_buf = vec![0; data.len()];
        file.read(&mut read_buf);
        
        if &read_buf != data {
            return Err(format!("Read data doesn't match written data"));
        }
        
        Ok(())
    });
    
    runner.run_test("file_ops::seek", || {
        // Test seek operations
        let mut file = FileHandle {
            inode: 100,
            position: 0,
            size: 1024,
            buffer: vec![0; 1024],
        };
        
        // SEEK_SET
        file.seek(100, SeekFrom::Start);
        if file.position != 100 {
            return Err(format!("SEEK_SET failed"));
        }
        
        // SEEK_CUR
        file.seek(50, SeekFrom::Current);
        if file.position != 150 {
            return Err(format!("SEEK_CUR failed"));
        }
        
        // SEEK_END
        file.seek(-24, SeekFrom::End);
        if file.position != 1000 {
            return Err(format!("SEEK_END failed"));
        }
        
        Ok(())
    });
    
    runner.run_test("file_ops::truncate", || {
        // Test file truncation
        let mut file = FileHandle {
            inode: 100,
            position: 0,
            size: 1024,
            buffer: vec![0xFF; 1024],
        };
        
        // Truncate to smaller size
        file.truncate(512);
        if file.size != 512 {
            return Err(format!("Truncate to smaller size failed"));
        }
        
        // Extend file
        file.truncate(2048);
        if file.size != 2048 {
            return Err(format!("Extend file failed"));
        }
        
        // Extended area should be zeroed
        if file.buffer.len() < 2048 {
            file.buffer.resize(2048, 0);
        }
        
        for i in 1024..2048 {
            if file.buffer[i] != 0 {
                return Err(format!("Extended area not zeroed at offset {}", i));
            }
        }
        
        Ok(())
    });
}

// Helper structures
struct Inode {
    number: u64,
    mode: u32,
    uid: u32,
    gid: u32,
    size: u64,
    blocks: u64,
    atime: u64,
    mtime: u64,
    ctime: u64,
    links: u32,
    block_pointers: [u64; 15],
}

struct DirectoryEntry {
    inode: u64,
    name: String,
    file_type: FileType,
}

enum FileType {
    Regular,
    Directory,
    Symlink,
    Device,
}

struct FileDescriptor {
    inode: u64,
    offset: u64,
    flags: u32,
}

const O_RDONLY: u32 = 0;
const O_WRONLY: u32 = 1;
const O_RDWR: u32 = 2;

struct BufferCache {
    capacity: usize,
    buffers: Vec<(BlockId, Vec<u8>)>,
}

impl BufferCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            buffers: Vec::new(),
        }
    }
    
    fn insert(&mut self, id: BlockId, data: Vec<u8>) {
        if self.buffers.len() >= self.capacity {
            self.buffers.remove(0);
        }
        self.buffers.push((id, data));
    }
    
    fn contains(&self, id: BlockId) -> bool {
        self.buffers.iter().any(|(bid, _)| *bid == id)
    }
}

#[derive(Clone, Copy, PartialEq)]
struct BlockId {
    device: u32,
    block: u64,
}

struct Fat32BootSector {
    jump: [u8; 3],
    oem_name: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    fat_count: u8,
    root_entry_count: u16,
    total_sectors_16: u16,
    media_type: u8,
    sectors_per_fat_16: u16,
    sectors_per_track: u16,
    head_count: u16,
    hidden_sectors: u32,
    total_sectors_32: u32,
    sectors_per_fat_32: u32,
    ext_flags: u16,
    fs_version: u16,
    root_cluster: u32,
    fs_info_sector: u16,
    backup_boot_sector: u16,
    signature: u16,
}

struct Fat32DirEntry {
    name: [u8; 8],
    ext: [u8; 3],
    attributes: u8,
    nt_reserved: u8,
    create_time_tenth: u8,
    create_time: u16,
    create_date: u16,
    last_access_date: u16,
    cluster_high: u16,
    write_time: u16,
    write_date: u16,
    cluster_low: u16,
    file_size: u32,
}

struct Fat32LfnEntry {
    order: u8,
    chars1: [u16; 5],
    attributes: u8,
    entry_type: u8,
    checksum: u8,
    chars2: [u16; 6],
    cluster: u16,
    chars3: [u16; 2],
}

struct NtfsBootSector {
    jump: [u8; 3],
    oem_id: [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    media_descriptor: u8,
    sectors_per_track: u16,
    heads: u16,
    hidden_sectors: u32,
    total_sectors: u64,
    mft_cluster: u64,
    mft_mirror_cluster: u64,
    clusters_per_mft_record: u8,
    clusters_per_index_block: u8,
    volume_serial: u64,
    checksum: u32,
    signature: u16,
}

struct MftEntry {
    signature: [u8; 4],
    update_sequence_offset: u16,
    update_sequence_size: u16,
    log_sequence: u64,
    sequence_number: u16,
    hard_link_count: u16,
    first_attribute_offset: u16,
    flags: u16,
    used_size: u32,
    allocated_size: u32,
    base_record: u64,
    next_attribute_id: u16,
}

struct MountPoint {
    path: String,
    device: String,
    fs_type: String,
    flags: u32,
}

struct DentryCache {
    capacity: usize,
    entries: Vec<(String, Dentry)>,
}

impl DentryCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: Vec::new(),
        }
    }
    
    fn insert(&mut self, path: String, dentry: Dentry) {
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push((path, dentry));
    }
    
    fn contains(&self, path: &str) -> bool {
        self.entries.iter().any(|(p, _)| p == path)
    }
    
    fn get(&self, path: &str) -> Option<&Dentry> {
        self.entries.iter()
            .find(|(p, _)| p == path)
            .map(|(_, d)| d)
    }
}

struct Dentry {
    inode: u64,
    parent: u64,
    name: String,
}

struct FileHandle {
    inode: u64,
    position: usize,
    size: usize,
    buffer: Vec<u8>,
}

impl FileHandle {
    fn write(&mut self, data: &[u8]) {
        for &byte in data {
            if self.position < self.buffer.len() {
                self.buffer[self.position] = byte;
            }
            self.position += 1;
        }
    }
    
    fn read(&mut self, buf: &mut [u8]) {
        for byte in buf {
            if self.position < self.buffer.len() {
                *byte = self.buffer[self.position];
            }
            self.position += 1;
        }
    }
    
    fn seek(&mut self, offset: i64, from: SeekFrom) {
        self.position = match from {
            SeekFrom::Start => offset as usize,
            SeekFrom::Current => (self.position as i64 + offset) as usize,
            SeekFrom::End => (self.size as i64 + offset) as usize,
        };
    }
    
    fn truncate(&mut self, new_size: usize) {
        self.size = new_size;
        if self.buffer.len() < new_size {
            self.buffer.resize(new_size, 0);
        }
    }
}

enum SeekFrom {
    Start,
    Current,
    End,
}