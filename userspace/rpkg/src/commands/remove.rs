use std::error::Error;
use crate::config::Config;

pub fn run(
    packages: Vec<String>,
    cascade: bool,
    keep_deps: bool,
    purge: bool,
    config: &Config,
    yes: bool,
) -> Result<(), Box<dyn Error>> {
    println!("Remove command not yet implemented");
    Ok(())
}
