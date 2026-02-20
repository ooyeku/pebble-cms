pub mod backup;
pub mod build;
pub mod config;
pub mod deploy;
pub mod doctor;
pub mod export;
pub mod import;
pub mod import_ghost;
pub mod import_wordpress;
pub mod init;
pub mod migrate;
pub mod registry;
pub mod rerender;
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
    /// Initialize a new Pebble site
    Init {
        /// Directory to create the site in
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Name of the site
        #[arg(long)]
        name: Option<String>,
    },
    /// Start the development server
    Serve {
        /// Host address to bind to
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,
        /// Port to listen on
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// Start the production server
    Deploy {
        /// Host address to bind to
        #[arg(short = 'H', long, default_value = "0.0.0.0")]
        host: String,
        /// Port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Generate a static site
    Build {
        /// Output directory for the static files
        #[arg(short, long, default_value = "./dist")]
        output: PathBuf,
        /// Base URL for the generated site
        #[arg(long)]
        base_url: Option<String>,
    },
    /// Export site content to portable format
    Export {
        /// Output directory for exported content
        #[arg(short, long, default_value = "./export")]
        output: PathBuf,
        /// Include draft posts and pages
        #[arg(long)]
        include_drafts: bool,
        /// Include media files
        #[arg(long)]
        include_media: bool,
        /// Export format: pebble, hugo, or zola
        #[arg(long, default_value = "pebble")]
        format: String,
    },
    /// Import content from an export directory
    Import {
        /// Path to the export directory
        #[arg(default_value = "./export")]
        path: PathBuf,
        /// Overwrite existing content with the same slug
        #[arg(long)]
        overwrite: bool,
    },
    /// Import from a WordPress WXR export file
    ImportWp {
        /// Path to the WordPress WXR XML file
        file: PathBuf,
        /// Overwrite existing content with the same slug
        #[arg(long)]
        overwrite: bool,
    },
    /// Import from a Ghost JSON export file
    ImportGhost {
        /// Path to the Ghost JSON export file
        file: PathBuf,
        /// Overwrite existing content with the same slug
        #[arg(long)]
        overwrite: bool,
    },
    /// Backup and restore site data
    Backup {
        #[command(subcommand)]
        command: BackupCommand,
    },
    /// Run database migrations
    Migrate {
        #[command(subcommand)]
        command: Option<MigrateCommand>,
    },
    /// Check system health and configuration
    Doctor,
    /// Re-render all content HTML from markdown
    Rerender,
    /// Manage users
    User {
        #[command(subcommand)]
        command: UserCommand,
    },
    /// Manage global configuration
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    /// Manage multiple Pebble sites
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
pub enum MigrateCommand {
    /// Show applied and pending migrations
    Status,
    /// Roll back the most recent migration(s)
    Rollback {
        /// Number of migrations to roll back
        #[arg(short, long, default_value = "1")]
        steps: u32,
        /// Force rollback without confirmation (required for migration 001)
        #[arg(long)]
        force: bool,
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
    /// Create a new site in the registry
    Init {
        /// Unique name for the site
        name: String,
        /// Display title for the site
        #[arg(long)]
        title: Option<String>,
    },
    /// List all registered sites
    List,
    /// Start a site's development server
    Serve {
        /// Name of the site to serve
        name: String,
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Start a site's production server
    Deploy {
        /// Name of the site to deploy
        name: String,
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Stop a running site
    Stop {
        /// Name of the site to stop
        name: String,
    },
    /// Stop all running sites
    StopAll,
    /// Remove a site from the registry
    Remove {
        /// Name of the site to remove
        name: String,
        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
    /// Show the status of a site
    Status {
        /// Name of the site
        name: String,
    },
    /// Show the path to a site or the registry directory
    Path {
        /// Name of the site (omit to show registry directory)
        name: Option<String>,
    },
    /// Re-render all content HTML from markdown for a site
    Rerender {
        /// Name of the site to re-render
        name: String,
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
