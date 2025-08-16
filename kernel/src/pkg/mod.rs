pub mod format;
pub mod database;
pub mod resolver;
pub mod repository;
pub mod operations;
pub mod signature;
pub mod cache;
pub mod config;
pub mod update;
pub mod build;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageError {
    NotFound(String),
    AlreadyInstalled(String),
    DependencyConflict(String),
    InvalidFormat(String),
    CorruptedPackage(String),
    SignatureVerificationFailed,
    InsufficientSpace,
    PermissionDenied,
    NetworkError(String),
    DatabaseError(String),
    IoError(String),
}

impl fmt::Display for PackageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(pkg) => write!(f, "Package not found: {}", pkg),
            Self::AlreadyInstalled(pkg) => write!(f, "Package already installed: {}", pkg),
            Self::DependencyConflict(msg) => write!(f, "Dependency conflict: {}", msg),
            Self::InvalidFormat(msg) => write!(f, "Invalid package format: {}", msg),
            Self::CorruptedPackage(msg) => write!(f, "Corrupted package: {}", msg),
            Self::SignatureVerificationFailed => write!(f, "Package signature verification failed"),
            Self::InsufficientSpace => write!(f, "Insufficient disk space"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::NetworkError(msg) => write!(f, "Network error: {}", msg),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

pub type Result<T> = core::result::Result<T, PackageError>;

#[derive(Debug, Clone)]
pub struct PackageManager {
    database: database::PackageDatabase,
    repositories: Vec<repository::Repository>,
    cache: cache::PackageCache,
    config: config::PackageConfig,
}

impl PackageManager {
    pub fn new() -> Self {
        Self {
            database: database::PackageDatabase::new(),
            repositories: Vec::new(),
            cache: cache::PackageCache::new(),
            config: config::PackageConfig::default(),
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.database.init()?;
        self.cache.init()?;
        self.load_repositories()?;
        Ok(())
    }

    pub fn add_repository(&mut self, repo: repository::Repository) -> Result<()> {
        if !self.repositories.iter().any(|r| r.url() == repo.url()) {
            self.repositories.push(repo);
            self.update_repository_index()?;
        }
        Ok(())
    }

    pub fn remove_repository(&mut self, url: &str) -> Result<()> {
        self.repositories.retain(|r| r.url() != url);
        Ok(())
    }

    pub fn update_repositories(&mut self) -> Result<()> {
        for repo in &mut self.repositories {
            repo.update()?;
        }
        self.update_repository_index()?;
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<format::PackageInfo> {
        let mut results = Vec::new();
        for repo in &self.repositories {
            results.extend(repo.search(query));
        }
        results
    }

    pub fn install(&mut self, package_name: &str) -> Result<()> {
        operations::install(self, package_name)
    }

    pub fn remove(&mut self, package_name: &str) -> Result<()> {
        operations::remove(self, package_name)
    }

    pub fn upgrade(&mut self, package_name: Option<&str>) -> Result<()> {
        operations::upgrade(self, package_name)
    }

    pub fn list_installed(&self) -> Vec<format::PackageInfo> {
        self.database.list_installed()
    }

    pub fn get_package_info(&self, name: &str) -> Option<format::PackageInfo> {
        self.database.get_package(name)
    }

    pub fn verify_package(&self, path: &str) -> Result<bool> {
        signature::verify_package(path)
    }

    pub fn clean_cache(&mut self) -> Result<()> {
        self.cache.clean()
    }

    fn load_repositories(&mut self) -> Result<()> {
        let repo_configs = self.config.get_repositories();
        for config in repo_configs {
            let repo = repository::Repository::from_config(config)?;
            self.repositories.push(repo);
        }
        Ok(())
    }

    fn update_repository_index(&mut self) -> Result<()> {
        for repo in &self.repositories {
            self.cache.update_index(repo)?;
        }
        Ok(())
    }
}

pub fn init() -> Result<PackageManager> {
    let mut manager = PackageManager::new();
    manager.init()?;
    Ok(manager)
}