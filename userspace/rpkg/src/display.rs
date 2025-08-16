use colored::*;
use prettytable::{Table, row, cell};
use humansize::{format_size, BINARY};

pub fn format_package_list(packages: &[String]) -> String {
    packages.join(", ")
}

pub fn format_size(bytes: u64) -> String {
    format_size(bytes, BINARY)
}

pub fn print_progress_bar(current: usize, total: usize, message: &str) {
    let percentage = (current as f32 / total as f32 * 100.0) as u32;
    let filled = (percentage / 2) as usize;
    let empty = 50 - filled;
    
    print!("\r{} [{}{}] {}% {}", 
        "::".blue().bold(),
        "=".repeat(filled).green(),
        " ".repeat(empty),
        percentage,
        message
    );
    
    if current == total {
        println!();
    }
}

pub fn print_table_header(headers: Vec<&str>) {
    let mut table = Table::new();
    let mut row_cells = Vec::new();
    
    for header in headers {
        row_cells.push(cell!(b->header));
    }
    
    table.add_row(prettytable::Row::new(row_cells));
    table.printstd();
}

pub fn format_time_ago(timestamp: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    let diff = now - timestamp;
    
    if diff < 60 {
        format!("{} seconds ago", diff)
    } else if diff < 3600 {
        format!("{} minutes ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}

pub fn print_error(message: &str) {
    eprintln!("{} {}", "Error:".red().bold(), message);
}

pub fn print_warning(message: &str) {
    eprintln!("{} {}", "Warning:".yellow().bold(), message);
}

pub fn print_info(message: &str) {
    println!("{} {}", "Info:".blue().bold(), message);
}

pub fn print_success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}