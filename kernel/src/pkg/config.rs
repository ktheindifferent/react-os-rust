use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use super::{PackageError, Result};
use super::repository::RepositoryConfig;
use super::format::Architecture;

const CONFIG_PATH: &str = "/etc/rpkg/config.toml";
const REPOS_DIR: &str = "/etc/rpkg/repos.d";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageConfig {
    pub general: GeneralConfig,
    pub repositories: Vec<RepositoryConfig>,
    pub cache: CacheConfig,
    pub network: NetworkConfig,
    pub security: SecurityConfig,
    pub hooks: HookConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub root_dir: String,
    pub db_path: String,
    pub log_level: LogLevel,
    pub architecture: Architecture,
    pub parallel_downloads: usize,
    pub color_output: bool,
    pub confirm_actions: bool,
    pub keep_old_packages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub dir: String,
    pub max_size: u64,
    pub clean_on_exit: bool,
    pub package_cache_days: u32,
    pub index_cache_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub timeout_seconds: u32,
    pub retry_count: u32,
    pub proxy: Option<String>,
    pub user_agent: String,
    pub bandwidth_limit: Option<u64>,
    pub mirror_selection: MirrorSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MirrorSelection {
    First,
    Random,
    Fastest,
    GeoIP,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub verify_signatures: bool,
    pub verify_checksums: bool,
    pub trusted_keys: Vec<String>,
    pub allow_downgrade: bool,
    pub allow_untrusted: bool,
    pub sandbox_scripts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    pub pre_transaction: Vec<String>,
    pub post_transaction: Vec<String>,
    pub pre_install: Vec<String>,
    pub post_install: Vec<String>,
    pub pre_remove: Vec<String>,
    pub post_remove: Vec<String>,
    pub pre_upgrade: Vec<String>,
    pub post_upgrade: Vec<String>,
}

impl Default for PackageConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                root_dir: String::from("/"),
                db_path: String::from("/var/lib/rpkg/db"),
                log_level: LogLevel::Info,
                architecture: Architecture::X86_64,
                parallel_downloads: 4,
                color_output: true,
                confirm_actions: true,
                keep_old_packages: 2,
            },
            repositories: Vec::new(),
            cache: CacheConfig {
                dir: String::from("/var/cache/rpkg"),
                max_size: 1024 * 1024 * 1024,
                clean_on_exit: false,
                package_cache_days: 30,
                index_cache_minutes: 60,
            },
            network: NetworkConfig {
                timeout_seconds: 30,
                retry_count: 3,
                proxy: None,
                user_agent: String::from("rpkg/1.0"),
                bandwidth_limit: None,
                mirror_selection: MirrorSelection::Fastest,
            },
            security: SecurityConfig {
                verify_signatures: true,
                verify_checksums: true,
                trusted_keys: Vec::new(),
                allow_downgrade: false,
                allow_untrusted: false,
                sandbox_scripts: true,
            },
            hooks: HookConfig {
                pre_transaction: Vec::new(),
                post_transaction: Vec::new(),
                pre_install: Vec::new(),
                post_install: Vec::new(),
                pre_remove: Vec::new(),
                post_remove: Vec::new(),
                pre_upgrade: Vec::new(),
                post_upgrade: Vec::new(),
            },
        }
    }
}

impl PackageConfig {
    pub fn load() -> Result<Self> {
        Self::load_from_file(CONFIG_PATH)
    }

    pub fn load_from_file(path: &str) -> Result<Self> {
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<()> {
        self.save_to_file(CONFIG_PATH)
    }

    pub fn save_to_file(&self, path: &str) -> Result<()> {
        Ok(())
    }

    pub fn get_repositories(&self) -> Vec<RepositoryConfig> {
        let mut repos = self.repositories.clone();
        
        repos.sort_by_key(|r| r.priority);
        repos
    }

    pub fn add_repository(&mut self, repo: RepositoryConfig) -> Result<()> {
        if self.repositories.iter().any(|r| r.url == repo.url) {
            return Err(PackageError::DatabaseError("Repository already exists".to_string()));
        }
        
        self.repositories.push(repo);
        Ok(())
    }

    pub fn remove_repository(&mut self, url: &str) -> Result<()> {
        let initial_len = self.repositories.len();
        self.repositories.retain(|r| r.url != url);
        
        if self.repositories.len() == initial_len {
            return Err(PackageError::NotFound(format!("Repository not found: {}", url)));
        }
        
        Ok(())
    }

    pub fn validate(&self) -> Result<()> {
        if self.general.parallel_downloads == 0 {
            return Err(PackageError::InvalidFormat("parallel_downloads must be > 0".to_string()));
        }
        
        if self.network.timeout_seconds == 0 {
            return Err(PackageError::InvalidFormat("timeout_seconds must be > 0".to_string()));
        }
        
        Ok(())
    }

    pub fn merge(&mut self, other: PackageConfig) {
        for repo in other.repositories {
            if !self.repositories.iter().any(|r| r.url == repo.url) {
                self.repositories.push(repo);
            }
        }
        
        self.security.trusted_keys.extend(other.security.trusted_keys);
        self.security.trusted_keys.sort();
        self.security.trusted_keys.dedup();
    }
}

pub struct ConfigBuilder {
    config: PackageConfig,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: PackageConfig::default(),
        }
    }

    pub fn root_dir(mut self, dir: String) -> Self {
        self.config.general.root_dir = dir;
        self
    }

    pub fn architecture(mut self, arch: Architecture) -> Self {
        self.config.general.architecture = arch;
        self
    }

    pub fn cache_dir(mut self, dir: String) -> Self {
        self.config.cache.dir = dir;
        self
    }

    pub fn cache_size(mut self, size: u64) -> Self {
        self.config.cache.max_size = size;
        self
    }

    pub fn parallel_downloads(mut self, count: usize) -> Self {
        self.config.general.parallel_downloads = count;
        self
    }

    pub fn verify_signatures(mut self, verify: bool) -> Self {
        self.config.security.verify_signatures = verify;
        self
    }

    pub fn add_repository(mut self, repo: RepositoryConfig) -> Self {
        self.config.repositories.push(repo);
        self
    }

    pub fn add_hook(mut self, event: HookEvent, command: String) -> Self {
        match event {
            HookEvent::PreTransaction => self.config.hooks.pre_transaction.push(command),
            HookEvent::PostTransaction => self.config.hooks.post_transaction.push(command),
            HookEvent::PreInstall => self.config.hooks.pre_install.push(command),
            HookEvent::PostInstall => self.config.hooks.post_install.push(command),
            HookEvent::PreRemove => self.config.hooks.pre_remove.push(command),
            HookEvent::PostRemove => self.config.hooks.post_remove.push(command),
            HookEvent::PreUpgrade => self.config.hooks.pre_upgrade.push(command),
            HookEvent::PostUpgrade => self.config.hooks.post_upgrade.push(command),
        }
        self
    }

    pub fn build(self) -> Result<PackageConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HookEvent {
    PreTransaction,
    PostTransaction,
    PreInstall,
    PostInstall,
    PreRemove,
    PostRemove,
    PreUpgrade,
    PostUpgrade,
}

impl PackageConfig {
    pub fn run_hooks(&self, event: HookEvent) -> Result<()> {
        let commands = match event {
            HookEvent::PreTransaction => &self.hooks.pre_transaction,
            HookEvent::PostTransaction => &self.hooks.post_transaction,
            HookEvent::PreInstall => &self.hooks.pre_install,
            HookEvent::PostInstall => &self.hooks.post_install,
            HookEvent::PreRemove => &self.hooks.pre_remove,
            HookEvent::PostRemove => &self.hooks.post_remove,
            HookEvent::PreUpgrade => &self.hooks.pre_upgrade,
            HookEvent::PostUpgrade => &self.hooks.post_upgrade,
        };
        
        for command in commands {
            self.run_hook_command(command)?;
        }
        
        Ok(())
    }

    fn run_hook_command(&self, command: &str) -> Result<()> {
        println!("Running hook: {}", command);
        Ok(())
    }
}

pub fn get_system_architecture() -> Architecture {
    #[cfg(target_arch = "x86_64")]
    return Architecture::X86_64;
    
    #[cfg(target_arch = "aarch64")]
    return Architecture::Aarch64;
    
    #[cfg(target_arch = "riscv64")]
    return Architecture::Riscv64;
    
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
    return Architecture::Any;
}

pub fn create_default_repositories() -> Vec<RepositoryConfig> {
    vec![
        RepositoryConfig {
            name: String::from("main"),
            url: String::from("https://packages.rustos.org/stable"),
            mirrors: vec![
                String::from("https://mirror1.rustos.org/stable"),
                String::from("https://mirror2.rustos.org/stable"),
            ],
            priority: 10,
            enabled: true,
            gpg_check: true,
            gpg_key: Some(String::from("https://packages.rustos.org/RPM-GPG-KEY")),
            architecture: vec![get_system_architecture()],
            components: vec![String::from("base"), String::from("extra")],
        },
        RepositoryConfig {
            name: String::from("community"),
            url: String::from("https://packages.rustos.org/community"),
            mirrors: vec![],
            priority: 20,
            enabled: true,
            gpg_check: true,
            gpg_key: Some(String::from("https://packages.rustos.org/RPM-GPG-KEY-community")),
            architecture: vec![get_system_architecture()],
            components: vec![String::from("community")],
        },
        RepositoryConfig {
            name: String::from("testing"),
            url: String::from("https://packages.rustos.org/testing"),
            mirrors: vec![],
            priority: 30,
            enabled: false,
            gpg_check: true,
            gpg_key: Some(String::from("https://packages.rustos.org/RPM-GPG-KEY-testing")),
            architecture: vec![get_system_architecture()],
            components: vec![String::from("testing")],
        },
    ]
}