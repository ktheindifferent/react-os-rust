use std::error::Error;
use crate::config::Config;

pub fn run(
    paths: Vec<String>,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Owns command not yet implemented");
    Ok(())
}
