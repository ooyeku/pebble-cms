use crate::global::{GlobalConfig, PebbleHome, Registry, RegistrySite, SiteStatus};
use anyhow::{bail, Context, Result};
use std::fs;
use std::process::{Command, Stdio};

pub async fn run(command: super::RegistryCommand) -> Result<()> {
    let home = PebbleHome::init()?;
    let global_config = GlobalConfig::load(&home.config_path)?;
    let mut registry = Registry::load(&home.registry_path)?;

    registry.cleanup_dead_processes();

    match command {
        super::RegistryCommand::Init { name, title } => {
            init_site(&home, &global_config, &mut registry, &name, title)?;
            registry.save(&home.registry_path)?;
        }
        super::RegistryCommand::List => {
            list_sites(&registry);
        }
        super::RegistryCommand::Serve { name, port } => {
            serve_site(&home, &global_config, &mut registry, &name, port, false).await?;
            registry.save(&home.registry_path)?;
        }
        super::RegistryCommand::Deploy { name, port } => {
            serve_site(&home, &global_config, &mut registry, &name, port, true).await?;
            registry.save(&home.registry_path)?;
        }
        super::RegistryCommand::Stop { name } => {
            stop_site(&mut registry, &name)?;
            registry.save(&home.registry_path)?;
        }
        super::RegistryCommand::StopAll => {
            stop_all_sites(&mut registry)?;
            registry.save(&home.registry_path)?;
        }
        super::RegistryCommand::Remove { name, force } => {
            remove_site(&home, &mut registry, &name, force)?;
            registry.save(&home.registry_path)?;
        }
        super::RegistryCommand::Status { name } => {
            show_status(&registry, &name)?;
        }
        super::RegistryCommand::Path { name } => {
            show_path(&home, &registry, name)?;
        }
    }

    Ok(())
}

fn init_site(
    home: &PebbleHome,
    global_config: &GlobalConfig,
    registry: &mut Registry,
    name: &str,
    title: Option<String>,
) -> Result<()> {
    if !is_valid_site_name(name) {
        bail!("Invalid site name: must be lowercase alphanumeric with hyphens only");
    }

    let site_path = home.site_path(name);
    if site_path.exists() {
        bail!("Site directory already exists: {}", site_path.display());
    }

    if registry.get_site(name).is_some() {
        bail!("Site '{}' already registered", name);
    }

    fs::create_dir_all(&site_path)?;

    let site_title = title.unwrap_or_else(|| name.replace('-', " ").to_string());
    let config_path = site_path.join("pebble.toml");
    let config_content = create_site_config_toml(name, &site_title, global_config);
    fs::write(&config_path, config_content)?;

    let db_dir = site_path.join("data");
    fs::create_dir_all(&db_dir)?;

    let db_path = db_dir.join("pebble.db");
    let db = crate::Database::open(db_path.to_str().unwrap())?;
    crate::Database::migrate(&db)?;

    let media_dir = site_path.join("data").join("media");
    fs::create_dir_all(&media_dir)?;

    let site = RegistrySite {
        name: name.to_string(),
        title: site_title.clone(),
        description: String::new(),
        created_at: chrono::Utc::now().to_rfc3339(),
        status: SiteStatus::Stopped,
        port: None,
        pid: None,
        last_started: None,
    };
    registry.add_site(site)?;

    println!("Created site '{}' at {}", name, site_path.display());
    println!("Run: pebble registry serve {} to start it", name);

    Ok(())
}

fn create_site_config_toml(name: &str, title: &str, global_config: &GlobalConfig) -> String {
    format!(
        r#"[site]
title = "{}"
description = "A Pebble site: {}"
url = "http://localhost:{}"
language = "{}"

[server]
host = "127.0.0.1"
port = {}

[database]
path = "./data/pebble.db"

[content]
posts_per_page = {}
excerpt_length = {}
auto_excerpt = true

[media]
upload_dir = "./data/media"
max_upload_size = "10MB"

[theme]
name = "{}"

[auth]
session_lifetime = "7d"
"#,
        title,
        name,
        global_config.defaults.dev_port,
        global_config.defaults.language,
        global_config.defaults.dev_port,
        global_config.defaults.posts_per_page,
        global_config.defaults.excerpt_length,
        global_config.defaults.theme,
    )
}

fn list_sites(registry: &Registry) {
    let sites = registry.list_sites();

    if sites.is_empty() {
        println!("No sites registered.");
        println!("Run: pebble registry init <name> to create one");
        return;
    }

    println!(
        "{:<20} {:<12} {:<8} {:<30}",
        "NAME", "STATUS", "PORT", "TITLE"
    );
    println!("{}", "-".repeat(72));

    for site in sites {
        let port_str = site
            .port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "-".to_string());
        let title = if site.title.len() > 28 {
            format!("{}...", &site.title[..25])
        } else {
            site.title.clone()
        };
        println!(
            "{:<20} {:<12} {:<8} {:<30}",
            site.name,
            site.status.to_string(),
            port_str,
            title
        );
    }
}

async fn serve_site(
    home: &PebbleHome,
    global_config: &GlobalConfig,
    registry: &mut Registry,
    name: &str,
    port: Option<u16>,
    production: bool,
) -> Result<()> {
    let site = registry
        .get_site(name)
        .context(format!("Site '{}' not found in registry", name))?;

    if site.status == SiteStatus::Running {
        println!(
            "Site '{}' is already running on port {}",
            name,
            site.port.unwrap_or(0)
        );
        return Ok(());
    }

    let site_path = home.site_path(name);
    if !site_path.exists() {
        bail!("Site directory not found: {}", site_path.display());
    }

    let config_path = site_path.join("pebble.toml");
    if !config_path.exists() {
        bail!("Site config not found: {}", config_path.display());
    }

    let port = port.unwrap_or_else(|| {
        registry
            .find_available_port(
                global_config.registry.auto_port_range_start,
                global_config.registry.auto_port_range_end,
            )
            .unwrap_or(global_config.defaults.dev_port)
    });

    let exe = std::env::current_exe()?;
    let mode = if production { "deploy" } else { "serve" };
    let host = if production { "0.0.0.0" } else { "127.0.0.1" };

    let child = Command::new(&exe)
        .args([
            "--config",
            config_path.to_str().unwrap(),
            mode,
            "-H",
            host,
            "-p",
            &port.to_string(),
        ])
        .current_dir(&site_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn server process")?;

    let pid = child.id();
    let status = if production {
        SiteStatus::Deploying
    } else {
        SiteStatus::Running
    };

    registry.update_site_status(name, status, Some(port), Some(pid));

    println!("Started '{}' ({}) on http://{}:{}", name, mode, host, port);
    println!("PID: {}", pid);

    Ok(())
}

fn stop_site(registry: &mut Registry, name: &str) -> Result<()> {
    let site = registry
        .get_site(name)
        .context(format!("Site '{}' not found in registry", name))?;

    if site.status == SiteStatus::Stopped {
        println!("Site '{}' is not running", name);
        return Ok(());
    }

    if let Some(pid) = site.pid {
        kill_process(pid)?;
        println!("Stopped site '{}' (PID: {})", name, pid);
    }

    registry.update_site_status(name, SiteStatus::Stopped, None, None);
    Ok(())
}

fn stop_all_sites(registry: &mut Registry) -> Result<()> {
    let running: Vec<String> = registry
        .running_sites()
        .iter()
        .map(|s| s.name.clone())
        .collect();

    if running.is_empty() {
        println!("No sites are running");
        return Ok(());
    }

    for name in running {
        if let Some(site) = registry.get_site(&name) {
            if let Some(pid) = site.pid {
                kill_process(pid)?;
                println!("Stopped '{}' (PID: {})", name, pid);
            }
        }
        registry.update_site_status(&name, SiteStatus::Stopped, None, None);
    }

    Ok(())
}

fn remove_site(home: &PebbleHome, registry: &mut Registry, name: &str, force: bool) -> Result<()> {
    let site = registry
        .get_site(name)
        .context(format!("Site '{}' not found in registry", name))?;

    if site.status == SiteStatus::Running && !force {
        bail!("Site '{}' is running. Stop it first or use --force", name);
    }

    if site.status == SiteStatus::Running {
        if let Some(pid) = site.pid {
            let _ = kill_process(pid);
        }
    }

    let site_path = home.site_path(name);
    if site_path.exists() {
        fs::remove_dir_all(&site_path)
            .with_context(|| format!("Failed to remove site directory: {}", site_path.display()))?;
    }

    registry.remove_site(name);
    println!("Removed site '{}'", name);

    Ok(())
}

fn show_status(registry: &Registry, name: &str) -> Result<()> {
    let site = registry
        .get_site(name)
        .context(format!("Site '{}' not found in registry", name))?;

    println!("Name:        {}", site.name);
    println!("Title:       {}", site.title);
    println!("Status:      {}", site.status);
    if let Some(port) = site.port {
        println!("Port:        {}", port);
        println!("URL:         http://localhost:{}", port);
    }
    if let Some(pid) = site.pid {
        println!("PID:         {}", pid);
    }
    println!("Created:     {}", site.created_at);
    if let Some(ref started) = site.last_started {
        println!("Last Start:  {}", started);
    }

    Ok(())
}

fn show_path(home: &PebbleHome, registry: &Registry, name: Option<String>) -> Result<()> {
    match name {
        Some(n) => {
            registry
                .get_site(&n)
                .context(format!("Site '{}' not found in registry", n))?;
            println!("{}", home.site_path(&n).display());
        }
        None => {
            println!("{}", home.registry_dir.display());
        }
    }
    Ok(())
}

fn is_valid_site_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

#[cfg(unix)]
fn kill_process(pid: u32) -> Result<()> {
    unsafe {
        if libc::kill(pid as i32, libc::SIGTERM) != 0 {
            bail!("Failed to kill process {}", pid);
        }
    }
    Ok(())
}

#[cfg(windows)]
fn kill_process(pid: u32) -> Result<()> {
    Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/F"])
        .output()
        .context("Failed to kill process")?;
    Ok(())
}
