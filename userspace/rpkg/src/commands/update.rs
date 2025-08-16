use std::error::Error;
use crate::config::Config;

pub fn run(
    force: bool,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Update command not yet implemented");
    Ok(())
}
