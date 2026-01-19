use anyhow::Result;
use std::path::PathBuf;

pub async fn run(path: PathBuf, name: Option<String>) -> Result<()> {
    let site_name = name.unwrap_or_else(|| "My Site".to_string());

    std::fs::create_dir_all(&path)?;
    std::fs::create_dir_all(path.join("data"))?;
    std::fs::create_dir_all(path.join("data/media"))?;
    std::fs::create_dir_all(path.join("themes"))?;

    let config = format!(
        r#"[site]
title = "{}"
description = "A personal blog"
url = "http://localhost:3000"
language = "en"

[server]
host = "127.0.0.1"
port = 3000

[database]
path = "./data/pebble.db"

[content]
posts_per_page = 10
excerpt_length = 200
auto_excerpt = true

[media]
upload_dir = "./data/media"
max_upload_size = "10MB"

[theme]
name = "default"

[auth]
session_lifetime = "7d"
"#,
        site_name
    );

    std::fs::write(path.join("pebble.toml"), config)?;

    tracing::info!("Created new Pebble site at {:?}", path);
    tracing::info!("Run 'pebble migrate' to set up the database");
    tracing::info!("Run 'pebble serve' to start the server");

    Ok(())
}
