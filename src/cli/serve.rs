use crate::{web, Config, Database};
use anyhow::Result;
use std::path::Path;

pub async fn run(config_path: &Path, host: &str, port: u16) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    db.migrate()?;

    let addr = format!("{}:{}", host, port);
    tracing::info!("Starting server at http://{}", addr);

    web::serve(config, db, &addr).await?;

    Ok(())
}
