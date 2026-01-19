use crate::{Config, Database};
use anyhow::Result;
use std::path::Path;

pub async fn run(config_path: &Path) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    db.migrate()?;

    tracing::info!("Migrations complete");
    Ok(())
}
