use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::fmt;
use serde::{Serialize, Deserialize};
use super::{PackageError, Result, format::{PackageInfo, Version}};

const DB_VERSION: u32 = 1;
const DB_PATH: &str = "/var/lib/rpkg/db";
const DB_LOCK_PATH: &str = "/var/lib/rpkg/db.lock";
const TRANSACTION_LOG: &str = "/var/lib/rpkg/transactions.log";

#[derive(Debug, Clone)]
pub struct PackageDatabase {
    packages: BTreeMap<String, InstalledPackage>,
    file_index: BTreeMap<String, String>,
    dependency_graph: DependencyGraph,
    transactions: Vec<Transaction>,
    version: u32,
    locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub info: PackageInfo,
    pub files: Vec<InstalledFile>,
    pub install_date: u64,
    pub install_reason: InstallReason,
    pub config_files: Vec<ConfigFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledFile {
    pub path: String,
    pub size: u64,
    pub mode: u32,
    pub checksum: String,
    pub modified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub path: String,
    pub original_checksum: String,
    pub current_checksum: String,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InstallReason {
    Explicit,
    Dependency,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: u64,
    pub timestamp: u64,
    pub operation: TransactionOp,
    pub packages: Vec<String>,
    pub status: TransactionStatus,
    pub rollback_id: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionOp {
    Install,
    Remove,
    Upgrade,
    Downgrade,
    Rollback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionStatus {
    Pending,
    InProgress,
    Completed,
    Failed(String),
    RolledBack,
}

#[derive(Debug, Clone)]
pub struct DependencyGraph {
    forward: BTreeMap<String, Vec<String>>,
    reverse: BTreeMap<String, Vec<String>>,
    provides: BTreeMap<String, Vec<String>>,
}

impl PackageDatabase {
    pub fn new() -> Self {
        Self {
            packages: BTreeMap::new(),
            file_index: BTreeMap::new(),
            dependency_graph: DependencyGraph::new(),
            transactions: Vec::new(),
            version: DB_VERSION,
            locked: false,
        }
    }

    pub fn init(&mut self) -> Result<()> {
        self.ensure_directories()?;
        self.load_or_create()?;
        Ok(())
    }

    pub fn lock(&mut self) -> Result<()> {
        if self.locked {
            return Err(PackageError::DatabaseError("Database already locked".to_string()));
        }
        
        self.locked = true;
        Ok(())
    }

    pub fn unlock(&mut self) -> Result<()> {
        if !self.locked {
            return Ok(());
        }
        
        self.locked = false;
        Ok(())
    }

    pub fn add_package(&mut self, package: InstalledPackage) -> Result<()> {
        if self.packages.contains_key(&package.info.name) {
            return Err(PackageError::AlreadyInstalled(package.info.name.clone()));
        }

        for file in &package.files {
            if let Some(owner) = self.file_index.get(&file.path) {
                return Err(PackageError::DependencyConflict(
                    format!("File {} already owned by {}", file.path, owner)
                ));
            }
        }

        for file in &package.files {
            self.file_index.insert(file.path.clone(), package.info.name.clone());
        }

        self.dependency_graph.add_package(&package.info);
        self.packages.insert(package.info.name.clone(), package);
        
        self.save()?;
        Ok(())
    }

    pub fn remove_package(&mut self, name: &str) -> Result<InstalledPackage> {
        let package = self.packages.remove(name)
            .ok_or_else(|| PackageError::NotFound(name.to_string()))?;

        for file in &package.files {
            self.file_index.remove(&file.path);
        }

        self.dependency_graph.remove_package(name);
        
        self.save()?;
        Ok(package)
    }

    pub fn get_package(&self, name: &str) -> Option<PackageInfo> {
        self.packages.get(name).map(|p| p.info.clone())
    }

    pub fn list_installed(&self) -> Vec<PackageInfo> {
        self.packages.values().map(|p| p.info.clone()).collect()
    }

    pub fn get_file_owner(&self, path: &str) -> Option<&String> {
        self.file_index.get(path)
    }

    pub fn find_orphans(&self) -> Vec<String> {
        let mut orphans = Vec::new();
        
        for (name, package) in &self.packages {
            if package.install_reason == InstallReason::Dependency {
                if self.dependency_graph.get_reverse_deps(name).is_empty() {
                    orphans.push(name.clone());
                }
            }
        }
        
        orphans
    }

    pub fn get_dependencies(&self, name: &str) -> Vec<String> {
        self.dependency_graph.get_forward_deps(name)
    }

    pub fn get_dependents(&self, name: &str) -> Vec<String> {
        self.dependency_graph.get_reverse_deps(name)
    }

    pub fn verify_package(&self, name: &str) -> Result<Vec<String>> {
        let package = self.packages.get(name)
            .ok_or_else(|| PackageError::NotFound(name.to_string()))?;

        let mut issues = Vec::new();

        for file in &package.files {
            if !self.verify_file(file)? {
                issues.push(format!("File modified: {}", file.path));
            }
        }

        Ok(issues)
    }

    pub fn begin_transaction(&mut self, op: TransactionOp, packages: Vec<String>) -> Result<u64> {
        let id = self.transactions.len() as u64 + 1;
        
        let transaction = Transaction {
            id,
            timestamp: self.current_timestamp(),
            operation: op,
            packages,
            status: TransactionStatus::Pending,
            rollback_id: None,
        };

        self.transactions.push(transaction);
        self.save_transaction_log()?;
        
        Ok(id)
    }

    pub fn commit_transaction(&mut self, id: u64) -> Result<()> {
        let transaction = self.transactions.iter_mut()
            .find(|t| t.id == id)
            .ok_or_else(|| PackageError::DatabaseError("Transaction not found".to_string()))?;

        if transaction.status != TransactionStatus::InProgress {
            return Err(PackageError::DatabaseError("Invalid transaction state".to_string()));
        }

        transaction.status = TransactionStatus::Completed;
        self.save_transaction_log()?;
        
        Ok(())
    }

    pub fn rollback_transaction(&mut self, id: u64) -> Result<()> {
        let transaction = self.transactions.iter()
            .find(|t| t.id == id)
            .ok_or_else(|| PackageError::DatabaseError("Transaction not found".to_string()))?
            .clone();

        if transaction.status != TransactionStatus::Completed {
            return Err(PackageError::DatabaseError("Can only rollback completed transactions".to_string()));
        }

        let rollback_id = self.begin_transaction(
            TransactionOp::Rollback,
            transaction.packages.clone()
        )?;

        match transaction.operation {
            TransactionOp::Install => {
                for package in &transaction.packages {
                    self.remove_package(package)?;
                }
            }
            TransactionOp::Remove => {
            }
            TransactionOp::Upgrade | TransactionOp::Downgrade => {
            }
            _ => {}
        }

        self.commit_transaction(rollback_id)?;
        
        if let Some(t) = self.transactions.iter_mut().find(|t| t.id == id) {
            t.status = TransactionStatus::RolledBack;
        }

        Ok(())
    }

    pub fn get_transaction_history(&self) -> &[Transaction] {
        &self.transactions
    }

    fn ensure_directories(&self) -> Result<()> {
        Ok(())
    }

    fn load_or_create(&mut self) -> Result<()> {
        Ok(())
    }

    fn save(&self) -> Result<()> {
        Ok(())
    }

    fn save_transaction_log(&self) -> Result<()> {
        Ok(())
    }

    fn verify_file(&self, file: &InstalledFile) -> Result<bool> {
        Ok(!file.modified)
    }

    fn current_timestamp(&self) -> u64 {
        0
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            forward: BTreeMap::new(),
            reverse: BTreeMap::new(),
            provides: BTreeMap::new(),
        }
    }

    pub fn add_package(&mut self, info: &PackageInfo) {
        let deps: Vec<String> = info.dependencies.iter()
            .map(|d| d.name.clone())
            .collect();
        
        self.forward.insert(info.name.clone(), deps.clone());
        
        for dep in deps {
            self.reverse.entry(dep)
                .or_insert_with(Vec::new)
                .push(info.name.clone());
        }

        for provided in &info.provides {
            self.provides.entry(provided.clone())
                .or_insert_with(Vec::new)
                .push(info.name.clone());
        }
    }

    pub fn remove_package(&mut self, name: &str) {
        if let Some(deps) = self.forward.remove(name) {
            for dep in deps {
                if let Some(rev_deps) = self.reverse.get_mut(&dep) {
                    rev_deps.retain(|n| n != name);
                }
            }
        }

        self.provides.retain(|_, providers| {
            providers.retain(|n| n != name);
            !providers.is_empty()
        });
    }

    pub fn get_forward_deps(&self, name: &str) -> Vec<String> {
        self.forward.get(name).cloned().unwrap_or_default()
    }

    pub fn get_reverse_deps(&self, name: &str) -> Vec<String> {
        self.reverse.get(name).cloned().unwrap_or_default()
    }

    pub fn get_providers(&self, name: &str) -> Vec<String> {
        self.provides.get(name).cloned().unwrap_or_default()
    }

    pub fn has_circular_dependency(&self, from: &str, to: &str) -> bool {
        let mut visited = Vec::new();
        self.dfs_circular(from, to, &mut visited)
    }

    fn dfs_circular(&self, current: &str, target: &str, visited: &mut Vec<String>) -> bool {
        if current == target && !visited.is_empty() {
            return true;
        }

        if visited.contains(&current.to_string()) {
            return false;
        }

        visited.push(current.to_string());

        if let Some(deps) = self.forward.get(current) {
            for dep in deps {
                if self.dfs_circular(dep, target, visited) {
                    return true;
                }
            }
        }

        visited.pop();
        false
    }
}

pub struct DatabaseIterator<'a> {
    packages: alloc::vec::IntoIter<(&'a String, &'a InstalledPackage)>,
}

impl<'a> Iterator for DatabaseIterator<'a> {
    type Item = &'a InstalledPackage;

    fn next(&mut self) -> Option<Self::Item> {
        self.packages.next().map(|(_, pkg)| pkg)
    }
}

impl PackageDatabase {
    pub fn iter(&self) -> DatabaseIterator {
        DatabaseIterator {
            packages: self.packages.iter().collect::<Vec<_>>().into_iter(),
        }
    }

    pub fn search(&self, query: &str) -> Vec<PackageInfo> {
        let query_lower = query.to_lowercase();
        
        self.packages.values()
            .filter(|p| {
                p.info.name.to_lowercase().contains(&query_lower) ||
                p.info.description.to_lowercase().contains(&query_lower) ||
                p.info.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower))
            })
            .map(|p| p.info.clone())
            .collect()
    }

    pub fn get_stats(&self) -> DatabaseStats {
        let total_packages = self.packages.len();
        let total_size: u64 = self.packages.values()
            .map(|p| p.info.installed_size)
            .sum();
        
        let explicit_count = self.packages.values()
            .filter(|p| p.install_reason == InstallReason::Explicit)
            .count();
        
        let dependency_count = self.packages.values()
            .filter(|p| p.install_reason == InstallReason::Dependency)
            .count();

        DatabaseStats {
            total_packages,
            total_size,
            explicit_count,
            dependency_count,
            orphan_count: self.find_orphans().len(),
            transaction_count: self.transactions.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DatabaseStats {
    pub total_packages: usize,
    pub total_size: u64,
    pub explicit_count: usize,
    pub dependency_count: usize,
    pub orphan_count: usize,
    pub transaction_count: usize,
}

impl fmt::Display for DatabaseStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Database Statistics:")?;
        writeln!(f, "  Total packages: {}", self.total_packages)?;
        writeln!(f, "  Total size: {} bytes", self.total_size)?;
        writeln!(f, "  Explicitly installed: {}", self.explicit_count)?;
        writeln!(f, "  Dependencies: {}", self.dependency_count)?;
        writeln!(f, "  Orphaned packages: {}", self.orphan_count)?;
        writeln!(f, "  Transactions: {}", self.transaction_count)?;
        Ok(())
    }
}