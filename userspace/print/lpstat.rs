use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_default_status();
        return;
    }
    
    for arg in &args[1..] {
        match arg.as_str() {
            "-a" => show_accepting(),
            "-c" => show_classes(),
            "-d" => show_default(),
            "-o" => show_jobs(),
            "-p" => show_printers(),
            "-r" => show_scheduler(),
            "-s" => show_status(),
            "-t" => show_all(),
            "-u" => show_user_jobs(),
            "-v" => show_devices(),
            "-h" | "--help" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown option: {}", arg);
                print_usage(&args[0]);
                process::exit(1);
            }
        }
    }
}

fn print_default_status() {
    println!("scheduler is running");
    println!("system default destination: PDF_Printer");
}

fn show_accepting() {
    println!("PDF_Printer accepting requests since Mon Jan 1 00:00:00 2024");
    println!("Network_Printer accepting requests since Mon Jan 1 00:00:00 2024");
}

fn show_classes() {
    println!("No printer classes defined");
}

fn show_default() {
    println!("system default destination: PDF_Printer");
}

fn show_jobs() {
    println!("No active jobs");
}

fn show_printers() {
    println!("printer PDF_Printer is idle. enabled since Mon Jan 1 00:00:00 2024");
    println!("printer Network_Printer is idle. enabled since Mon Jan 1 00:00:00 2024");
}

fn show_scheduler() {
    println!("scheduler is running");
}

fn show_status() {
    println!("system default destination: PDF_Printer");
    println!("scheduler is running");
}

fn show_all() {
    show_scheduler();
    show_default();
    show_printers();
    show_jobs();
}

fn show_user_jobs() {
    println!("No jobs for current user");
}

fn show_devices() {
    println!("device for PDF_Printer: pdf://localhost/pdf");
    println!("device for Network_Printer: ipp://192.168.1.100:631/printers/printer1");
}

fn print_usage(program: &str) {
    println!("Usage: {} [options]", program);
    println!("\nOptions:");
    println!("  -a    Show accepting state");
    println!("  -c    Show printer classes");
    println!("  -d    Show default printer");
    println!("  -o    Show jobs");
    println!("  -p    Show printers");
    println!("  -r    Show scheduler status");
    println!("  -s    Show status summary");
    println!("  -t    Show all information");
    println!("  -u    Show user jobs");
    println!("  -v    Show devices");
    println!("  -h    Show this help");
}