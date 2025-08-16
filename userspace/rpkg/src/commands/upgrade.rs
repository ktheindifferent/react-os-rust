use std::error::Error;
use crate::config::Config;

pub fn run(
    packages: Vec<String>, ignore: Vec<String>, download_only: bool,
    config: &Config,
    yes: bool,
) -> Result<(), Box<dyn Error>> {
    println!("Upgrade command not yet implemented");
    Ok(())
}
