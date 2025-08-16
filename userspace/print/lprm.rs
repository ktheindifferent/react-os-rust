use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }
    
    let mut printer: Option<String> = None;
    let mut jobs: Vec<String> = Vec::new();
    let mut cancel_all = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-P" => {
                i += 1;
                if i < args.len() {
                    printer = Some(args[i].clone());
                }
            }
            "-" => {
                cancel_all = true;
            }
            "-h" | "--help" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            _ => {
                if !args[i].starts_with('-') {
                    jobs.push(args[i].clone());
                }
            }
        }
        i += 1;
    }
    
    let printer_name = printer.as_deref().unwrap_or("default");
    
    if cancel_all {
        println!("Cancelling all jobs on printer {}...", printer_name);
        println!("All jobs cancelled");
    } else if !jobs.is_empty() {
        for job in jobs {
            println!("Cancelling job {} on printer {}...", job, printer_name);
            println!("Job {} cancelled", job);
        }
    } else {
        println!("No jobs specified");
        print_usage(&args[0]);
        process::exit(1);
    }
}

fn print_usage(program: &str) {
    println!("Usage: {} [options] [job-id...]", program);
    println!("\nOptions:");
    println!("  -P printer    Specify printer");
    println!("  -             Cancel all jobs");
    println!("  -h, --help    Show this help");
    println!("\nExamples:");
    println!("  {} 123", program);
    println!("  {} -P printer1 456 789", program);
    println!("  {} -", program);
}