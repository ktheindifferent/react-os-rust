use std::error::Error;
use crate::config::Config;

pub fn run(
    package: &str, reverse: bool, max_depth: Option<usize>,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Deptree command not yet implemented");
    Ok(())
}
