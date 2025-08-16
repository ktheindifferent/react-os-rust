use std::error::Error;
use crate::config::Config;

pub fn run(
    packages: Vec<String>, all: bool, quiet: bool,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Verify command not yet implemented");
    Ok(())
}
