use colored::*;
use prettytable::{Table, row, cell};
use std::error::Error;
use crate::config::Config;
use crate::utils::PackageManager;

pub fn run(
    query: &str,
    installed: bool,
    repo: Option<String>,
    config: &Config,
) -> Result<(), Box<dyn Error>> {
    let pm = PackageManager::new(config)?;
    
    let results = if installed {
        pm.search_installed(query)?
    } else if let Some(repo_name) = repo {
        pm.search_repository(query, &repo_name)?
    } else {
        pm.search_all(query)?
    };
    
    if results.is_empty() {
        println!("{} No packages found matching '{}'", 
            "::".yellow().bold(), 
            query.yellow()
        );
        return Ok(());
    }
    
    println!("{} Found {} package(s) matching '{}'", 
        "::".green().bold(),
        results.len(),
        query.green()
    );
    println!();
    
    let mut table = Table::new();
    table.add_row(row![
        b->"Repository",
        b->"Name",
        b->"Version",
        b->"Description"
    ]);
    
    for result in results {
        let repo_name = if installed {
            "[installed]".green().to_string()
        } else {
            result.repository.cyan().to_string()
        };
        
        let installed_marker = if result.is_installed {
            " ✓".green().to_string()
        } else {
            String::new()
        };
        
        let name_with_marker = format!("{}{}", result.name, installed_marker);
        
        let description = if result.description.len() > 60 {
            format!("{}...", &result.description[..57])
        } else {
            result.description.clone()
        };
        
        table.add_row(row![
            repo_name,
            name_with_marker,
            result.version,
            description
        ]);
    }
    
    table.printstd();
    
    println!("\n{} Use 'rpkg info <package>' for detailed information", 
        "Tip:".cyan().bold()
    );
    
    Ok(())
}