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
    #[serde(default)]
    pub audit: AuditConfig,
    #[serde(default)]
    pub homepage: HomepageConfig,
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
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContentConfig {
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    #[serde(default = "default_excerpt_length")]
    pub excerpt_length: usize,
    #[serde(default = "default_true")]
    pub auto_excerpt: bool,
    /// Number of versions to keep per content item (0 = unlimited)
    #[serde(default = "default_version_retention")]
    pub version_retention: usize,
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
    #[serde(default)]
    pub custom: CustomThemeOptions,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CustomThemeOptions {
    pub primary_color: Option<String>,
    pub primary_color_hover: Option<String>,
    pub accent_color: Option<String>,
    pub background_color: Option<String>,
    pub background_secondary: Option<String>,
    pub text_color: Option<String>,
    pub text_muted: Option<String>,
    pub border_color: Option<String>,
    pub link_color: Option<String>,
    pub font_family: Option<String>,
    pub heading_font_family: Option<String>,
    pub font_size: Option<String>,
    pub heading_scale: Option<f32>,
    pub line_height: Option<f32>,
    pub border_radius: Option<String>,
}

impl CustomThemeOptions {
    pub fn to_css_variables(&self) -> String {
        let mut vars = Vec::new();

        if let Some(ref v) = self.primary_color {
            vars.push(format!("--color-primary: {};", v));
            vars.push(format!("--color-primary-light: {}1a;", v));
        }
        if let Some(ref v) = self.primary_color_hover {
            vars.push(format!("--color-primary-hover: {};", v));
        }
        if let Some(ref v) = self.accent_color {
            vars.push(format!("--color-accent: {};", v));
        }
        if let Some(ref v) = self.background_color {
            vars.push(format!("--bg: {};", v));
        }
        if let Some(ref v) = self.background_secondary {
            vars.push(format!("--bg-secondary: {};", v));
        }
        if let Some(ref v) = self.text_color {
            vars.push(format!("--text: {};", v));
        }
        if let Some(ref v) = self.text_muted {
            vars.push(format!("--text-muted: {};", v));
        }
        if let Some(ref v) = self.border_color {
            vars.push(format!("--border: {};", v));
        }
        if let Some(ref v) = self.link_color {
            vars.push(format!("--color-link: {};", v));
        }
        if let Some(ref v) = self.font_family {
            vars.push(format!("--font-sans: {};", v));
        }
        if let Some(ref v) = self.heading_font_family {
            vars.push(format!("--font-display: {};", v));
        }
        if let Some(ref v) = self.font_size {
            vars.push(format!("--font-size-base: {};", v));
        }
        if let Some(v) = self.line_height {
            vars.push(format!("--line-height-normal: {};", v));
        }
        if let Some(ref v) = self.border_radius {
            vars.push(format!("--radius: {};", v));
            vars.push(format!("--radius-sm: {};", v));
            vars.push(format!("--radius-lg: {};", v));
        }

        vars.join("\n    ")
    }

    pub fn has_customizations(&self) -> bool {
        self.primary_color.is_some()
            || self.primary_color_hover.is_some()
            || self.accent_color.is_some()
            || self.background_color.is_some()
            || self.background_secondary.is_some()
            || self.text_color.is_some()
            || self.text_muted.is_some()
            || self.border_color.is_some()
            || self.link_color.is_some()
            || self.font_family.is_some()
            || self.heading_font_family.is_some()
            || self.font_size.is_some()
            || self.heading_scale.is_some()
            || self.line_height.is_some()
            || self.border_radius.is_some()
    }
}

impl ThemeConfig {
    pub const AVAILABLE_THEMES: [&'static str; 5] =
        ["default", "minimal", "magazine", "brutalist", "neon"];

    pub fn validate(&self) -> Result<()> {
        if !Self::AVAILABLE_THEMES.contains(&self.name.as_str()) {
            anyhow::bail!(
                "Invalid theme '{}'. Available themes: {}",
                self.name,
                Self::AVAILABLE_THEMES.join(", ")
            );
        }
        Ok(())
    }

    pub fn is_valid_theme(name: &str) -> bool {
        Self::AVAILABLE_THEMES.contains(&name)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthConfig {
    #[serde(default = "default_session_lifetime")]
    pub session_lifetime: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuditConfig {
    #[serde(default = "default_audit_enabled")]
    pub enabled: bool,
    #[serde(default = "default_audit_retention_days")]
    pub retention_days: u32,
    #[serde(default = "default_audit_log_auth")]
    pub log_auth_events: bool,
    #[serde(default)]
    pub log_content_views: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: default_audit_enabled(),
            retention_days: default_audit_retention_days(),
            log_auth_events: default_audit_log_auth(),
            log_content_views: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HomepageConfig {
    #[serde(default = "default_hero_layout")]
    pub hero_layout: String,
    #[serde(default)]
    pub hero_image: Option<String>,
    #[serde(default = "default_hero_height")]
    pub hero_height: String,
    #[serde(default = "default_hero_text_align")]
    pub hero_text_align: String,
    #[serde(default = "default_true")]
    pub show_hero: bool,
    #[serde(default = "default_true")]
    pub show_pages: bool,
    #[serde(default = "default_true")]
    pub show_posts: bool,
    #[serde(default = "default_posts_layout")]
    pub posts_layout: String,
    #[serde(default = "default_posts_columns")]
    pub posts_columns: u8,
    #[serde(default = "default_pages_layout")]
    pub pages_layout: String,
    #[serde(default)]
    pub sections_order: Vec<String>,
}

impl Default for HomepageConfig {
    fn default() -> Self {
        Self {
            hero_layout: default_hero_layout(),
            hero_image: None,
            hero_height: default_hero_height(),
            hero_text_align: default_hero_text_align(),
            show_hero: true,
            show_pages: true,
            show_posts: true,
            posts_layout: default_posts_layout(),
            posts_columns: default_posts_columns(),
            pages_layout: default_pages_layout(),
            sections_order: Vec::new(),
        }
    }
}

impl HomepageConfig {
    pub fn get_sections_order(&self) -> Vec<&str> {
        if self.sections_order.is_empty() {
            vec!["hero", "pages", "posts"]
        } else {
            self.sections_order.iter().map(|s| s.as_str()).collect()
        }
    }
}

fn default_hero_layout() -> String {
    "centered".to_string()
}

fn default_hero_height() -> String {
    "medium".to_string()
}

fn default_hero_text_align() -> String {
    "center".to_string()
}

fn default_posts_layout() -> String {
    "grid".to_string()
}

fn default_posts_columns() -> u8 {
    2
}

fn default_pages_layout() -> String {
    "grid".to_string()
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

fn default_pool_size() -> u32 {
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

fn default_version_retention() -> usize {
    50
}

fn default_audit_enabled() -> bool {
    true
}

fn default_audit_retention_days() -> u32 {
    90
}

fn default_audit_log_auth() -> bool {
    true
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
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<()> {
        if self.content.posts_per_page == 0 {
            anyhow::bail!("content.posts_per_page must be greater than 0");
        }
        if self.content.posts_per_page > 100 {
            anyhow::bail!("content.posts_per_page must be 100 or less");
        }
        if self.content.excerpt_length == 0 {
            anyhow::bail!("content.excerpt_length must be greater than 0");
        }
        if self.content.excerpt_length > 10000 {
            anyhow::bail!("content.excerpt_length must be 10000 or less");
        }
        self.theme.validate()?;
        Ok(())
    }
}
