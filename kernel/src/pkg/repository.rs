use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use super::{PackageError, Result};
use super::format::{PackageInfo, Version, Architecture};

const REPO_INDEX_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct Repository {
    config: RepositoryConfig,
    index: RepositoryIndex,
    packages: BTreeMap<String, Vec<PackageInfo>>,
    last_update: Option<u64>,
    enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub name: String,
    pub url: String,
    pub mirrors: Vec<String>,
    pub priority: u32,
    pub enabled: bool,
    pub gpg_check: bool,
    pub gpg_key: Option<String>,
    pub architecture: Vec<Architecture>,
    pub components: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryIndex {
    pub version: u32,
    pub timestamp: u64,
    pub packages: Vec<PackageEntry>,
    pub checksum: String,
    pub signature: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntry {
    pub info: PackageInfo,
    pub location: String,
    pub delta: Option<DeltaInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaInfo {
    pub from_version: Version,
    pub location: String,
    pub size: u64,
    pub checksum: String,
}

impl Repository {
    pub fn new(config: RepositoryConfig) -> Self {
        Self {
            config,
            index: RepositoryIndex {
                version: REPO_INDEX_VERSION,
                timestamp: 0,
                packages: Vec::new(),
                checksum: String::new(),
                signature: None,
            },
            packages: BTreeMap::new(),
            last_update: None,
            enabled: true,
        }
    }

    pub fn from_config(config: RepositoryConfig) -> Result<Self> {
        let mut repo = Self::new(config);
        repo.load_index()?;
        Ok(repo)
    }

    pub fn url(&self) -> &str {
        &self.config.url
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn priority(&self) -> u32 {
        self.config.priority
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled && self.config.enabled
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn update(&mut self) -> Result<()> {
        if !self.is_enabled() {
            return Ok(());
        }

        let index_data = self.fetch_index()?;
        let new_index: RepositoryIndex = serde_json::from_slice(&index_data)
            .map_err(|e| PackageError::InvalidFormat(format!("Invalid index: {:?}", e)))?;

        if self.config.gpg_check {
            self.verify_index(&new_index)?;
        }

        if new_index.timestamp <= self.index.timestamp {
            return Ok(());
        }

        self.index = new_index;
        self.rebuild_package_cache();
        self.last_update = Some(self.current_timestamp());

        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<PackageInfo> {
        if !self.is_enabled() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for versions in self.packages.values() {
            for pkg in versions {
                if pkg.name.to_lowercase().contains(&query_lower) ||
                   pkg.description.to_lowercase().contains(&query_lower) ||
                   pkg.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower)) {
                    results.push(pkg.clone());
                }
            }
        }

        results
    }

    pub fn get_package(&self, name: &str, version: Option<&Version>) -> Option<PackageInfo> {
        if !self.is_enabled() {
            return None;
        }

        self.packages.get(name).and_then(|versions| {
            if let Some(v) = version {
                versions.iter().find(|p| &p.version == v).cloned()
            } else {
                versions.first().cloned()
            }
        })
    }

    pub fn list_packages(&self) -> Vec<PackageInfo> {
        if !self.is_enabled() {
            return Vec::new();
        }

        self.packages.values()
            .flat_map(|versions| versions.iter().cloned())
            .collect()
    }

    pub fn download_package(&self, info: &PackageInfo) -> Result<Vec<u8>> {
        if !self.is_enabled() {
            return Err(PackageError::NetworkError("Repository disabled".to_string()));
        }

        let entry = self.index.packages.iter()
            .find(|e| e.info.name == info.name && e.info.version == info.version)
            .ok_or_else(|| PackageError::NotFound(info.name.clone()))?;

        let url = format!("{}/{}", self.config.url, entry.location);
        let data = self.fetch_url(&url)?;

        if self.config.gpg_check {
            self.verify_package(&data, info)?;
        }

        Ok(data)
    }

    pub fn download_delta(&self, from: &PackageInfo, to: &PackageInfo) -> Result<Option<Vec<u8>>> {
        if !self.is_enabled() {
            return Ok(None);
        }

        let entry = self.index.packages.iter()
            .find(|e| e.info.name == to.name && e.info.version == to.version)
            .ok_or_else(|| PackageError::NotFound(to.name.clone()))?;

        if let Some(delta) = &entry.delta {
            if delta.from_version == from.version {
                let url = format!("{}/{}", self.config.url, delta.location);
                let data = self.fetch_url(&url)?;
                
                if self.verify_checksum(&data, &delta.checksum)? {
                    return Ok(Some(data));
                }
            }
        }

        Ok(None)
    }

    pub fn get_mirrors(&self) -> &[String] {
        &self.config.mirrors
    }

    pub fn add_mirror(&mut self, mirror: String) {
        if !self.config.mirrors.contains(&mirror) {
            self.config.mirrors.push(mirror);
        }
    }

    pub fn remove_mirror(&mut self, mirror: &str) {
        self.config.mirrors.retain(|m| m != mirror);
    }

    fn load_index(&mut self) -> Result<()> {
        Ok(())
    }

    fn fetch_index(&self) -> Result<Vec<u8>> {
        let index_url = format!("{}/repodata/index.json", self.config.url);
        self.fetch_url(&index_url)
    }

    fn fetch_url(&self, url: &str) -> Result<Vec<u8>> {
        Ok(Vec::new())
    }

    fn verify_index(&self, index: &RepositoryIndex) -> Result<()> {
        if index.signature.is_none() {
            return Err(PackageError::SignatureVerificationFailed);
        }

        Ok(())
    }

    fn verify_package(&self, data: &[u8], info: &PackageInfo) -> Result<()> {
        self.verify_checksum(data, &info.checksum)?;
        
        if info.signature.is_some() {
        }

        Ok(())
    }

    fn verify_checksum(&self, data: &[u8], expected: &str) -> Result<bool> {
        let actual = self.calculate_checksum(data);
        Ok(actual == expected)
    }

    fn calculate_checksum(&self, data: &[u8]) -> String {
        let mut hash = [0u8; 32];
        for (i, byte) in data.iter().enumerate() {
            hash[i % 32] ^= byte;
        }
        hex::encode(&hash)
    }

    fn rebuild_package_cache(&mut self) {
        self.packages.clear();

        for entry in &self.index.packages {
            let arch_matches = self.config.architecture.is_empty() ||
                               self.config.architecture.contains(&entry.info.architecture) ||
                               entry.info.architecture == Architecture::Any;

            if arch_matches {
                self.packages.entry(entry.info.name.clone())
                    .or_insert_with(Vec::new)
                    .push(entry.info.clone());
            }
        }

        for versions in self.packages.values_mut() {
            versions.sort_by(|a, b| b.version.cmp(&a.version));
        }
    }

    fn current_timestamp(&self) -> u64 {
        0
    }
}

#[derive(Debug, Clone)]
pub struct RepositoryManager {
    repositories: Vec<Repository>,
    cache_dir: String,
}

impl RepositoryManager {
    pub fn new() -> Self {
        Self {
            repositories: Vec::new(),
            cache_dir: String::from("/var/cache/rpkg"),
        }
    }

    pub fn add_repository(&mut self, repo: Repository) -> Result<()> {
        if self.repositories.iter().any(|r| r.url() == repo.url()) {
            return Err(PackageError::DatabaseError("Repository already exists".to_string()));
        }

        self.repositories.push(repo);
        self.sort_by_priority();
        Ok(())
    }

    pub fn remove_repository(&mut self, url: &str) -> Result<()> {
        let initial_len = self.repositories.len();
        self.repositories.retain(|r| r.url() != url);
        
        if self.repositories.len() == initial_len {
            return Err(PackageError::NotFound(format!("Repository not found: {}", url)));
        }

        Ok(())
    }

    pub fn get_repository(&self, name: &str) -> Option<&Repository> {
        self.repositories.iter().find(|r| r.name() == name)
    }

    pub fn get_repository_mut(&mut self, name: &str) -> Option<&mut Repository> {
        self.repositories.iter_mut().find(|r| r.name() == name)
    }

    pub fn list_repositories(&self) -> &[Repository] {
        &self.repositories
    }

    pub fn update_all(&mut self) -> Result<Vec<String>> {
        let mut updated = Vec::new();

        for repo in &mut self.repositories {
            if repo.is_enabled() {
                match repo.update() {
                    Ok(()) => updated.push(repo.name().to_string()),
                    Err(e) => eprintln!("Failed to update {}: {}", repo.name(), e),
                }
            }
        }

        Ok(updated)
    }

    pub fn search_all(&self, query: &str) -> Vec<(String, Vec<PackageInfo>)> {
        let mut results = Vec::new();

        for repo in &self.repositories {
            let packages = repo.search(query);
            if !packages.is_empty() {
                results.push((repo.name().to_string(), packages));
            }
        }

        results
    }

    pub fn find_package(&self, name: &str, version: Option<&Version>) -> Option<(String, PackageInfo)> {
        for repo in &self.repositories {
            if let Some(pkg) = repo.get_package(name, version) {
                return Some((repo.name().to_string(), pkg));
            }
        }
        None
    }

    pub fn clean_cache(&self) -> Result<()> {
        Ok(())
    }

    fn sort_by_priority(&mut self) {
        self.repositories.sort_by_key(|r| r.priority());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMetadata {
    pub name: String,
    pub description: String,
    pub maintainer: String,
    pub homepage: String,
    pub timestamp: u64,
    pub packages_count: usize,
    pub total_size: u64,
    pub architectures: Vec<Architecture>,
    pub components: Vec<String>,
}

impl Repository {
    pub fn get_metadata(&self) -> RepoMetadata {
        let packages_count = self.index.packages.len();
        let total_size: u64 = self.index.packages.iter()
            .map(|e| e.info.size)
            .sum();

        let mut architectures = Vec::new();
        for entry in &self.index.packages {
            if !architectures.contains(&entry.info.architecture) {
                architectures.push(entry.info.architecture.clone());
            }
        }

        RepoMetadata {
            name: self.config.name.clone(),
            description: String::from("Package repository"),
            maintainer: String::from("System Administrator"),
            homepage: self.config.url.clone(),
            timestamp: self.index.timestamp,
            packages_count,
            total_size,
            architectures,
            components: self.config.components.clone(),
        }
    }

    pub fn export_index(&self) -> Result<String> {
        serde_json::to_string_pretty(&self.index)
            .map_err(|e| PackageError::InvalidFormat(format!("Failed to export index: {:?}", e)))
    }

    pub fn import_index(&mut self, data: &str) -> Result<()> {
        let index: RepositoryIndex = serde_json::from_str(data)
            .map_err(|e| PackageError::InvalidFormat(format!("Failed to import index: {:?}", e)))?;
        
        self.index = index;
        self.rebuild_package_cache();
        Ok(())
    }
}

mod hex {
    use alloc::string::String;

    pub fn encode(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for &b in bytes {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }
}