use crate::services::content;
use crate::{web, Config, Database};
use anyhow::Result;
use std::path::Path;
use std::time::Duration;

pub async fn run(config_path: &Path, host: &str, port: u16) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    db.migrate()?;

    let scheduler_db = db.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            if let Ok(count) = content::publish_scheduled(&scheduler_db) {
                if count > 0 {
                    tracing::info!("Scheduled publisher: {} post(s) published", count);
                }
            }
        }
    });

    let addr = format!("{}:{}", host, port);
    tracing::info!("Starting server at http://{}", addr);

    web::serve(config, db, &addr).await?;

    Ok(())
}
