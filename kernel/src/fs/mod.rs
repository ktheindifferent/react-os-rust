pub mod fat32;
pub mod vfs;
pub mod file_ops;
pub mod ntfs;

use alloc::vec::Vec;
use alloc::string::String;

#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub path: String,
    pub data: Vec<u8>,
}

impl File {
    pub fn new(name: String, path: String) -> Self {
        Self {
            name,
            path,
            data: Vec::new(),
        }
    }
    
    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }
    
    pub fn read_all(&self) -> Result<Vec<u8>, FileSystemError> {
        Ok(self.data.clone())
    }
}

#[derive(Debug, Clone)]
pub enum FileType {
    Regular,
    Directory,
    SymLink,
    Device,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub name: String,
    pub size: u64,
    pub file_type: FileType,
    pub permissions: u32,
}

#[derive(Debug)]
pub enum FileSystemError {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidPath,
    IoError(String),
    NotSupported,
    FileNotFound,
}

pub trait FileSystem {
    fn read_file(&self, path: &str) -> Result<Vec<u8>, FileSystemError>;
    fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), FileSystemError>;
    fn create_directory(&mut self, path: &str) -> Result<(), FileSystemError>;
    fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>, FileSystemError>;
    fn delete(&mut self, path: &str) -> Result<(), FileSystemError>;
    fn get_file_info(&self, path: &str) -> Result<FileInfo, FileSystemError>;
}