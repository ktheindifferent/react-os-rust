use clap::{Parser, Subcommand};
use colored::*;
use std::process;

mod commands;
mod config;
mod utils;
mod display;

#[derive(Parser)]
#[command(name = "rpkg")]
#[command(author = "RustOS Contributors")]
#[command(version = "1.0.0")]
#[command(about = "Package manager for RustOS", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(short, long, global = true)]
    verbose: bool,

    #[arg(short, long, global = true)]
    quiet: bool,

    #[arg(short = 'y', long, global = true)]
    yes: bool,

    #[arg(long, global = true)]
    no_color: bool,

    #[arg(long, value_name = "FILE", global = true)]
    config: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Install one or more packages")]
    Install {
        #[arg(required = true)]
        packages: Vec<String>,

        #[arg(long)]
        no_deps: bool,

        #[arg(long)]
        as_deps: bool,

        #[arg(long)]
        reinstall: bool,
    },

    #[command(about = "Remove one or more packages")]
    Remove {
        #[arg(required = true)]
        packages: Vec<String>,

        #[arg(long)]
        cascade: bool,

        #[arg(long)]
        keep_deps: bool,

        #[arg(long)]
        purge: bool,
    },

    #[command(about = "Upgrade installed packages")]
    Upgrade {
        packages: Vec<String>,

        #[arg(long)]
        ignore: Vec<String>,

        #[arg(long)]
        download_only: bool,
    },

    #[command(about = "Search for packages")]
    Search {
        #[arg(required = true)]
        query: String,

        #[arg(short, long)]
        installed: bool,

        #[arg(short, long)]
        repo: Option<String>,
    },

    #[command(about = "Show package information")]
    Info {
        #[arg(required = true)]
        package: String,

        #[arg(short, long)]
        files: bool,

        #[arg(short, long)]
        deps: bool,

        #[arg(short, long)]
        reverse_deps: bool,
    },

    #[command(about = "List installed packages")]
    List {
        #[arg(short, long)]
        explicit: bool,

        #[arg(short, long)]
        deps: bool,

        #[arg(short, long)]
        orphans: bool,

        #[arg(short, long)]
        outdated: bool,
    },

    #[command(about = "Update repository databases")]
    Update {
        #[arg(long)]
        force: bool,
    },

    #[command(about = "Clean package cache")]
    Clean {
        #[arg(long)]
        all: bool,

        #[arg(long)]
        keep: Option<usize>,
    },

    #[command(about = "Verify installed packages")]
    Verify {
        packages: Vec<String>,

        #[arg(long)]
        all: bool,

        #[arg(long)]
        quiet: bool,
    },

    #[command(about = "Manage repositories")]
    Repo {
        #[command(subcommand)]
        action: RepoAction,
    },

    #[command(about = "Show package ownership of files")]
    Owns {
        #[arg(required = true)]
        paths: Vec<String>,
    },

    #[command(about = "Show package that provides a capability")]
    Provides {
        #[arg(required = true)]
        capability: String,
    },

    #[command(about = "Download packages without installing")]
    Download {
        #[arg(required = true)]
        packages: Vec<String>,

        #[arg(short, long, value_name = "DIR")]
        dest: Option<String>,
    },

    #[command(about = "Show dependency tree")]
    DepTree {
        #[arg(required = true)]
        package: String,

        #[arg(long)]
        reverse: bool,

        #[arg(long)]
        max_depth: Option<usize>,
    },

    #[command(about = "Show statistics")]
    Stats,

    #[command(about = "Build package from source")]
    Build {
        #[arg(required = true)]
        spec_file: String,

        #[arg(short, long, value_name = "DIR")]
        output: Option<String>,

        #[arg(long)]
        no_deps: bool,

        #[arg(long)]
        sign: bool,
    },

    #[command(about = "Create package from directory")]
    Pack {
        #[arg(required = true)]
        directory: String,

        #[arg(required = true)]
        spec_file: String,

        #[arg(short, long, value_name = "FILE")]
        output: Option<String>,
    },

    #[command(about = "Extract package contents")]
    Unpack {
        #[arg(required = true)]
        package_file: String,

        #[arg(short, long, value_name = "DIR")]
        dest: Option<String>,
    },

    #[command(about = "Show or manage configuration")]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
}

#[derive(Subcommand)]
enum RepoAction {
    #[command(about = "List configured repositories")]
    List,

    #[command(about = "Add a repository")]
    Add {
        name: String,
        url: String,

        #[arg(long)]
        priority: Option<u32>,
    },

    #[command(about = "Remove a repository")]
    Remove {
        name: String,
    },

    #[command(about = "Enable a repository")]
    Enable {
        name: String,
    },

    #[command(about = "Disable a repository")]
    Disable {
        name: String,
    },
}

#[derive(Subcommand)]
enum ConfigAction {
    #[command(about = "Show current configuration")]
    Show,

    #[command(about = "Set a configuration value")]
    Set {
        key: String,
        value: String,
    },

    #[command(about = "Get a configuration value")]
    Get {
        key: String,
    },

    #[command(about = "Reset configuration to defaults")]
    Reset {
        #[arg(long)]
        force: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    if cli.no_color {
        colored::control::set_override(false);
    }

    let config = match config::load_config(cli.config.as_deref()) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("{} Failed to load configuration: {}", "Error:".red().bold(), e);
            process::exit(1);
        }
    };

    let result = match cli.command {
        Commands::Install { packages, no_deps, as_deps, reinstall } => {
            commands::install::run(packages, no_deps, as_deps, reinstall, &config, cli.yes)
        }
        Commands::Remove { packages, cascade, keep_deps, purge } => {
            commands::remove::run(packages, cascade, keep_deps, purge, &config, cli.yes)
        }
        Commands::Upgrade { packages, ignore, download_only } => {
            commands::upgrade::run(packages, ignore, download_only, &config, cli.yes)
        }
        Commands::Search { query, installed, repo } => {
            commands::search::run(&query, installed, repo, &config)
        }
        Commands::Info { package, files, deps, reverse_deps } => {
            commands::info::run(&package, files, deps, reverse_deps, &config)
        }
        Commands::List { explicit, deps, orphans, outdated } => {
            commands::list::run(explicit, deps, orphans, outdated, &config)
        }
        Commands::Update { force } => {
            commands::update::run(force, &config)
        }
        Commands::Clean { all, keep } => {
            commands::clean::run(all, keep, &config, cli.yes)
        }
        Commands::Verify { packages, all, quiet } => {
            commands::verify::run(packages, all, quiet, &config)
        }
        Commands::Repo { action } => {
            match action {
                RepoAction::List => commands::repo::list(&config),
                RepoAction::Add { name, url, priority } => {
                    commands::repo::add(&name, &url, priority, &config)
                }
                RepoAction::Remove { name } => commands::repo::remove(&name, &config),
                RepoAction::Enable { name } => commands::repo::enable(&name, &config),
                RepoAction::Disable { name } => commands::repo::disable(&name, &config),
            }
        }
        Commands::Owns { paths } => {
            commands::owns::run(paths, &config)
        }
        Commands::Provides { capability } => {
            commands::provides::run(&capability, &config)
        }
        Commands::Download { packages, dest } => {
            commands::download::run(packages, dest, &config)
        }
        Commands::DepTree { package, reverse, max_depth } => {
            commands::deptree::run(&package, reverse, max_depth, &config)
        }
        Commands::Stats => {
            commands::stats::run(&config)
        }
        Commands::Build { spec_file, output, no_deps, sign } => {
            commands::build::run(&spec_file, output, no_deps, sign, &config)
        }
        Commands::Pack { directory, spec_file, output } => {
            commands::pack::run(&directory, &spec_file, output, &config)
        }
        Commands::Unpack { package_file, dest } => {
            commands::unpack::run(&package_file, dest, &config)
        }
        Commands::Config { action } => {
            match action {
                ConfigAction::Show => commands::config::show(&config),
                ConfigAction::Set { key, value } => commands::config::set(&key, &value, &config),
                ConfigAction::Get { key } => commands::config::get(&key, &config),
                ConfigAction::Reset { force } => commands::config::reset(force, &config),
            }
        }
    };

    if let Err(e) = result {
        if !cli.quiet {
            eprintln!("{} {}", "Error:".red().bold(), e);
        }
        process::exit(1);
    }
}