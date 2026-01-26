use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub defaults: SiteDefaults,
    #[serde(default)]
    pub registry: RegistryConfig,
    #[serde(default)]
    pub custom: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteDefaults {
    #[serde(default = "default_author")]
    pub author: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_posts_per_page")]
    pub posts_per_page: usize,
    #[serde(default = "default_excerpt_length")]
    pub excerpt_length: usize,
    #[serde(default = "default_dev_port")]
    pub dev_port: u16,
    #[serde(default = "default_prod_port")]
    pub prod_port: u16,
}

impl Default for SiteDefaults {
    fn default() -> Self {
        Self {
            author: default_author(),
            language: default_language(),
            theme: default_theme(),
            posts_per_page: default_posts_per_page(),
            excerpt_length: default_excerpt_length(),
            dev_port: default_dev_port(),
            prod_port: default_prod_port(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    #[serde(default = "default_auto_port_range_start")]
    pub auto_port_range_start: u16,
    #[serde(default = "default_auto_port_range_end")]
    pub auto_port_range_end: u16,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            auto_port_range_start: default_auto_port_range_start(),
            auto_port_range_end: default_auto_port_range_end(),
        }
    }
}

fn default_author() -> String {
    whoami::username()
}

fn default_language() -> String {
    "en".to_string()
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_posts_per_page() -> usize {
    10
}

fn default_excerpt_length() -> usize {
    200
}

fn default_dev_port() -> u16 {
    3000
}

fn default_prod_port() -> u16 {
    8080
}

fn default_auto_port_range_start() -> u16 {
    3001
}

fn default_auto_port_range_end() -> u16 {
    3100
}

impl GlobalConfig {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config from {}", path.display()))?;
        let config: GlobalConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config from {}", path.display()))?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write config to {}", path.display()))?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["defaults", "author"] => Some(self.defaults.author.clone()),
            ["defaults", "language"] => Some(self.defaults.language.clone()),
            ["defaults", "theme"] => Some(self.defaults.theme.clone()),
            ["defaults", "posts_per_page"] => Some(self.defaults.posts_per_page.to_string()),
            ["defaults", "excerpt_length"] => Some(self.defaults.excerpt_length.to_string()),
            ["defaults", "dev_port"] => Some(self.defaults.dev_port.to_string()),
            ["defaults", "prod_port"] => Some(self.defaults.prod_port.to_string()),
            ["registry", "auto_port_range_start"] => {
                Some(self.registry.auto_port_range_start.to_string())
            }
            ["registry", "auto_port_range_end"] => {
                Some(self.registry.auto_port_range_end.to_string())
            }
            ["custom", k] => self.custom.get(*k).cloned(),
            _ => None,
        }
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["defaults", "author"] => self.defaults.author = value.to_string(),
            ["defaults", "language"] => self.defaults.language = value.to_string(),
            ["defaults", "theme"] => self.defaults.theme = value.to_string(),
            ["defaults", "posts_per_page"] => {
                self.defaults.posts_per_page = value.parse().context("Invalid number")?
            }
            ["defaults", "excerpt_length"] => {
                self.defaults.excerpt_length = value.parse().context("Invalid number")?
            }
            ["defaults", "dev_port"] => {
                self.defaults.dev_port = value.parse().context("Invalid port")?
            }
            ["defaults", "prod_port"] => {
                self.defaults.prod_port = value.parse().context("Invalid port")?
            }
            ["registry", "auto_port_range_start"] => {
                self.registry.auto_port_range_start = value.parse().context("Invalid port")?
            }
            ["registry", "auto_port_range_end"] => {
                self.registry.auto_port_range_end = value.parse().context("Invalid port")?
            }
            ["custom", k] => {
                self.custom.insert(k.to_string(), value.to_string());
            }
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
        Ok(())
    }

    pub fn list(&self) -> Vec<(String, String)> {
        let mut items = vec![
            ("defaults.author".to_string(), self.defaults.author.clone()),
            (
                "defaults.language".to_string(),
                self.defaults.language.clone(),
            ),
            ("defaults.theme".to_string(), self.defaults.theme.clone()),
            (
                "defaults.posts_per_page".to_string(),
                self.defaults.posts_per_page.to_string(),
            ),
            (
                "defaults.excerpt_length".to_string(),
                self.defaults.excerpt_length.to_string(),
            ),
            (
                "defaults.dev_port".to_string(),
                self.defaults.dev_port.to_string(),
            ),
            (
                "defaults.prod_port".to_string(),
                self.defaults.prod_port.to_string(),
            ),
            (
                "registry.auto_port_range_start".to_string(),
                self.registry.auto_port_range_start.to_string(),
            ),
            (
                "registry.auto_port_range_end".to_string(),
                self.registry.auto_port_range_end.to_string(),
            ),
        ];

        for (k, v) in &self.custom {
            items.push((format!("custom.{}", k), v.clone()));
        }

        items
    }

    pub fn remove(&mut self, key: &str) -> Result<bool> {
        let parts: Vec<&str> = key.split('.').collect();
        match parts.as_slice() {
            ["custom", k] => Ok(self.custom.remove(*k).is_some()),
            _ => anyhow::bail!("Can only remove custom.* keys"),
        }
    }
}
