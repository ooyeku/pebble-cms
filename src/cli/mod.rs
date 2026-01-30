pub mod backup;
pub mod build;
pub mod config;
pub mod deploy;
pub mod export;
pub mod import;
pub mod init;
pub mod migrate;
pub mod registry;
pub mod serve;
pub mod user;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pebble")]
#[command(version)]
#[command(about = "A lightweight personal CMS", long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value = "pebble.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    Init {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    Serve {
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    Deploy {
        #[arg(short = 'H', long, default_value = "0.0.0.0")]
        host: String,
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    Build {
        #[arg(short, long, default_value = "./dist")]
        output: PathBuf,
        #[arg(long)]
        base_url: Option<String>,
    },
    Export {
        #[arg(short, long, default_value = "./export")]
        output: PathBuf,
        #[arg(long)]
        include_drafts: bool,
        #[arg(long)]
        include_media: bool,
    },
    Import {
        #[arg(default_value = "./export")]
        path: PathBuf,
        #[arg(long)]
        overwrite: bool,
    },
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    Migrate,
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Registry {
        #[command(subcommand)]
        command: RegistryCommand,
    },
}

#[derive(Subcommand)]
pub enum UserCommand {
    Add {
        #[arg(long)]
        username: String,
        #[arg(long)]
        email: String,
        #[arg(long, default_value = "author")]
        role: String,
        #[arg(long)]
        password: Option<String>,
    },
    List,
    Remove {
        username: String,
    },
    Passwd {
        username: String,
    },
}

#[derive(Subcommand)]
pub enum BackupCommand {
    Create {
        #[arg(short, long, default_value = "./backups")]
        output: PathBuf,
    },
    Restore {
        file: PathBuf,
    },
    List {
        #[arg(short, long, default_value = "./backups")]
        dir: PathBuf,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommand {
    Get { key: String },
    Set { key: String, value: String },
    List,
    Remove { key: String },
    Path,
}

#[derive(Subcommand)]
pub enum RegistryCommand {
    Init {
        name: String,
        #[arg(long)]
        title: Option<String>,
    },
    List,
    Serve {
        name: String,
        #[arg(short, long)]
        port: Option<u16>,
    },
    Deploy {
        name: String,
        #[arg(short, long)]
        port: Option<u16>,
    },
    Stop {
        name: String,
    },
    StopAll,
    Remove {
        name: String,
        #[arg(long)]
        force: bool,
    },
    Status {
        name: String,
    },
    Path {
        name: Option<String>,
    },
    /// View or edit a site's configuration
    Config {
        /// Site name
        name: String,
        #[command(subcommand)]
        command: Option<SiteConfigCommand>,
    },
}

#[derive(Subcommand)]
pub enum SiteConfigCommand {
    /// Get a config value (e.g., theme.name, site.title)
    Get {
        /// Config key in dot notation (e.g., theme.name)
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key in dot notation (e.g., theme.name)
        key: String,
        /// New value
        value: String,
    },
    /// Open config file in editor
    Edit,
}
