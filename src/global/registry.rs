use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SiteStatus {
    Stopped,
    Running,
    Deploying,
}

impl Default for SiteStatus {
    fn default() -> Self {
        Self::Stopped
    }
}

impl std::fmt::Display for SiteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SiteStatus::Stopped => write!(f, "stopped"),
            SiteStatus::Running => write!(f, "running"),
            SiteStatus::Deploying => write!(f, "deploying"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrySite {
    pub name: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    #[serde(default)]
    pub status: SiteStatus,
    pub port: Option<u16>,
    pub pid: Option<u32>,
    #[serde(default)]
    pub last_started: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Registry {
    #[serde(default)]
    pub sites: HashMap<String, RegistrySite>,
}

impl Registry {
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read registry from {}", path.display()))?;
        let registry: Registry = toml::from_str(&content)
            .with_context(|| format!("Failed to parse registry from {}", path.display()))?;
        Ok(registry)
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize registry")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content)
            .with_context(|| format!("Failed to write registry to {}", path.display()))?;
        Ok(())
    }

    pub fn add_site(&mut self, site: RegistrySite) -> Result<()> {
        if self.sites.contains_key(&site.name) {
            bail!("Site '{}' already exists in registry", site.name);
        }
        self.sites.insert(site.name.clone(), site);
        Ok(())
    }

    pub fn get_site(&self, name: &str) -> Option<&RegistrySite> {
        self.sites.get(name)
    }

    pub fn get_site_mut(&mut self, name: &str) -> Option<&mut RegistrySite> {
        self.sites.get_mut(name)
    }

    pub fn remove_site(&mut self, name: &str) -> Option<RegistrySite> {
        self.sites.remove(name)
    }

    pub fn list_sites(&self) -> Vec<&RegistrySite> {
        let mut sites: Vec<_> = self.sites.values().collect();
        sites.sort_by(|a, b| a.name.cmp(&b.name));
        sites
    }

    pub fn running_sites(&self) -> Vec<&RegistrySite> {
        self.sites
            .values()
            .filter(|s| s.status == SiteStatus::Running)
            .collect()
    }

    pub fn find_available_port(&self, start: u16, end: u16) -> Option<u16> {
        let used_ports: std::collections::HashSet<u16> = self
            .sites
            .values()
            .filter_map(|s| {
                if s.status == SiteStatus::Running {
                    s.port
                } else {
                    None
                }
            })
            .collect();

        (start..=end).find(|port| !used_ports.contains(port) && is_port_available(*port))
    }

    pub fn update_site_status(
        &mut self,
        name: &str,
        status: SiteStatus,
        port: Option<u16>,
        pid: Option<u32>,
    ) {
        if let Some(site) = self.sites.get_mut(name) {
            site.status = status;
            site.port = port;
            site.pid = pid;
            if status == SiteStatus::Running {
                site.last_started = Some(chrono::Utc::now().to_rfc3339());
            }
        }
    }

    pub fn cleanup_dead_processes(&mut self) {
        for site in self.sites.values_mut() {
            if site.status == SiteStatus::Running {
                if let Some(pid) = site.pid {
                    if !is_process_running(pid) {
                        site.status = SiteStatus::Stopped;
                        site.port = None;
                        site.pid = None;
                    }
                }
            }
        }
    }
}

fn is_port_available(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

#[cfg(unix)]
fn is_process_running(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(windows)]
fn is_process_running(pid: u32) -> bool {
    use std::process::Command;
    Command::new("tasklist")
        .args(["/FI", &format!("PID eq {}", pid), "/NH"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()))
        .unwrap_or(false)
}
