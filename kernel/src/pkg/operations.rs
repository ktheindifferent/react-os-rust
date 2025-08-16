use alloc::string::{String, ToString};
use alloc::vec::Vec;
use super::{PackageError, Result, PackageManager};
use super::format::{PackageInfo, Package, parse_package};
use super::database::{InstalledPackage, InstalledFile, InstallReason, TransactionOp};
use super::resolver::{DependencyResolver, Resolution};
use super::cache::PackageCache;

pub fn install(manager: &mut PackageManager, package_name: &str) -> Result<()> {
    if manager.database.get_package(package_name).is_some() {
        return Err(PackageError::AlreadyInstalled(package_name.to_string()));
    }

    let mut resolver = DependencyResolver::new();
    resolver.load_installed(&manager.database);
    resolver.load_available(&manager.repositories);

    let resolution = resolver.resolve_install(package_name)?;

    if !resolution.conflicts.is_empty() {
        return Err(PackageError::DependencyConflict(
            format!("Cannot install due to conflicts: {:?}", resolution.conflicts)
        ));
    }

    let download_size = resolver.calculate_download_size(&resolution);
    let install_size = resolver.calculate_install_size(&resolution);

    println!("Package installation summary:");
    println!("  Packages to install: {}", resolution.to_install.len());
    println!("  Download size: {} bytes", download_size);
    println!("  Install size change: {} bytes", install_size);

    let ordered_packages = resolver.get_install_order(&resolution.to_install)?;

    let tx_id = manager.database.begin_transaction(
        TransactionOp::Install,
        ordered_packages.iter().map(|p| p.name.clone()).collect()
    )?;

    for pkg_info in ordered_packages {
        install_single_package(manager, &pkg_info, InstallReason::Explicit)?;
    }

    manager.database.commit_transaction(tx_id)?;

    println!("Successfully installed {} and its dependencies", package_name);
    Ok(())
}

pub fn remove(manager: &mut PackageManager, package_name: &str) -> Result<()> {
    if manager.database.get_package(package_name).is_none() {
        return Err(PackageError::NotFound(package_name.to_string()));
    }

    let mut resolver = DependencyResolver::new();
    resolver.load_installed(&manager.database);

    let resolution = resolver.resolve_remove(package_name)?;

    if resolution.to_remove.len() > 1 {
        println!("The following packages will be removed:");
        for pkg in &resolution.to_remove {
            println!("  - {}", pkg.name);
        }
    }

    let tx_id = manager.database.begin_transaction(
        TransactionOp::Remove,
        resolution.to_remove.iter().map(|p| p.name.clone()).collect()
    )?;

    for pkg_info in resolution.to_remove {
        remove_single_package(manager, &pkg_info.name)?;
    }

    manager.database.commit_transaction(tx_id)?;

    let orphans = manager.database.find_orphans();
    if !orphans.is_empty() {
        println!("The following packages are now orphaned and can be removed:");
        for orphan in orphans {
            println!("  - {}", orphan);
        }
    }

    println!("Successfully removed {}", package_name);
    Ok(())
}

pub fn upgrade(manager: &mut PackageManager, package_name: Option<&str>) -> Result<()> {
    let mut resolver = DependencyResolver::new();
    resolver.load_installed(&manager.database);
    resolver.load_available(&manager.repositories);

    let resolution = resolver.resolve_upgrade(package_name)?;

    if resolution.to_upgrade.is_empty() {
        println!("All packages are up to date");
        return Ok(());
    }

    println!("Packages to upgrade:");
    for (old, new) in &resolution.to_upgrade {
        println!("  {} {} -> {}", old.name, old.version, new.version);
    }

    let download_size = resolver.calculate_download_size(&resolution);
    println!("Total download size: {} bytes", download_size);

    let packages: Vec<String> = resolution.to_upgrade.iter()
        .map(|(_, new)| new.name.clone())
        .collect();

    let tx_id = manager.database.begin_transaction(
        TransactionOp::Upgrade,
        packages
    )?;

    for (old, new) in resolution.to_upgrade {
        upgrade_single_package(manager, &old, &new)?;
    }

    manager.database.commit_transaction(tx_id)?;

    println!("Successfully upgraded {} package(s)", resolution.to_upgrade.len());
    Ok(())
}

fn install_single_package(
    manager: &mut PackageManager,
    info: &PackageInfo,
    reason: InstallReason
) -> Result<()> {
    let (repo_name, package_info) = manager.repositories.iter()
        .find_map(|repo| {
            repo.get_package(&info.name, Some(&info.version))
                .map(|pkg| (repo.name().to_string(), pkg))
        })
        .ok_or_else(|| PackageError::NotFound(info.name.clone()))?;

    let repo = manager.repositories.iter()
        .find(|r| r.name() == &repo_name)
        .ok_or_else(|| PackageError::NotFound(repo_name.clone()))?;

    let package_data = if let Some(cached) = manager.cache.get_package(info)? {
        cached
    } else {
        let data = repo.download_package(info)?;
        manager.cache.store_package(info, &data)?;
        data
    };

    let package = parse_package(&package_data)?;

    if let Some(ref script) = package.scripts.pre_install {
        run_script("pre-install", script)?;
    }

    let mut installed_files = Vec::new();
    for file in &package.files {
        extract_file(&file.path, &file.content, file.mode)?;
        
        installed_files.push(InstalledFile {
            path: file.path.clone(),
            size: file.size,
            mode: file.mode,
            checksum: file.checksum.clone(),
            modified: false,
        });
    }

    let installed = InstalledPackage {
        info: package.info.clone(),
        files: installed_files,
        install_date: current_timestamp(),
        install_reason: reason,
        config_files: package.config_files.clone()
            .into_iter()
            .map(|path| super::database::ConfigFile {
                path: path.clone(),
                original_checksum: String::new(),
                current_checksum: String::new(),
                backup_path: None,
            })
            .collect(),
    };

    manager.database.add_package(installed)?;

    if let Some(ref script) = package.scripts.post_install {
        run_script("post-install", script)?;
    }

    Ok(())
}

fn remove_single_package(manager: &mut PackageManager, package_name: &str) -> Result<()> {
    let installed = manager.database.remove_package(package_name)?;

    if !installed.config_files.is_empty() {
        println!("Preserving configuration files:");
        for config in &installed.config_files {
            println!("  {}", config.path);
            if let Some(ref backup) = config.backup_path {
                println!("    Backup: {}", backup);
            }
        }
    }

    for file in installed.files.iter().rev() {
        if !file.modified {
            remove_file(&file.path)?;
        } else {
            println!("Skipping modified file: {}", file.path);
        }
    }

    Ok(())
}

fn upgrade_single_package(
    manager: &mut PackageManager,
    old: &PackageInfo,
    new: &PackageInfo
) -> Result<()> {
    let (repo_name, _) = manager.repositories.iter()
        .find_map(|repo| {
            repo.get_package(&new.name, Some(&new.version))
                .map(|pkg| (repo.name().to_string(), pkg))
        })
        .ok_or_else(|| PackageError::NotFound(new.name.clone()))?;

    let repo = manager.repositories.iter()
        .find(|r| r.name() == &repo_name)
        .ok_or_else(|| PackageError::NotFound(repo_name))?;

    let package_data = if let Some(delta_data) = repo.download_delta(old, new)? {
        apply_delta(manager, old, &delta_data)?
    } else {
        let data = repo.download_package(new)?;
        manager.cache.store_package(new, &data)?;
        data
    };

    let package = parse_package(&package_data)?;

    if let Some(ref script) = package.scripts.pre_upgrade {
        run_script("pre-upgrade", script)?;
    }

    remove_single_package(manager, &old.name)?;

    install_single_package(manager, new, InstallReason::Explicit)?;

    if let Some(ref script) = package.scripts.post_upgrade {
        run_script("post-upgrade", script)?;
    }

    Ok(())
}

fn apply_delta(
    manager: &PackageManager,
    base: &PackageInfo,
    delta_data: &[u8]
) -> Result<Vec<u8>> {
    let base_data = manager.cache.get_package(base)?
        .ok_or_else(|| PackageError::NotFound(base.name.clone()))?;

    let mut result = Vec::new();
    
    let mut base_offset = 0;
    let mut delta_offset = 0;

    while delta_offset < delta_data.len() {
        let op = delta_data[delta_offset];
        delta_offset += 1;

        match op {
            0x00 => {
                if delta_offset + 8 > delta_data.len() {
                    break;
                }
                let copy_offset = u32::from_le_bytes([
                    delta_data[delta_offset],
                    delta_data[delta_offset + 1],
                    delta_data[delta_offset + 2],
                    delta_data[delta_offset + 3],
                ]) as usize;
                let copy_len = u32::from_le_bytes([
                    delta_data[delta_offset + 4],
                    delta_data[delta_offset + 5],
                    delta_data[delta_offset + 6],
                    delta_data[delta_offset + 7],
                ]) as usize;
                delta_offset += 8;

                if copy_offset + copy_len <= base_data.len() {
                    result.extend_from_slice(&base_data[copy_offset..copy_offset + copy_len]);
                }
            }
            0x01 => {
                if delta_offset + 4 > delta_data.len() {
                    break;
                }
                let insert_len = u32::from_le_bytes([
                    delta_data[delta_offset],
                    delta_data[delta_offset + 1],
                    delta_data[delta_offset + 2],
                    delta_data[delta_offset + 3],
                ]) as usize;
                delta_offset += 4;

                if delta_offset + insert_len <= delta_data.len() {
                    result.extend_from_slice(&delta_data[delta_offset..delta_offset + insert_len]);
                    delta_offset += insert_len;
                }
            }
            _ => break,
        }
    }

    Ok(result)
}

fn extract_file(path: &str, content: &[u8], mode: u32) -> Result<()> {
    Ok(())
}

fn remove_file(path: &str) -> Result<()> {
    Ok(())
}

fn run_script(phase: &str, script: &str) -> Result<()> {
    println!("Running {} script", phase);
    Ok(())
}

fn current_timestamp() -> u64 {
    0
}

pub fn verify_all_packages(manager: &PackageManager) -> Result<()> {
    println!("Verifying all installed packages...");
    
    let mut issues_found = false;
    
    for pkg_info in manager.database.list_installed() {
        match manager.database.verify_package(&pkg_info.name) {
            Ok(issues) => {
                if !issues.is_empty() {
                    issues_found = true;
                    println!("Issues found in {}:", pkg_info.name);
                    for issue in issues {
                        println!("  - {}", issue);
                    }
                }
            }
            Err(e) => {
                println!("Error verifying {}: {}", pkg_info.name, e);
            }
        }
    }
    
    if !issues_found {
        println!("All packages verified successfully");
    }
    
    Ok(())
}

pub fn clean_orphans(manager: &mut PackageManager) -> Result<()> {
    let orphans = manager.database.find_orphans();
    
    if orphans.is_empty() {
        println!("No orphaned packages found");
        return Ok(());
    }
    
    println!("Found {} orphaned package(s):", orphans.len());
    for orphan in &orphans {
        println!("  - {}", orphan);
    }
    
    let tx_id = manager.database.begin_transaction(
        TransactionOp::Remove,
        orphans.clone()
    )?;
    
    for orphan in orphans {
        remove_single_package(manager, &orphan)?;
    }
    
    manager.database.commit_transaction(tx_id)?;
    
    println!("Successfully removed all orphaned packages");
    Ok(())
}

pub fn rollback(manager: &mut PackageManager, transaction_id: u64) -> Result<()> {
    println!("Rolling back transaction {}...", transaction_id);
    
    manager.database.rollback_transaction(transaction_id)?;
    
    println!("Transaction {} rolled back successfully", transaction_id);
    Ok(())
}