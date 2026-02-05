use crate::config::Config;
use crate::global::{GlobalConfig, PebbleHome, Registry, RegistrySite, SiteStatus};
use anyhow::{bail, Context, Result};
use std::fs::{self, File};
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
        super::RegistryCommand::Rerender { name } => {
            rerender_site(&home, &registry, &name)?;
        }
        super::RegistryCommand::Config { name, command } => {
            site_config(&home, &global_config, &mut registry, &name, command).await?;
            registry.save(&home.registry_path)?;
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

    let port = match port {
        Some(p) => p,
        None => {
            // Read port from site config, fall back to auto-assign if not available
            Config::load(&config_path)
                .map(|c| c.server.port)
                .unwrap_or_else(|_| {
                    registry
                        .find_available_port(
                            global_config.registry.auto_port_range_start,
                            global_config.registry.auto_port_range_end,
                        )
                        .unwrap_or(global_config.defaults.dev_port)
                })
        }
    };

    let exe = std::env::current_exe()?;
    let mode = if production { "deploy" } else { "serve" };
    let host = if production { "0.0.0.0" } else { "127.0.0.1" };

    let logs_dir = site_path.join("logs");
    fs::create_dir_all(&logs_dir)?;
    let log_file = logs_dir.join(format!("{}.log", name));
    let stdout_file = File::create(&log_file).context("Failed to create log file")?;
    let stderr_file = stdout_file
        .try_clone()
        .context("Failed to clone log file handle")?;

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
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
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
    println!("Logs: {}", log_file.display());

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

fn rerender_site(home: &PebbleHome, registry: &Registry, name: &str) -> Result<()> {
    registry
        .get_site(name)
        .context(format!("Site '{}' not found in registry", name))?;

    let site_path = home.site_path(name);
    let config_path = site_path.join("pebble.toml");

    if !config_path.exists() {
        bail!("Config file not found: {}", config_path.display());
    }

    let config = Config::load(&config_path)?;
    let db = crate::Database::open(&config.database.path)?;

    println!("Re-rendering all content for '{}'...", name);
    let count = crate::services::content::rerender_all_content(&db)?;
    println!("Successfully re-rendered {} content items.", count);

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

async fn site_config(
    home: &PebbleHome,
    global_config: &GlobalConfig,
    registry: &mut Registry,
    name: &str,
    command: Option<super::SiteConfigCommand>,
) -> Result<()> {
    let site = registry
        .get_site(name)
        .context(format!("Site '{}' not found in registry", name))?;

    let was_running = site.status == SiteStatus::Running;

    let site_path = home.site_path(name);
    let config_path = site_path.join("pebble.toml");

    if !config_path.exists() {
        bail!("Config file not found: {}", config_path.display());
    }

    let needs_restart = match &command {
        None => false,
        Some(super::SiteConfigCommand::Get { .. }) => false,
        Some(super::SiteConfigCommand::Set { .. }) => true,
        Some(super::SiteConfigCommand::Edit) => true,
    };

    match command {
        None => {
            // Show full config
            show_site_config(&config_path)?;
        }
        Some(super::SiteConfigCommand::Get { key }) => {
            get_site_config_value(&config_path, &key)?;
        }
        Some(super::SiteConfigCommand::Set { key, value }) => {
            set_site_config_value(&config_path, &key, &value)?;
        }
        Some(super::SiteConfigCommand::Edit) => {
            edit_site_config(&config_path)?;
        }
    }

    // Restart site if it was running and config was modified
    if needs_restart && was_running {
        println!("Restarting site to apply changes...");
        stop_site(registry, name)?;
        // Use None for port to let it read from the updated config
        serve_site(home, global_config, registry, name, None, false).await?;
    }

    Ok(())
}

fn show_site_config(config_path: &std::path::Path) -> Result<()> {
    let config = Config::load(config_path)?;

    println!("# Site");
    println!("{:<30}  {}", "site.title", config.site.title);
    println!("{:<30}  {}", "site.description", config.site.description);
    println!("{:<30}  {}", "site.url", config.site.url);
    println!("{:<30}  {}", "site.language", config.site.language);
    println!();

    println!("# Server");
    println!("{:<30}  {}", "server.host", config.server.host);
    println!("{:<30}  {}", "server.port", config.server.port);
    println!();

    println!("# Content");
    println!(
        "{:<30}  {}",
        "content.posts_per_page", config.content.posts_per_page
    );
    println!(
        "{:<30}  {}",
        "content.excerpt_length", config.content.excerpt_length
    );
    println!(
        "{:<30}  {}",
        "content.auto_excerpt", config.content.auto_excerpt
    );
    println!();

    println!("# Theme");
    println!("{:<30}  {}", "theme.name", config.theme.name);
    if let Some(ref v) = config.theme.custom.primary_color {
        println!("{:<30}  {}", "theme.custom.primary_color", v);
    }
    if let Some(ref v) = config.theme.custom.accent_color {
        println!("{:<30}  {}", "theme.custom.accent_color", v);
    }
    if let Some(ref v) = config.theme.custom.background_color {
        println!("{:<30}  {}", "theme.custom.background_color", v);
    }
    if let Some(ref v) = config.theme.custom.text_color {
        println!("{:<30}  {}", "theme.custom.text_color", v);
    }
    if let Some(ref v) = config.theme.custom.font_family {
        println!("{:<30}  {}", "theme.custom.font_family", v);
    }
    println!();

    println!("# Homepage");
    println!(
        "{:<30}  {}",
        "homepage.show_hero", config.homepage.show_hero
    );
    println!(
        "{:<30}  {}",
        "homepage.hero_layout", config.homepage.hero_layout
    );
    println!(
        "{:<30}  {}",
        "homepage.show_posts", config.homepage.show_posts
    );
    println!(
        "{:<30}  {}",
        "homepage.posts_layout", config.homepage.posts_layout
    );
    println!(
        "{:<30}  {}",
        "homepage.show_pages", config.homepage.show_pages
    );

    Ok(())
}

fn get_site_config_value(config_path: &std::path::Path, key: &str) -> Result<()> {
    let config = Config::load(config_path)?;
    let value = get_config_value(&config, key)?;
    println!("{}", value);
    Ok(())
}

fn get_config_value(config: &Config, key: &str) -> Result<String> {
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        // Site
        ["site", "title"] => Ok(config.site.title.clone()),
        ["site", "description"] => Ok(config.site.description.clone()),
        ["site", "url"] => Ok(config.site.url.clone()),
        ["site", "language"] => Ok(config.site.language.clone()),
        // Server
        ["server", "host"] => Ok(config.server.host.clone()),
        ["server", "port"] => Ok(config.server.port.to_string()),
        // Content
        ["content", "posts_per_page"] => Ok(config.content.posts_per_page.to_string()),
        ["content", "excerpt_length"] => Ok(config.content.excerpt_length.to_string()),
        ["content", "auto_excerpt"] => Ok(config.content.auto_excerpt.to_string()),
        // Theme
        ["theme", "name"] => Ok(config.theme.name.clone()),
        ["theme", "custom", "primary_color"] => Ok(config
            .theme
            .custom
            .primary_color
            .clone()
            .unwrap_or_default()),
        ["theme", "custom", "accent_color"] => {
            Ok(config.theme.custom.accent_color.clone().unwrap_or_default())
        }
        ["theme", "custom", "background_color"] => Ok(config
            .theme
            .custom
            .background_color
            .clone()
            .unwrap_or_default()),
        ["theme", "custom", "text_color"] => {
            Ok(config.theme.custom.text_color.clone().unwrap_or_default())
        }
        ["theme", "custom", "font_family"] => {
            Ok(config.theme.custom.font_family.clone().unwrap_or_default())
        }
        // Homepage
        ["homepage", "show_hero"] => Ok(config.homepage.show_hero.to_string()),
        ["homepage", "hero_layout"] => Ok(config.homepage.hero_layout.clone()),
        ["homepage", "hero_height"] => Ok(config.homepage.hero_height.clone()),
        ["homepage", "show_posts"] => Ok(config.homepage.show_posts.to_string()),
        ["homepage", "posts_layout"] => Ok(config.homepage.posts_layout.clone()),
        ["homepage", "posts_columns"] => Ok(config.homepage.posts_columns.to_string()),
        ["homepage", "show_pages"] => Ok(config.homepage.show_pages.to_string()),
        ["homepage", "pages_layout"] => Ok(config.homepage.pages_layout.clone()),
        // Auth
        ["auth", "session_lifetime"] => Ok(config.auth.session_lifetime.clone()),
        _ => bail!("Unknown config key: {}", key),
    }
}

fn set_site_config_value(config_path: &std::path::Path, key: &str, value: &str) -> Result<()> {
    let content = fs::read_to_string(config_path)?;
    let mut doc = content
        .parse::<toml_edit::DocumentMut>()
        .context("Failed to parse config file")?;

    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        // Site
        ["site", "title"] => doc["site"]["title"] = toml_edit::value(value),
        ["site", "description"] => doc["site"]["description"] = toml_edit::value(value),
        ["site", "url"] => doc["site"]["url"] = toml_edit::value(value),
        ["site", "language"] => doc["site"]["language"] = toml_edit::value(value),
        // Server
        ["server", "host"] => doc["server"]["host"] = toml_edit::value(value),
        ["server", "port"] => {
            let port: i64 = value.parse().context("Invalid port number")?;
            doc["server"]["port"] = toml_edit::value(port);
        }
        // Content
        ["content", "posts_per_page"] => {
            let n: i64 = value.parse().context("Invalid number")?;
            doc["content"]["posts_per_page"] = toml_edit::value(n);
        }
        ["content", "excerpt_length"] => {
            let n: i64 = value.parse().context("Invalid number")?;
            doc["content"]["excerpt_length"] = toml_edit::value(n);
        }
        ["content", "auto_excerpt"] => {
            let b: bool = value.parse().context("Invalid boolean (use true/false)")?;
            doc["content"]["auto_excerpt"] = toml_edit::value(b);
        }
        // Theme
        ["theme", "name"] => {
            if !crate::config::ThemeConfig::is_valid_theme(value) {
                bail!(
                    "Invalid theme '{}'. Available: {}",
                    value,
                    crate::config::ThemeConfig::AVAILABLE_THEMES.join(", ")
                );
            }
            doc["theme"]["name"] = toml_edit::value(value);
        }
        ["theme", "custom", field] => {
            if !doc.contains_key("theme") {
                doc["theme"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            if !doc["theme"]
                .as_table()
                .map_or(false, |t| t.contains_key("custom"))
            {
                doc["theme"]["custom"] = toml_edit::Item::Table(toml_edit::Table::new());
            }
            doc["theme"]["custom"][*field] = toml_edit::value(value);
        }
        // Homepage
        ["homepage", "show_hero"] => {
            let b: bool = value.parse().context("Invalid boolean (use true/false)")?;
            ensure_homepage_table(&mut doc);
            doc["homepage"]["show_hero"] = toml_edit::value(b);
        }
        ["homepage", "hero_layout"] => {
            ensure_homepage_table(&mut doc);
            doc["homepage"]["hero_layout"] = toml_edit::value(value);
        }
        ["homepage", "hero_height"] => {
            ensure_homepage_table(&mut doc);
            doc["homepage"]["hero_height"] = toml_edit::value(value);
        }
        ["homepage", "show_posts"] => {
            let b: bool = value.parse().context("Invalid boolean (use true/false)")?;
            ensure_homepage_table(&mut doc);
            doc["homepage"]["show_posts"] = toml_edit::value(b);
        }
        ["homepage", "posts_layout"] => {
            ensure_homepage_table(&mut doc);
            doc["homepage"]["posts_layout"] = toml_edit::value(value);
        }
        ["homepage", "posts_columns"] => {
            let n: i64 = value.parse().context("Invalid number")?;
            ensure_homepage_table(&mut doc);
            doc["homepage"]["posts_columns"] = toml_edit::value(n);
        }
        ["homepage", "show_pages"] => {
            let b: bool = value.parse().context("Invalid boolean (use true/false)")?;
            ensure_homepage_table(&mut doc);
            doc["homepage"]["show_pages"] = toml_edit::value(b);
        }
        ["homepage", "pages_layout"] => {
            ensure_homepage_table(&mut doc);
            doc["homepage"]["pages_layout"] = toml_edit::value(value);
        }
        // Auth
        ["auth", "session_lifetime"] => {
            doc["auth"]["session_lifetime"] = toml_edit::value(value);
        }
        _ => bail!("Unknown or read-only config key: {}", key),
    }

    fs::write(config_path, doc.to_string())?;
    println!("Set {} = {}", key, value);
    Ok(())
}

fn ensure_homepage_table(doc: &mut toml_edit::DocumentMut) {
    if !doc.contains_key("homepage") {
        doc["homepage"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
}

fn edit_site_config(config_path: &std::path::Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
        if cfg!(target_os = "windows") {
            "notepad".to_string()
        } else {
            "vi".to_string()
        }
    });

    let status = Command::new(&editor)
        .arg(config_path)
        .status()
        .with_context(|| format!("Failed to open editor: {}", editor))?;

    if !status.success() {
        bail!("Editor exited with error");
    }

    // Validate the config after editing
    Config::load(config_path).context("Config validation failed after editing")?;
    println!("Config saved and validated successfully");

    Ok(())
}
