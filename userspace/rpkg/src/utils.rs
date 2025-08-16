use std::error::Error;
use std::io::{self, Write};
use colored::*;
use crate::config::Config;

pub fn confirm_action(prompt: &str) -> Result<bool, Box<dyn Error>> {
    print!("{} {} [Y/n] ", "::".blue().bold(), prompt);
    io::stdout().flush()?;
    
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    
    let response = input.trim().to_lowercase();
    Ok(response.is_empty() || response == "y" || response == "yes")
}

pub struct PackageManager {
    config: Config,
}

impl PackageManager {
    pub fn new(config: &Config) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config: config.clone(),
        })
    }
    
    pub fn resolve_install(&self, packages: &[String], no_deps: bool) -> Result<Resolution, Box<dyn Error>> {
        Ok(Resolution::default())
    }
    
    pub fn search_installed(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        Ok(Vec::new())
    }
    
    pub fn search_repository(&self, query: &str, repo: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        Ok(Vec::new())
    }
    
    pub fn search_all(&self, query: &str) -> Result<Vec<SearchResult>, Box<dyn Error>> {
        Ok(Vec::new())
    }
    
    pub fn download_package(&self, pkg: &PackageInfo) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    
    pub fn install_package(&self, pkg: &PackageInfo) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
    
    pub fn install_as_dependency(&self, pkg: &PackageInfo) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[derive(Default)]
pub struct Resolution {
    pub to_install: Vec<PackageInfo>,
    pub to_upgrade: Vec<(PackageInfo, PackageInfo)>,
    pub to_remove: Vec<PackageInfo>,
    pub conflicts: Vec<Conflict>,
    pub suggestions: Vec<Suggestion>,
}

impl Resolution {
    pub fn total_download_size(&self) -> u64 {
        self.to_install.iter().map(|p| p.size).sum::<u64>() +
        self.to_upgrade.iter().map(|(_, p)| p.size).sum::<u64>()
    }
    
    pub fn total_install_size(&self) -> u64 {
        self.to_install.iter().map(|p| p.installed_size).sum()
    }
}

pub struct PackageInfo {
    pub name: String,
    pub version: Version,
    pub size: u64,
    pub installed_size: u64,
}

pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ToString for Version {
    fn to_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

pub struct Conflict {
    pub package1: String,
    pub package2: String,
    pub reason: String,
}

pub struct Suggestion {
    pub name: String,
    pub reason: String,
}

pub struct SearchResult {
    pub repository: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub is_installed: bool,
}