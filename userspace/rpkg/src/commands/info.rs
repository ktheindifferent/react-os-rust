use std::error::Error;
use crate::config::Config;

pub fn run(
    package: &str, files: bool, deps: bool, reverse_deps: bool,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Info command not yet implemented");
    Ok(())
}
