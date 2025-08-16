// File operations - handles file I/O for processes
use alloc::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};
use spin::Mutex;
use lazy_static::lazy_static;
use super::{FileSystemError, FileInfo, FileType};
use crate::process::pcb::FileDescriptor;

// File access modes
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct FileMode: u32 {
        const READ = 0x1;
        const WRITE = 0x2;
        const APPEND = 0x4;
        const CREATE = 0x8;
        const TRUNCATE = 0x10;
        const EXCLUSIVE = 0x20;
    }
}

// File seek position
#[derive(Debug, Clone, Copy)]
pub enum SeekFrom {
    Start(u64),
    End(i64),
    Current(i64),
}

// Open file handle
#[derive(Debug)]
pub struct FileHandle {
    pub fd: i32,
    pub path: String,
    pub mode: FileMode,
    pub position: u64,
    pub inode: u64,
    pub size: u64,
}

impl FileHandle {
    pub fn new(fd: i32, path: String, mode: FileMode, inode: u64, size: u64) -> Self {
        Self {
            fd,
            path,
            mode,
            position: 0,
            inode,
            size,
        }
    }
    
    pub fn can_read(&self) -> bool {
        self.mode.contains(FileMode::READ)
    }
    
    pub fn can_write(&self) -> bool {
        self.mode.contains(FileMode::WRITE) || self.mode.contains(FileMode::APPEND)
    }
}

// File table - tracks all open files system-wide
pub struct FileTable {
    handles: BTreeMap<i32, FileHandle>,
    next_fd: i32,
}

impl FileTable {
    pub fn new() -> Self {
        Self {
            handles: BTreeMap::new(),
            next_fd: 3,  // 0=stdin, 1=stdout, 2=stderr
        }
    }
    
    pub fn open(&mut self, path: String, mode: FileMode) -> Result<i32, FileSystemError> {
        // Check if file exists (would query VFS)
        let inode = self.get_inode(&path)?;
        let size = self.get_file_size(&path)?;
        
        // Allocate file descriptor
        let fd = self.next_fd;
        self.next_fd += 1;
        
        // Create file handle
        let handle = FileHandle::new(fd, path, mode, inode, size);
        self.handles.insert(fd, handle);
        
        Ok(fd)
    }
    
    pub fn close(&mut self, fd: i32) -> Result<(), FileSystemError> {
        self.handles.remove(&fd)
            .ok_or(FileSystemError::NotFound)?;
        Ok(())
    }
    
    pub fn read(&mut self, fd: i32, buffer: &mut [u8]) -> Result<usize, FileSystemError> {
        let handle = self.handles.get_mut(&fd)
            .ok_or(FileSystemError::NotFound)?;
        
        if !handle.can_read() {
            return Err(FileSystemError::PermissionDenied);
        }
        
        // Read from VFS
        use super::vfs::VFS;
        let vfs = VFS.lock();
        let data = vfs.read_file(&handle.path)?;
        
        // Read from current position
        let start = handle.position as usize;
        let available = data.len().saturating_sub(start);
        let to_read = buffer.len().min(available);
        
        if to_read > 0 {
            buffer[..to_read].copy_from_slice(&data[start..start + to_read]);
            handle.position += to_read as u64;
        }
        
        Ok(to_read)
    }
    
    pub fn write(&mut self, fd: i32, data: &[u8]) -> Result<usize, FileSystemError> {
        let handle = self.handles.get_mut(&fd)
            .ok_or(FileSystemError::NotFound)?;
        
        if !handle.can_write() {
            return Err(FileSystemError::PermissionDenied);
        }
        
        // Write to VFS
        use super::vfs::VFS;
        let mut vfs = VFS.lock();
        
        if handle.mode.contains(FileMode::APPEND) {
            handle.position = handle.size;
        }
        
        // This would write at position
        vfs.write_file(&handle.path, data)?;
        
        handle.position += data.len() as u64;
        handle.size = handle.size.max(handle.position);
        
        Ok(data.len())
    }
    
    pub fn seek(&mut self, fd: i32, pos: SeekFrom) -> Result<u64, FileSystemError> {
        let handle = self.handles.get_mut(&fd)
            .ok_or(FileSystemError::NotFound)?;
        
        let new_pos = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::End(offset) => {
                if offset < 0 {
                    handle.size.saturating_sub((-offset) as u64)
                } else {
                    handle.size + offset as u64
                }
            },
            SeekFrom::Current(offset) => {
                if offset < 0 {
                    handle.position.saturating_sub((-offset) as u64)
                } else {
                    handle.position + offset as u64
                }
            },
        };
        
        handle.position = new_pos.min(handle.size);
        Ok(handle.position)
    }
    
    pub fn get_handle(&self, fd: i32) -> Option<&FileHandle> {
        self.handles.get(&fd)
    }
    
    fn get_inode(&self, _path: &str) -> Result<u64, FileSystemError> {
        // This would query the VFS for the inode
        Ok(1)  // Dummy inode
    }
    
    fn get_file_size(&self, _path: &str) -> Result<u64, FileSystemError> {
        // This would query the VFS for file size
        Ok(0)  // Dummy size
    }
}

lazy_static! {
    pub static ref FILE_TABLE: Mutex<FileTable> = Mutex::new(FileTable::new());
}

// Standard I/O file descriptors
pub const STDIN_FD: i32 = 0;
pub const STDOUT_FD: i32 = 1;
pub const STDERR_FD: i32 = 2;

// File system calls for processes
pub fn sys_open(path: &str, flags: u32) -> Result<i32, FileSystemError> {
    let mode = FileMode::from_bits_truncate(flags);
    FILE_TABLE.lock().open(String::from(path), mode)
}

pub fn sys_close(fd: i32) -> Result<(), FileSystemError> {
    FILE_TABLE.lock().close(fd)
}

pub fn sys_read(fd: i32, buffer: &mut [u8]) -> Result<usize, FileSystemError> {
    // Handle special file descriptors
    match fd {
        STDIN_FD => {
            // Read from keyboard buffer
            Ok(0)  // No input available
        },
        _ => FILE_TABLE.lock().read(fd, buffer),
    }
}

pub fn sys_write(fd: i32, data: &[u8]) -> Result<usize, FileSystemError> {
    // Handle special file descriptors
    match fd {
        STDOUT_FD | STDERR_FD => {
            // Write to console
            for &byte in data {
                crate::print!("{}", byte as char);
            }
            Ok(data.len())
        },
        _ => FILE_TABLE.lock().write(fd, data),
    }
}

pub fn sys_seek(fd: i32, offset: i64, whence: i32) -> Result<u64, FileSystemError> {
    let pos = match whence {
        0 => SeekFrom::Start(offset as u64),
        1 => SeekFrom::Current(offset),
        2 => SeekFrom::End(offset),
        _ => return Err(FileSystemError::InvalidPath),
    };
    
    FILE_TABLE.lock().seek(fd, pos)
}

// Directory operations
pub fn sys_mkdir(path: &str, _mode: u32) -> Result<(), FileSystemError> {
    use super::vfs::VFS;
    let mut vfs = VFS.lock();
    
    // This would delegate to the appropriate filesystem
    // For now, return not implemented
    Err(FileSystemError::IoError(String::from("Not implemented")))
}

pub fn sys_readdir(path: &str) -> Result<Vec<FileInfo>, FileSystemError> {
    use super::vfs::VFS;
    let vfs = VFS.lock();
    vfs.list_directory(path)
}

pub fn sys_stat(path: &str) -> Result<FileInfo, FileSystemError> {
    use super::vfs::VFS;
    let vfs = VFS.lock();
    
    // This would get file info from VFS
    // For now, return dummy info
    Ok(FileInfo {
        name: String::from(path),
        size: 0,
        file_type: FileType::Regular,
        permissions: 0o644,
    })
}