use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use humansize::{format_size, BINARY};
use std::error::Error;
use crate::config::Config;
use crate::utils::{confirm_action, PackageManager};
use crate::display::format_package_list;

pub fn run(
    packages: Vec<String>,
    no_deps: bool,
    as_deps: bool,
    reinstall: bool,
    config: &Config,
    yes: bool,
) -> Result<(), Box<dyn Error>> {
    let mut pm = PackageManager::new(config)?;
    
    println!("{} Resolving dependencies...", "::".blue().bold());
    
    let resolution = pm.resolve_install(&packages, no_deps)?;
    
    if resolution.to_install.is_empty() && !reinstall {
        println!("{} All requested packages are already installed", "::".green().bold());
        return Ok(());
    }
    
    if !resolution.conflicts.is_empty() {
        eprintln!("{} Package conflicts detected:", "Error:".red().bold());
        for conflict in &resolution.conflicts {
            eprintln!("  {} {} conflicts with {}", 
                "•".red(), 
                conflict.package1.yellow(), 
                conflict.package2.yellow()
            );
        }
        return Err("Cannot proceed due to conflicts".into());
    }
    
    let download_size = resolution.total_download_size();
    let install_size = resolution.total_install_size();
    
    println!("\n{}", "Package Installation Summary:".bold());
    println!("{}════════════════════════════", "═".dimmed());
    
    if !resolution.to_install.is_empty() {
        println!("\n{} ({}):", "Packages to install".green(), resolution.to_install.len());
        for pkg in &resolution.to_install {
            println!("  {} {}-{}", 
                "•".green(), 
                pkg.name.bold(), 
                pkg.version.to_string().dimmed()
            );
        }
    }
    
    if !resolution.to_upgrade.is_empty() {
        println!("\n{} ({}):", "Packages to upgrade".yellow(), resolution.to_upgrade.len());
        for (old, new) in &resolution.to_upgrade {
            println!("  {} {} {} → {}", 
                "•".yellow(),
                old.name.bold(),
                old.version.to_string().dimmed(),
                new.version.to_string().green()
            );
        }
    }
    
    println!("\n{} {}", 
        "Total download size:".bold(), 
        format_size(download_size, BINARY).cyan()
    );
    println!("{} {}", 
        "Total installed size:".bold(), 
        format_size(install_size, BINARY).cyan()
    );
    
    if !yes && !confirm_action("Proceed with installation?")? {
        println!("{} Installation cancelled", "::".yellow().bold());
        return Ok(());
    }
    
    let pb = ProgressBar::new(resolution.to_install.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    
    println!("\n{} Downloading packages...", "::".blue().bold());
    
    for pkg in &resolution.to_install {
        pb.set_message(format!("Downloading {}", pkg.name));
        pm.download_package(pkg)?;
        pb.inc(1);
    }
    
    pb.finish_with_message("Downloads complete");
    
    println!("\n{} Installing packages...", "::".blue().bold());
    
    let pb = ProgressBar::new(resolution.to_install.len() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-")
    );
    
    for pkg in &resolution.to_install {
        pb.set_message(format!("Installing {}", pkg.name));
        
        if as_deps {
            pm.install_as_dependency(pkg)?;
        } else {
            pm.install_package(pkg)?;
        }
        
        pb.inc(1);
    }
    
    pb.finish_with_message("Installation complete");
    
    println!("\n{} Successfully installed {} package(s)", 
        "✓".green().bold(),
        resolution.to_install.len()
    );
    
    if !resolution.suggestions.is_empty() {
        println!("\n{} Optional dependencies:", "Tip:".cyan().bold());
        for suggestion in &resolution.suggestions {
            println!("  {} {} - {}", 
                "•".dimmed(), 
                suggestion.name.yellow(),
                suggestion.reason.dimmed()
            );
        }
    }
    
    Ok(())
}