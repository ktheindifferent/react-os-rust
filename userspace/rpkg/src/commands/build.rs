use std::error::Error;
use crate::config::Config;

pub fn run(
    spec_file: &str, output: Option<String>, no_deps: bool, sign: bool,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    println!("Build command not yet implemented");
    Ok(())
}
