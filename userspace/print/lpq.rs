use std::{env, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut printer: Option<String> = None;
    let mut verbose = false;
    let mut all_jobs = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-P" => {
                i += 1;
                if i < args.len() {
                    printer = Some(args[i].clone());
                }
            }
            "-l" => {
                verbose = true;
            }
            "-a" => {
                all_jobs = true;
            }
            "-h" | "--help" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    
    let printer_name = printer.as_deref().unwrap_or("default");
    
    println!("{} is ready", printer_name);
    
    if all_jobs {
        println!("Rank    Owner   Job     File(s)                         Total Size");
        println!("active  user    1       document.pdf                    123456 bytes");
        println!("1st     alice   2       report.txt                      45678 bytes");
        println!("2nd     bob     3       presentation.pdf                234567 bytes");
    } else {
        println!("no entries");
    }
    
    if verbose {
        println!("\nDetailed job information:");
        println!("Job 1: submitted by user at Mon Jan 1 10:00:00 2024");
        println!("  Pages: 10");
        println!("  Priority: normal");
        println!("  Status: printing");
    }
}

fn print_usage(program: &str) {
    println!("Usage: {} [options]", program);
    println!("\nOptions:");
    println!("  -P printer    Specify printer");
    println!("  -l            Long/verbose format");
    println!("  -a            Show all jobs");
    println!("  -h, --help    Show this help");
}