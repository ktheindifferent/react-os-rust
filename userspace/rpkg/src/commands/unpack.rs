use std::error::Error;
use crate::config::Config;

pub fn run(
    package_file: &str, dest: Option<String>,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Unpack command not yet implemented");
    Ok(())
}
