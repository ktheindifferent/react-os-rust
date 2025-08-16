use std::error::Error;
use crate::config::Config;

pub fn run(
    packages: Vec<String>, dest: Option<String>,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Download command not yet implemented");
    Ok(())
}
