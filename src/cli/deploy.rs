use crate::services::search;
use crate::web;
use crate::{Config, Database};
use anyhow::Result;
use std::path::Path;
use std::time::Duration;

pub async fn run(config_path: &Path, host: &str, port: u16) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    if let Ok(count) = search::rebuild_fts_index(&db) {
        tracing::info!("Search index rebuilt: {} documents indexed", count);
    }

    // Auto-backup scheduler
    if config.backup.auto_enabled {
        let backup_config = config.backup.clone();
        let backup_site_config = config.clone();
        tokio::spawn(async move {
            let interval_secs = backup_config.interval_hours.max(1) * 3600;
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            // Skip the first immediate tick
            interval.tick().await;
            loop {
                interval.tick().await;
                let backup_dir = std::path::Path::new(&backup_config.directory);
                match crate::cli::backup::create_backup(&backup_site_config, backup_dir) {
                    Ok(()) => {
                        tracing::info!("Auto-backup completed successfully");
                        let _ = crate::cli::backup::enforce_retention(
                            backup_dir,
                            backup_config.retention_count,
                        );
                    }
                    Err(e) => {
                        tracing::error!("Auto-backup failed: {}", e);
                    }
                }
            }
        });
        tracing::info!(
            "Auto-backup enabled: every {} hours, keeping {} backups in {}",
            config.backup.interval_hours,
            config.backup.retention_count,
            config.backup.directory
        );
    }

    tracing::info!("Deploying in production mode at http://{}:{}", host, port);
    tracing::info!("Admin routes disabled, read-only mode active");

    web::serve_production(&config, config_path.to_path_buf(), host, port).await
}
