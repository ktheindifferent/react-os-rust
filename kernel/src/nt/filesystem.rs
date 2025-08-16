use super::{NtStatus, object::{Handle, ObjectHeader, ObjectTrait, ObjectType}};
use super::io::{FileObject, IoStatusBlock, Irp};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::VirtAddr;

// File system types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSystemType {
    Unknown = 0,
    Fat12 = 1,
    Fat16 = 2,
    Fat32 = 3,
    Ntfs = 4,
    Ext2 = 5,
    Ext3 = 6,
    Ext4 = 7,
    ReFS = 8,
}

// File attributes - Windows compatible
bitflags::bitflags! {
    #[repr(transparent)]
    #[derive(Debug)]
    pub struct FileAttributes: u32 {
        const READONLY              = 0x00000001;
        const HIDDEN                = 0x00000002;
        const SYSTEM                = 0x00000004;
        const DIRECTORY             = 0x00000010;
        const ARCHIVE               = 0x00000020;
        const DEVICE                = 0x00000040;
        const NORMAL                = 0x00000080;
        const TEMPORARY             = 0x00000100;
        const SPARSE_FILE           = 0x00000200;
        const REPARSE_POINT         = 0x00000400;
        const COMPRESSED            = 0x00000800;
        const OFFLINE               = 0x00001000;
        const NOT_CONTENT_INDEXED   = 0x00002000;
        const ENCRYPTED             = 0x00004000;
        const INTEGRITY_STREAM      = 0x00008000;
        const VIRTUAL               = 0x00010000;
        const NO_SCRUB_DATA         = 0x00020000;
        const EA                    = 0x00040000;
        const PINNED                = 0x00080000;
        const UNPINNED              = 0x00100000;
        const RECALL_ON_OPEN        = 0x00040000;
        const RECALL_ON_DATA_ACCESS = 0x00400000;
    }
}

// File information classes
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileInformationClass {
    FileDirectoryInformation = 1,
    FileFullDirectoryInformation = 2,
    FileBothDirectoryInformation = 3,
    FileBasicInformation = 4,
    FileStandardInformation = 5,
    FileInternalInformation = 6,
    FileEaInformation = 7,
    FileAccessInformation = 8,
    FileNameInformation = 9,
    FileRenameInformation = 10,
    FileLinkInformation = 11,
    FileNamesInformation = 12,
    FileDispositionInformation = 13,
    FilePositionInformation = 14,
    FileFullEaInformation = 15,
    FileModeInformation = 16,
    FileAlignmentInformation = 17,
    FileAllInformation = 18,
    FileAllocationInformation = 19,
    FileEndOfFileInformation = 20,
    FileAlternateNameInformation = 21,
    FileStreamInformation = 22,
    FilePipeInformation = 23,
    FilePipeLocalInformation = 24,
    FilePipeRemoteInformation = 25,
    FileMailslotQueryInformation = 26,
    FileMailslotSetInformation = 27,
    FileCompressionInformation = 28,
    FileObjectIdInformation = 29,
    FileCompletionInformation = 30,
    FileMoveClusterInformation = 31,
    FileQuotaInformation = 32,
    FileReparsePointInformation = 33,
    FileNetworkOpenInformation = 34,
    FileAttributeTagInformation = 35,
    FileTrackingInformation = 36,
    FileIdBothDirectoryInformation = 37,
    FileIdFullDirectoryInformation = 38,
    FileValidDataLengthInformation = 39,
    FileShortNameInformation = 40,
    FileIoCompletionNotificationInformation = 41,
    FileIoStatusBlockRangeInformation = 42,
    FileIoPriorityHintInformation = 43,
    FileSfioReserveInformation = 44,
    FileSfioVolumeInformation = 45,
    FileHardLinkInformation = 46,
    FileProcessIdsUsingFileInformation = 47,
    FileNormalizedNameInformation = 48,
    FileNetworkPhysicalNameInformation = 49,
    FileIdGlobalTxDirectoryInformation = 50,
    FileIsRemoteDeviceInformation = 51,
    FileUnusedInformation = 52,
    FileNumaNodeInformation = 53,
    FileStandardLinkInformation = 54,
    FileRemoteProtocolInformation = 55,
    FileRenameInformationBypassAccessCheck = 56,
    FileLinkInformationBypassAccessCheck = 57,
    FileVolumeNameInformation = 58,
    FileIdInformation = 59,
    FileIdExtdDirectoryInformation = 60,
    FileReplaceCompletionInformation = 61,
    FileHardLinkFullIdInformation = 62,
    FileIdExtdBothDirectoryInformation = 63,
    FileDispositionInformationEx = 64,
    FileRenameInformationEx = 65,
    FileRenameInformationExBypassAccessCheck = 66,
    FileDesiredStorageClassInformation = 67,
    FileStatInformation = 68,
    FileMemoryPartitionInformation = 69,
    FileStatLxInformation = 70,
    FileCaseSensitiveInformation = 71,
    FileLinkInformationEx = 72,
    FileLinkInformationExBypassAccessCheck = 73,
    FileStorageReserveIdInformation = 74,
    FileCaseSensitiveInformationForceAccessCheck = 75,
    FileKnownFolderInformation = 76,
    FileMaximumInformation = 77,
}

// File information structures
#[repr(C)]
#[derive(Debug, Clone)]
pub struct FileBasicInformation {
    pub creation_time: i64,
    pub last_access_time: i64,
    pub last_write_time: i64,
    pub change_time: i64,
    pub file_attributes: FileAttributes,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FileStandardInformation {
    pub allocation_size: i64,
    pub end_of_file: i64,
    pub number_of_links: u32,
    pub delete_pending: bool,
    pub directory: bool,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FilePositionInformation {
    pub current_byte_offset: i64,
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FileNameInformation {
    pub file_name_length: u32,
    // file_name follows as WCHAR array
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct FileNetworkOpenInformation {
    pub creation_time: i64,
    pub last_access_time: i64,
    pub last_write_time: i64,
    pub change_time: i64,
    pub allocation_size: i64,
    pub end_of_file: i64,
    pub file_attributes: FileAttributes,
}

// Directory entry
#[derive(Debug, Clone)]
pub struct DirectoryEntry {
    pub name: String,
    pub file_attributes: FileAttributes,
    pub file_size: u64,
    pub creation_time: i64,
    pub last_access_time: i64,
    pub last_write_time: i64,
    pub change_time: i64,
    pub file_id: u64,
    pub is_directory: bool,
}

// File system interface trait
pub trait FileSystem: Send + Sync {
    fn get_type(&self) -> FileSystemType;
    fn get_volume_label(&self) -> String;
    fn get_volume_size(&self) -> u64;
    fn get_free_space(&self) -> u64;
    
    // File operations
    fn create_file(
        &mut self,
        path: &str,
        desired_access: u32,
        file_attributes: FileAttributes,
        share_access: u32,
        create_disposition: u32,
        create_options: u32,
    ) -> Result<Handle, NtStatus>;
    
    fn open_file(
        &mut self,
        path: &str,
        desired_access: u32,
        share_access: u32,
        open_options: u32,
    ) -> Result<Handle, NtStatus>;
    
    fn close_file(&mut self, handle: Handle) -> NtStatus;
    
    fn read_file(
        &mut self,
        handle: Handle,
        buffer: &mut [u8],
        offset: u64,
    ) -> Result<usize, NtStatus>;
    
    fn write_file(
        &mut self,
        handle: Handle,
        buffer: &[u8],
        offset: u64,
    ) -> Result<usize, NtStatus>;
    
    fn delete_file(&mut self, path: &str) -> NtStatus;
    
    fn rename_file(&mut self, old_path: &str, new_path: &str) -> NtStatus;
    
    // Directory operations
    fn create_directory(&mut self, path: &str) -> NtStatus;
    fn remove_directory(&mut self, path: &str) -> NtStatus;
    fn enumerate_directory(&self, path: &str) -> Result<Vec<DirectoryEntry>, NtStatus>;
    
    // Information queries
    fn query_file_information(
        &self,
        handle: Handle,
        info_class: FileInformationClass,
    ) -> Result<Vec<u8>, NtStatus>;
    
    fn set_file_information(
        &mut self,
        handle: Handle,
        info_class: FileInformationClass,
        buffer: &[u8],
    ) -> NtStatus;
}

// Simple in-memory file system implementation
pub struct MemoryFileSystem {
    fs_type: FileSystemType,
    volume_label: String,
    volume_size: u64,
    files: BTreeMap<String, MemoryFile>,
    directories: BTreeMap<String, MemoryDirectory>,
    next_handle: AtomicU64,
    open_handles: BTreeMap<Handle, String>,
}

#[derive(Debug, Clone)]
struct MemoryFile {
    data: Vec<u8>,
    attributes: FileAttributes,
    creation_time: i64,
    last_access_time: i64,
    last_write_time: i64,
    change_time: i64,
    file_id: u64,
}

#[derive(Debug, Clone)]
struct MemoryDirectory {
    attributes: FileAttributes,
    creation_time: i64,
    last_access_time: i64,
    last_write_time: i64,
    change_time: i64,
    entries: Vec<String>,
}

impl MemoryFileSystem {
    pub fn new(volume_label: String, volume_size: u64) -> Self {
        let mut fs = Self {
            fs_type: FileSystemType::Unknown,
            volume_label,
            volume_size,
            files: BTreeMap::new(),
            directories: BTreeMap::new(),
            next_handle: AtomicU64::new(1),
            open_handles: BTreeMap::new(),
        };
        
        // Create root directory
        fs.directories.insert("\\".to_string(), MemoryDirectory {
            attributes: FileAttributes::DIRECTORY,
            creation_time: 0,
            last_access_time: 0,
            last_write_time: 0,
            change_time: 0,
            entries: Vec::new(),
        });
        
        fs
    }
    
    fn get_next_handle(&self) -> Handle {
        Handle::new()
    }
    
    fn normalize_path(&self, path: &str) -> String {
        // Convert to uppercase and normalize path separators
        path.to_uppercase().replace('/', "\\")
    }
    
    fn get_parent_directory(&self, path: &str) -> String {
        let normalized = self.normalize_path(path);
        if let Some(pos) = normalized.rfind('\\') {
            if pos == 0 {
                "\\".to_string()
            } else {
                normalized[..pos].to_string()
            }
        } else {
            "\\".to_string()
        }
    }
    
    fn get_filename(&self, path: &str) -> String {
        let normalized = self.normalize_path(path);
        if let Some(pos) = normalized.rfind('\\') {
            normalized[pos + 1..].to_string()
        } else {
            normalized
        }
    }
    
    fn current_time(&self) -> i64 {
        // Simplified - return fixed time
        0x01D6E1A0E1A0E1A0 // Some NT time value
    }
}

impl FileSystem for MemoryFileSystem {
    fn get_type(&self) -> FileSystemType {
        self.fs_type
    }
    
    fn get_volume_label(&self) -> String {
        self.volume_label.clone()
    }
    
    fn get_volume_size(&self) -> u64 {
        self.volume_size
    }
    
    fn get_free_space(&self) -> u64 {
        let used_space: u64 = self.files.values().map(|f| f.data.len() as u64).sum();
        self.volume_size.saturating_sub(used_space)
    }
    
    fn create_file(
        &mut self,
        path: &str,
        _desired_access: u32,
        file_attributes: FileAttributes,
        _share_access: u32,
        create_disposition: u32,
        _create_options: u32,
    ) -> Result<Handle, NtStatus> {
        let normalized_path = self.normalize_path(path);
        let parent_dir = self.get_parent_directory(&normalized_path);
        let filename = self.get_filename(&normalized_path);
        
        // Check if parent directory exists
        if !self.directories.contains_key(&parent_dir) {
            return Err(NtStatus::ObjectPathNotFound);
        }
        
        // Handle create disposition
        let file_exists = self.files.contains_key(&normalized_path);
        match create_disposition {
            1 => { // CREATE_NEW
                if file_exists {
                    return Err(NtStatus::ObjectNameCollision);
                }
            }
            2 => { // CREATE_ALWAYS
                // Always create, overwrite if exists
            }
            3 => { // OPEN_EXISTING
                if !file_exists {
                    return Err(NtStatus::ObjectNameNotFound);
                }
                return self.open_file(path, _desired_access, _share_access, 0);
            }
            4 => { // OPEN_ALWAYS
                if file_exists {
                    return self.open_file(path, _desired_access, _share_access, 0);
                }
            }
            5 => { // TRUNCATE_EXISTING
                if !file_exists {
                    return Err(NtStatus::ObjectNameNotFound);
                }
            }
            _ => return Err(NtStatus::InvalidParameter),
        }
        
        // Create the file
        let current_time = self.current_time();
        let file = MemoryFile {
            data: Vec::new(),
            attributes: file_attributes,
            creation_time: current_time,
            last_access_time: current_time,
            last_write_time: current_time,
            change_time: current_time,
            file_id: self.next_handle.fetch_add(1, Ordering::SeqCst),
        };
        
        self.files.insert(normalized_path.clone(), file);
        
        // Add to parent directory
        if let Some(parent) = self.directories.get_mut(&parent_dir) {
            if !parent.entries.contains(&filename) {
                parent.entries.push(filename);
            }
        }
        
        let handle = self.get_next_handle();
        self.open_handles.insert(handle, normalized_path);
        
        Ok(handle)
    }
    
    fn open_file(
        &mut self,
        path: &str,
        _desired_access: u32,
        _share_access: u32,
        _open_options: u32,
    ) -> Result<Handle, NtStatus> {
        let normalized_path = self.normalize_path(path);
        
        if !self.files.contains_key(&normalized_path) && !self.directories.contains_key(&normalized_path) {
            return Err(NtStatus::ObjectNameNotFound);
        }
        
        let handle = self.get_next_handle();
        self.open_handles.insert(handle, normalized_path);
        
        Ok(handle)
    }
    
    fn close_file(&mut self, handle: Handle) -> NtStatus {
        if self.open_handles.remove(&handle).is_some() {
            NtStatus::Success
        } else {
            NtStatus::InvalidHandle
        }
    }
    
    fn read_file(
        &mut self,
        handle: Handle,
        buffer: &mut [u8],
        offset: u64,
    ) -> Result<usize, NtStatus> {
        let path = self.open_handles.get(&handle)
            .ok_or(NtStatus::InvalidHandle)?
            .clone();
        
        let file = self.files.get_mut(&path)
            .ok_or(NtStatus::InvalidDeviceRequest)?;
        
        let start = offset as usize;
        if start >= file.data.len() {
            return Ok(0);
        }
        
        let end = core::cmp::min(start + buffer.len(), file.data.len());
        let bytes_to_read = end - start;
        
        buffer[..bytes_to_read].copy_from_slice(&file.data[start..end]);
        
        // Update last access time
        file.last_access_time = self.current_time();
        
        Ok(bytes_to_read)
    }
    
    fn write_file(
        &mut self,
        handle: Handle,
        buffer: &[u8],
        offset: u64,
    ) -> Result<usize, NtStatus> {
        let path = self.open_handles.get(&handle)
            .ok_or(NtStatus::InvalidHandle)?
            .clone();
        
        let file = self.files.get_mut(&path)
            .ok_or(NtStatus::InvalidDeviceRequest)?;
        
        let start = offset as usize;
        let end = start + buffer.len();
        
        // Extend file if necessary
        if end > file.data.len() {
            file.data.resize(end, 0);
        }
        
        file.data[start..end].copy_from_slice(buffer);
        
        // Update times
        let current_time = self.current_time();
        file.last_write_time = current_time;
        file.change_time = current_time;
        
        Ok(buffer.len())
    }
    
    fn delete_file(&mut self, path: &str) -> NtStatus {
        let normalized_path = self.normalize_path(path);
        let parent_dir = self.get_parent_directory(&normalized_path);
        let filename = self.get_filename(&normalized_path);
        
        if self.files.remove(&normalized_path).is_some() {
            // Remove from parent directory
            if let Some(parent) = self.directories.get_mut(&parent_dir) {
                parent.entries.retain(|name| name != &filename);
            }
            NtStatus::Success
        } else {
            NtStatus::ObjectNameNotFound
        }
    }
    
    fn rename_file(&mut self, old_path: &str, new_path: &str) -> NtStatus {
        let old_normalized = self.normalize_path(old_path);
        let new_normalized = self.normalize_path(new_path);
        
        if let Some(file) = self.files.remove(&old_normalized) {
            self.files.insert(new_normalized, file);
            
            // Update directory entries
            let old_parent = self.get_parent_directory(&old_normalized);
            let old_filename = self.get_filename(&old_normalized);
            let new_parent = self.get_parent_directory(&new_normalized);
            let new_filename = self.get_filename(&new_normalized);
            
            if let Some(parent) = self.directories.get_mut(&old_parent) {
                parent.entries.retain(|name| name != &old_filename);
            }
            
            if let Some(parent) = self.directories.get_mut(&new_parent) {
                parent.entries.push(new_filename);
            }
            
            NtStatus::Success
        } else {
            NtStatus::ObjectNameNotFound
        }
    }
    
    fn create_directory(&mut self, path: &str) -> NtStatus {
        let normalized_path = self.normalize_path(path);
        let parent_dir = self.get_parent_directory(&normalized_path);
        let dirname = self.get_filename(&normalized_path);
        
        if self.directories.contains_key(&normalized_path) {
            return NtStatus::ObjectNameCollision;
        }
        
        if !self.directories.contains_key(&parent_dir) {
            return NtStatus::ObjectPathNotFound;
        }
        
        let current_time = self.current_time();
        let directory = MemoryDirectory {
            attributes: FileAttributes::DIRECTORY,
            creation_time: current_time,
            last_access_time: current_time,
            last_write_time: current_time,
            change_time: current_time,
            entries: Vec::new(),
        };
        
        self.directories.insert(normalized_path, directory);
        
        // Add to parent directory
        if let Some(parent) = self.directories.get_mut(&parent_dir) {
            parent.entries.push(dirname);
        }
        
        NtStatus::Success
    }
    
    fn remove_directory(&mut self, path: &str) -> NtStatus {
        let normalized_path = self.normalize_path(path);
        let parent_dir = self.get_parent_directory(&normalized_path);
        let dirname = self.get_filename(&normalized_path);
        
        if let Some(directory) = self.directories.get(&normalized_path) {
            if !directory.entries.is_empty() {
                return NtStatus::DirectoryNotEmpty;
            }
        } else {
            return NtStatus::ObjectNameNotFound;
        }
        
        self.directories.remove(&normalized_path);
        
        // Remove from parent directory
        if let Some(parent) = self.directories.get_mut(&parent_dir) {
            parent.entries.retain(|name| name != &dirname);
        }
        
        NtStatus::Success
    }
    
    fn enumerate_directory(&self, path: &str) -> Result<Vec<DirectoryEntry>, NtStatus> {
        let normalized_path = self.normalize_path(path);
        
        let directory = self.directories.get(&normalized_path)
            .ok_or(NtStatus::ObjectNameNotFound)?;
        
        let mut entries = Vec::new();
        
        for entry_name in &directory.entries {
            let entry_path = if normalized_path == "\\" {
                format!("\\{}", entry_name)
            } else {
                format!("{}\\{}", normalized_path, entry_name)
            };
            
            if let Some(file) = self.files.get(&entry_path) {
                entries.push(DirectoryEntry {
                    name: entry_name.clone(),
                    file_attributes: file.attributes,
                    file_size: file.data.len() as u64,
                    creation_time: file.creation_time,
                    last_access_time: file.last_access_time,
                    last_write_time: file.last_write_time,
                    change_time: file.change_time,
                    file_id: file.file_id,
                    is_directory: false,
                });
            } else if let Some(dir) = self.directories.get(&entry_path) {
                entries.push(DirectoryEntry {
                    name: entry_name.clone(),
                    file_attributes: dir.attributes,
                    file_size: 0,
                    creation_time: dir.creation_time,
                    last_access_time: dir.last_access_time,
                    last_write_time: dir.last_write_time,
                    change_time: dir.change_time,
                    file_id: 0,
                    is_directory: true,
                });
            }
        }
        
        Ok(entries)
    }
    
    fn query_file_information(
        &self,
        handle: Handle,
        info_class: FileInformationClass,
    ) -> Result<Vec<u8>, NtStatus> {
        let path = self.open_handles.get(&handle)
            .ok_or(NtStatus::InvalidHandle)?;
        
        match info_class {
            FileInformationClass::FileBasicInformation => {
                if let Some(file) = self.files.get(path) {
                    let info = FileBasicInformation {
                        creation_time: file.creation_time,
                        last_access_time: file.last_access_time,
                        last_write_time: file.last_write_time,
                        change_time: file.change_time,
                        file_attributes: file.attributes,
                    };
                    
                    let bytes = unsafe {
                        core::slice::from_raw_parts(
                            &info as *const _ as *const u8,
                            core::mem::size_of::<FileBasicInformation>()
                        )
                    };
                    Ok(bytes.to_vec())
                } else {
                    Err(NtStatus::InvalidHandle)
                }
            }
            FileInformationClass::FileStandardInformation => {
                if let Some(file) = self.files.get(path) {
                    let info = FileStandardInformation {
                        allocation_size: file.data.len() as i64,
                        end_of_file: file.data.len() as i64,
                        number_of_links: 1,
                        delete_pending: false,
                        directory: false,
                    };
                    
                    let bytes = unsafe {
                        core::slice::from_raw_parts(
                            &info as *const _ as *const u8,
                            core::mem::size_of::<FileStandardInformation>()
                        )
                    };
                    Ok(bytes.to_vec())
                } else {
                    Err(NtStatus::InvalidHandle)
                }
            }
            _ => Err(NtStatus::InvalidInfoClass),
        }
    }
    
    fn set_file_information(
        &mut self,
        _handle: Handle,
        _info_class: FileInformationClass,
        _buffer: &[u8],
    ) -> NtStatus {
        // Simplified implementation
        NtStatus::NotImplemented
    }
}

// File system manager
pub struct FileSystemManager {
    mounted_volumes: BTreeMap<String, Box<dyn FileSystem>>,
    drive_letters: BTreeMap<char, String>,
}

impl FileSystemManager {
    pub fn new() -> Self {
        Self {
            mounted_volumes: BTreeMap::new(),
            drive_letters: BTreeMap::new(),
        }
    }
    
    pub fn mount_volume(
        &mut self,
        volume_path: String,
        filesystem: Box<dyn FileSystem>,
        drive_letter: Option<char>,
    ) -> NtStatus {
        if let Some(letter) = drive_letter {
            self.drive_letters.insert(letter, volume_path.clone());
        }
        
        self.mounted_volumes.insert(volume_path, filesystem);
        NtStatus::Success
    }
    
    pub fn unmount_volume(&mut self, volume_path: &str) -> NtStatus {
        if self.mounted_volumes.remove(volume_path).is_some() {
            // Remove drive letter mapping
            self.drive_letters.retain(|_, path| path != volume_path);
            NtStatus::Success
        } else {
            NtStatus::ObjectNameNotFound
        }
    }
    
    pub fn get_filesystem(&mut self, path: &str) -> Result<&mut Box<dyn FileSystem>, NtStatus> {
        // Parse drive letter or volume path
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive_letter = path.chars().nth(0).unwrap().to_ascii_uppercase();
            if let Some(volume_path) = self.drive_letters.get(&drive_letter).cloned() {
                if let Some(fs) = self.mounted_volumes.get_mut(&volume_path) {
                    return Ok(fs);
                }
            }
        }
        
        // Default to first mounted volume
        if let Some((_, fs)) = self.mounted_volumes.iter_mut().next() {
            Ok(fs)
        } else {
            Err(NtStatus::NoSuchDevice)
        }
    }
    
    pub fn resolve_path(&self, path: &str) -> (String, String) {
        // Returns (volume_path, relative_path)
        if path.len() >= 2 && path.chars().nth(1) == Some(':') {
            let drive_letter = path.chars().nth(0).unwrap().to_ascii_uppercase();
            if let Some(volume_path) = self.drive_letters.get(&drive_letter) {
                let relative_path = if path.len() > 3 {
                    path[3..].to_string()
                } else {
                    "\\".to_string()
                };
                return (volume_path.clone(), relative_path);
            }
        }
        
        // Default to first volume and full path
        if let Some((volume_path, _)) = self.mounted_volumes.iter().next() {
            (volume_path.clone(), path.to_string())
        } else {
            ("\\".to_string(), path.to_string())
        }
    }
    
    pub fn initialize_default_filesystems(&mut self) -> NtStatus {
        use crate::serial_println;
        
        serial_println!("FileSystem: Initializing default file systems");
        
        // Create C: drive with memory filesystem
        let c_drive = Box::new(MemoryFileSystem::new("System".to_string(), 1024 * 1024 * 1024)); // 1GB
        self.mount_volume("\\Device\\HarddiskVolume1".to_string(), c_drive, Some('C'))?;
        
        // Create some default directories and files
        if let Ok(fs) = self.get_filesystem("C:\\") {
            let _ = fs.create_directory("C:\\Windows");
            let _ = fs.create_directory("C:\\Windows\\System32");
            let _ = fs.create_directory("C:\\Program Files");
            let _ = fs.create_directory("C:\\Users");
            let _ = fs.create_directory("C:\\Temp");
            
            // Create some basic system files (empty for now)
            let _ = fs.create_file(
                "C:\\Windows\\System32\\kernel32.dll",
                0x80000000, // GENERIC_READ
                FileAttributes::SYSTEM,
                1, // FILE_SHARE_READ
                4, // OPEN_ALWAYS
                0,
            );
            
            let _ = fs.create_file(
                "C:\\Windows\\System32\\ntdll.dll",
                0x80000000,
                FileAttributes::SYSTEM,
                1,
                4,
                0,
            );
        }
        
        serial_println!("FileSystem: Default file systems initialized");
        NtStatus::Success
    }
}

// Global file system manager
lazy_static! {
    pub static ref FILESYSTEM_MANAGER: Mutex<FileSystemManager> = Mutex::new(FileSystemManager::new());
}

// Public API functions
pub fn mount_volume(
    volume_path: String,
    filesystem: Box<dyn FileSystem>,
    drive_letter: Option<char>,
) -> NtStatus {
    let mut fsm = FILESYSTEM_MANAGER.lock();
    fsm.mount_volume(volume_path, filesystem, drive_letter)
}

pub fn unmount_volume(volume_path: &str) -> NtStatus {
    let mut fsm = FILESYSTEM_MANAGER.lock();
    fsm.unmount_volume(volume_path)
}

pub fn initialize_default_filesystems() -> NtStatus {
    let mut fsm = FILESYSTEM_MANAGER.lock();
    fsm.initialize_default_filesystems()
}

// NT File API implementations
pub fn nt_create_file(
    file_handle: &mut Handle,
    desired_access: u32,
    object_attributes: &str,
    io_status_block: &mut IoStatusBlock,
    allocation_size: Option<u64>,
    file_attributes: FileAttributes,
    share_access: u32,
    create_disposition: u32,
    create_options: u32,
    ea_buffer: Option<&[u8]>,
    ea_length: u32,
) -> NtStatus {
    let mut fsm = FILESYSTEM_MANAGER.lock();
    
    if let Ok(fs) = fsm.get_filesystem(object_attributes) {
        match fs.create_file(
            object_attributes,
            desired_access,
            file_attributes,
            share_access,
            create_disposition,
            create_options,
        ) {
            Ok(handle) => {
                *file_handle = handle;
                io_status_block.status = NtStatus::Success;
                io_status_block.information = 1; // FILE_CREATED
                NtStatus::Success
            }
            Err(status) => {
                io_status_block.status = status;
                io_status_block.information = 0;
                status
            }
        }
    } else {
        io_status_block.status = NtStatus::NoSuchDevice;
        io_status_block.information = 0;
        NtStatus::NoSuchDevice
    }
}

pub fn nt_read_file(
    file_handle: Handle,
    event: Option<Handle>,
    apc_routine: Option<fn()>,
    apc_context: Option<*mut u8>,
    io_status_block: &mut IoStatusBlock,
    buffer: &mut [u8],
    byte_offset: Option<u64>,
    key: Option<u32>,
) -> NtStatus {
    let mut fsm = FILESYSTEM_MANAGER.lock();
    
    // Find which filesystem owns this handle (simplified)
    for (_, fs) in fsm.mounted_volumes.iter_mut() {
        match fs.read_file(file_handle, buffer, byte_offset.unwrap_or(0)) {
            Ok(bytes_read) => {
                io_status_block.status = NtStatus::Success;
                io_status_block.information = bytes_read;
                return NtStatus::Success;
            }
            Err(NtStatus::InvalidHandle) => continue,
            Err(status) => {
                io_status_block.status = status;
                io_status_block.information = 0;
                return status;
            }
        }
    }
    
    io_status_block.status = NtStatus::InvalidHandle;
    io_status_block.information = 0;
    NtStatus::InvalidHandle
}

pub fn nt_write_file(
    file_handle: Handle,
    event: Option<Handle>,
    apc_routine: Option<fn()>,
    apc_context: Option<*mut u8>,
    io_status_block: &mut IoStatusBlock,
    buffer: &[u8],
    byte_offset: Option<u64>,
    key: Option<u32>,
) -> NtStatus {
    let mut fsm = FILESYSTEM_MANAGER.lock();
    
    // Find which filesystem owns this handle (simplified)
    for (_, fs) in fsm.mounted_volumes.iter_mut() {
        match fs.write_file(file_handle, buffer, byte_offset.unwrap_or(0)) {
            Ok(bytes_written) => {
                io_status_block.status = NtStatus::Success;
                io_status_block.information = bytes_written;
                return NtStatus::Success;
            }
            Err(NtStatus::InvalidHandle) => continue,
            Err(status) => {
                io_status_block.status = status;
                io_status_block.information = 0;
                return status;
            }
        }
    }
    
    io_status_block.status = NtStatus::InvalidHandle;
    io_status_block.information = 0;
    NtStatus::InvalidHandle
}