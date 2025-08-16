use std::error::Error;
use crate::config::Config;

pub fn run(
    capability: &str,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Provides command not yet implemented");
    Ok(())
}
