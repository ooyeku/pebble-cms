use crate::web;
use crate::Config;
use anyhow::Result;
use std::path::Path;

pub async fn run(config_path: &Path, host: &str, port: u16) -> Result<()> {
    let config = Config::load(config_path)?;

    tracing::info!("Deploying in production mode at http://{}:{}", host, port);
    tracing::info!("Admin routes disabled, read-only mode active");

    web::serve_production(&config, host, port).await
}
