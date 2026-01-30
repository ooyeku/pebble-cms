use crate::services::search;
use crate::web;
use crate::{Config, Database};
use anyhow::Result;
use std::path::Path;

pub async fn run(config_path: &Path, host: &str, port: u16) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    if let Ok(count) = search::rebuild_fts_index(&db) {
        tracing::info!("Search index rebuilt: {} documents indexed", count);
    }

    tracing::info!("Deploying in production mode at http://{}:{}", host, port);
    tracing::info!("Admin routes disabled, read-only mode active");

    web::serve_production(&config, config_path.to_path_buf(), host, port).await
}
