use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }
    
    let mut printer: Option<String> = None;
    let mut action: Option<Action> = None;
    let mut device: Option<String> = None;
    let mut ppd: Option<String> = None;
    let mut location: Option<String> = None;
    let mut description: Option<String> = None;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-p" => {
                i += 1;
                if i < args.len() {
                    printer = Some(args[i].clone());
                }
            }
            "-x" => {
                action = Some(Action::Delete);
            }
            "-E" => {
                action = Some(Action::Enable);
            }
            "-v" => {
                i += 1;
                if i < args.len() {
                    device = Some(args[i].clone());
                }
            }
            "-P" => {
                i += 1;
                if i < args.len() {
                    ppd = Some(args[i].clone());
                }
            }
            "-L" => {
                i += 1;
                if i < args.len() {
                    location = Some(args[i].clone());
                }
            }
            "-D" => {
                i += 1;
                if i < args.len() {
                    description = Some(args[i].clone());
                }
            }
            "-d" => {
                action = Some(Action::SetDefault);
            }
            "-h" | "--help" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    
    if let Some(p) = printer {
        match action {
            Some(Action::Delete) => {
                println!("Deleting printer {}...", p);
                println!("Printer {} deleted successfully", p);
            }
            Some(Action::Enable) => {
                println!("Enabling printer {}...", p);
                println!("Printer {} enabled successfully", p);
            }
            Some(Action::SetDefault) => {
                println!("Setting {} as default printer...", p);
                println!("Default printer set to {}", p);
            }
            None => {
                if device.is_some() {
                    println!("Adding/modifying printer {}...", p);
                    
                    if let Some(d) = device {
                        println!("  Device: {}", d);
                    }
                    if let Some(ppd_file) = ppd {
                        println!("  PPD: {}", ppd_file);
                    }
                    if let Some(loc) = location {
                        println!("  Location: {}", loc);
                    }
                    if let Some(desc) = description {
                        println!("  Description: {}", desc);
                    }
                    
                    println!("Printer {} added/modified successfully", p);
                } else {
                    println!("Error: Device URI required when adding printer");
                    process::exit(1);
                }
            }
        }
    } else {
        println!("Error: Printer name required");
        process::exit(1);
    }
}

enum Action {
    Delete,
    Enable,
    SetDefault,
}

fn print_usage(program: &str) {
    println!("Usage: {} [options]", program);
    println!("\nOptions:");
    println!("  -p printer    Printer name");
    println!("  -v device     Device URI");
    println!("  -P ppd        PPD file");
    println!("  -L location   Printer location");
    println!("  -D desc       Printer description");
    println!("  -E            Enable printer");
    println!("  -x            Delete printer");
    println!("  -d            Set as default");
    println!("  -h, --help    Show this help");
    println!("\nExamples:");
    println!("  {} -p printer1 -v ipp://192.168.1.100/ipp/print -E", program);
    println!("  {} -p printer1 -d", program);
    println!("  {} -x -p printer1", program);
}