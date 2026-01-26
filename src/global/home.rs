use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PebbleHome {
    pub root: PathBuf,
    pub config_path: PathBuf,
    pub registry_dir: PathBuf,
    pub registry_path: PathBuf,
}

impl PebbleHome {
    pub fn get_home_dir() -> Result<PathBuf> {
        let home = dirs::home_dir().context("Could not determine home directory")?;
        Ok(home.join(".pebble"))
    }

    pub fn init() -> Result<Self> {
        let root = Self::get_home_dir()?;
        let config_path = root.join("config.toml");
        let registry_dir = root.join("registry");
        let registry_path = root.join("registry.toml");

        if !root.exists() {
            std::fs::create_dir_all(&root)
                .with_context(|| format!("Failed to create Pebble home at {}", root.display()))?;
            tracing::info!("Created Pebble home directory: {}", root.display());
        }

        if !registry_dir.exists() {
            std::fs::create_dir_all(&registry_dir).with_context(|| {
                format!(
                    "Failed to create registry directory at {}",
                    registry_dir.display()
                )
            })?;
        }

        Ok(Self {
            root,
            config_path,
            registry_dir,
            registry_path,
        })
    }

    pub fn exists() -> Result<bool> {
        let home = Self::get_home_dir()?;
        Ok(home.exists())
    }

    pub fn site_path(&self, name: &str) -> PathBuf {
        self.registry_dir.join(name)
    }
}
