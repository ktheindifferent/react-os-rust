use std::error::Error;
use crate::config::Config;

pub fn run(
    explicit: bool, deps: bool, orphans: bool, outdated: bool,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("List command not yet implemented");
    Ok(())
}
