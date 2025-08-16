use std::error::Error;
use crate::config::Config;

pub fn list(config: &Config) -> Result<(), Box<dyn Error>> {
    println!("Repo list command not yet implemented");
    Ok(())
}

pub fn add(name: &str, url: &str, priority: Option<u32>, config: &Config) -> Result<(), Box<dyn Error>> {
    println!("Repo add command not yet implemented");
    Ok(())
}

pub fn remove(name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    println!("Repo remove command not yet implemented");
    Ok(())
}

pub fn enable(name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    println!("Repo enable command not yet implemented");
    Ok(())
}

pub fn disable(name: &str, config: &Config) -> Result<(), Box<dyn Error>> {
    println!("Repo disable command not yet implemented");
    Ok(())
}
