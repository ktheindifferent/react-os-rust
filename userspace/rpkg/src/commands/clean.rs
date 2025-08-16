use std::error::Error;
use crate::config::Config;

pub fn run(
    all: bool, keep: Option<usize>,
    config: &Config,
    yes: bool,
) -> Result<(), Box<dyn Error>> {
    println!("Clean command not yet implemented");
    Ok(())
}
