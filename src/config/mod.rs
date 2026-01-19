use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub site: SiteConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub content: ContentConfig,
    pub media: MediaConfig,
    pub theme: ThemeConfig,
    pub auth: AuthConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SiteConfig {
    pub title: String,
    pub description: String,
    pub url: String,
    #[serde(default = "default_language")]
    pub language: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContentConfig {
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    #[serde(default = "default_excerpt_length")]
    pub excerpt_length: usize,
    #[serde(default = "default_true")]
    pub auto_excerpt: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaConfig {
    pub upload_dir: String,
    #[serde(default = "default_max_upload")]
    pub max_upload_size: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ThemeConfig {
    #[serde(default = "default_theme")]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    #[serde(default = "default_session_lifetime")]
    pub session_lifetime: String,
}

fn default_language() -> String {
    "en".to_string()
}
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    3000
}
fn default_posts_per_page() -> usize {
    10
}
fn default_excerpt_length() -> usize {
    200
}
fn default_true() -> bool {
    true
}
fn default_max_upload() -> String {
    "10MB".to_string()
}
fn default_theme() -> String {
    "default".to_string()
}
fn default_session_lifetime() -> String {
    "7d".to_string()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            anyhow::anyhow!(
                "Could not read config file '{}': {}. Are you in a Pebble site directory?",
                path.display(),
                e
            )
        })?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
