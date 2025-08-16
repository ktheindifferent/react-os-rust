use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub repositories: Vec<RepositoryConfig>,
    pub cache: CacheConfig,
    pub network: NetworkConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub root_dir: String,
    pub db_path: String,
    pub log_level: String,
    pub architecture: String,
    pub parallel_downloads: usize,
    pub color_output: bool,
    pub confirm_actions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    pub priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub dir: String,
    pub max_size: u64,
    pub clean_on_exit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub timeout_seconds: u32,
    pub retry_count: u32,
    pub proxy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub verify_signatures: bool,
    pub verify_checksums: bool,
    pub allow_downgrade: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                root_dir: String::from("/"),
                db_path: String::from("/var/lib/rpkg/db"),
                log_level: String::from("info"),
                architecture: String::from("x86_64"),
                parallel_downloads: 4,
                color_output: true,
                confirm_actions: true,
            },
            repositories: vec![
                RepositoryConfig {
                    name: String::from("main"),
                    url: String::from("https://packages.rustos.org/stable"),
                    enabled: true,
                    priority: 10,
                },
                RepositoryConfig {
                    name: String::from("community"),
                    url: String::from("https://packages.rustos.org/community"),
                    enabled: true,
                    priority: 20,
                },
            ],
            cache: CacheConfig {
                dir: String::from("/var/cache/rpkg"),
                max_size: 1024 * 1024 * 1024,
                clean_on_exit: false,
            },
            network: NetworkConfig {
                timeout_seconds: 30,
                retry_count: 3,
                proxy: None,
            },
            security: SecurityConfig {
                verify_signatures: true,
                verify_checksums: true,
                allow_downgrade: false,
            },
        }
    }
}

pub fn load_config(path: Option<&str>) -> Result<Config, Box<dyn Error>> {
    let config_path = path.unwrap_or("/etc/rpkg/config.toml");
    
    if Path::new(config_path).exists() {
        let contents = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&contents)?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

pub fn save_config(config: &Config, path: Option<&str>) -> Result<(), Box<dyn Error>> {
    let config_path = path.unwrap_or("/etc/rpkg/config.toml");
    let contents = toml::to_string_pretty(config)?;
    
    if let Some(parent) = Path::new(config_path).parent() {
        fs::create_dir_all(parent)?;
    }
    
    fs::write(config_path, contents)?;
    Ok(())
}