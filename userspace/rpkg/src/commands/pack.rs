use std::error::Error;
use crate::config::Config;

pub fn run(
    directory: &str, spec_file: &str, output: Option<String>,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Pack command not yet implemented");
    Ok(())
}
