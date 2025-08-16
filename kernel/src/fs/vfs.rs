use super::{FileSystem, FileSystemError, FileInfo};
use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

pub struct VirtualFileSystem {
    filesystems: Vec<(String, Box<dyn FileSystem + Send + Sync>)>,
}

impl VirtualFileSystem {
    pub fn new() -> Self {
        Self {
            filesystems: Vec::new(),
        }
    }

    pub fn mount(&mut self, mount_point: String, fs: Box<dyn FileSystem + Send + Sync>) {
        self.filesystems.push((mount_point, fs));
    }

    fn find_filesystem<'a>(&'a self, path: &'a str) -> Option<(&'a dyn FileSystem, &'a str)> {
        for (mount_point, fs) in &self.filesystems {
            if path.starts_with(mount_point.as_str()) {
                let relative_path = &path[mount_point.len()..];
                return Some((fs.as_ref(), relative_path));
            }
        }
        None
    }

    fn find_filesystem_mut<'a>(&'a mut self, path: &'a str) -> Option<(&'a mut dyn FileSystem, &'a str)> {
        for (mount_point, fs) in &mut self.filesystems {
            if path.starts_with(mount_point.as_str()) {
                let relative_path = &path[mount_point.len()..];
                return Some((fs.as_mut(), relative_path));
            }
        }
        None
    }

    pub fn read_file(&self, path: &str) -> Result<Vec<u8>, FileSystemError> {
        if let Some((fs, relative_path)) = self.find_filesystem(path) {
            fs.read_file(relative_path)
        } else {
            Err(FileSystemError::NotFound)
        }
    }

    pub fn write_file(&mut self, path: &str, data: &[u8]) -> Result<(), FileSystemError> {
        if let Some((fs, relative_path)) = self.find_filesystem_mut(path) {
            fs.write_file(relative_path, data)
        } else {
            Err(FileSystemError::NotFound)
        }
    }

    pub fn list_directory(&self, path: &str) -> Result<Vec<FileInfo>, FileSystemError> {
        if let Some((fs, relative_path)) = self.find_filesystem(path) {
            fs.list_directory(relative_path)
        } else {
            Err(FileSystemError::NotFound)
        }
    }
}

lazy_static! {
    pub static ref VFS: Mutex<VirtualFileSystem> = Mutex::new(VirtualFileSystem::new());
}