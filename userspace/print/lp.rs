use std::{env, fs, io::{self, Read}, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }
    
    let mut printer: Option<String> = None;
    let mut copies = 1;
    let mut files: Vec<String> = Vec::new();
    let mut title: Option<String> = None;
    let mut options: Vec<String> = Vec::new();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                i += 1;
                if i < args.len() {
                    printer = Some(args[i].clone());
                }
            }
            "-n" => {
                i += 1;
                if i < args.len() {
                    copies = args[i].parse().unwrap_or(1);
                }
            }
            "-t" => {
                i += 1;
                if i < args.len() {
                    title = Some(args[i].clone());
                }
            }
            "-o" => {
                i += 1;
                if i < args.len() {
                    options.push(args[i].clone());
                }
            }
            "-h" | "--help" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            _ => {
                if !args[i].starts_with('-') {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }
    
    if files.is_empty() {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer).expect("Failed to read from stdin");
        print_data(&buffer, printer.as_deref(), copies, title.as_deref(), &options);
    } else {
        for file in files {
            match fs::read_to_string(&file) {
                Ok(content) => {
                    let file_title = title.as_deref().unwrap_or(&file);
                    print_data(&content, printer.as_deref(), copies, Some(file_title), &options);
                }
                Err(e) => {
                    eprintln!("Error reading file {}: {}", file, e);
                    process::exit(1);
                }
            }
        }
    }
}

fn print_data(data: &str, printer: Option<&str>, copies: u32, title: Option<&str>, options: &[String]) {
    println!("Submitting print job...");
    
    if let Some(p) = printer {
        println!("Printer: {}", p);
    } else {
        println!("Printer: default");
    }
    
    if let Some(t) = title {
        println!("Title: {}", t);
    }
    
    println!("Copies: {}", copies);
    
    if !options.is_empty() {
        println!("Options: {}", options.join(" "));
    }
    
    println!("Job submitted successfully (job-id: 001)");
}

fn print_usage(program: &str) {
    println!("Usage: {} [options] [file...]", program);
    println!("\nOptions:");
    println!("  -d printer    Specify printer");
    println!("  -n copies     Number of copies");
    println!("  -t title      Job title");
    println!("  -o option     Print options");
    println!("  -h, --help    Show this help");
    println!("\nIf no files are specified, reads from stdin");
}