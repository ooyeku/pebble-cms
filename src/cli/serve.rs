use crate::services::{content, search};
use crate::{web, Config, Database};
use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use tokio::task::JoinHandle;

pub async fn run(config_path: &Path, host: &str, port: u16) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = Database::open(&config.database.path)?;

    db.migrate()?;

    if let Ok(count) = search::rebuild_fts_index(&db) {
        tracing::info!("Search index rebuilt: {} documents indexed", count);
    }

    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
    let mut bg_handles: Vec<JoinHandle<()>> = Vec::new();

    let scheduler_db = db.clone();
    let mut scheduler_rx = shutdown_rx.clone();
    bg_handles.push(tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Ok(count) = content::publish_scheduled(&scheduler_db) {
                        if count > 0 {
                            tracing::info!("Scheduled publisher: {} post(s) published", count);
                        }
                    }
                }
                _ = scheduler_rx.changed() => {
                    tracing::info!("Scheduled publisher stopping...");
                    break;
                }
            }
        }
    }));

    // Auto-backup scheduler
    if config.backup.auto_enabled {
        let backup_config = config.backup.clone();
        let backup_site_config = config.clone();
        let mut backup_rx = shutdown_rx.clone();
        bg_handles.push(tokio::spawn(async move {
            let interval_secs = backup_config.interval_hours.max(1) * 3600;
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            // Skip the first immediate tick
            interval.tick().await;
            loop {
                tokio::select! {
                    _ = interval.tick() => {
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
                    _ = backup_rx.changed() => {
                        tracing::info!("Auto-backup scheduler stopping...");
                        break;
                    }
                }
            }
        }));
        tracing::info!(
            "Auto-backup enabled: every {} hours, keeping {} backups in {}",
            config.backup.interval_hours,
            config.backup.retention_count,
            config.backup.directory
        );
    }

    let addr = format!("{}:{}", host, port);
    tracing::info!("Starting server at http://{}", addr);

    web::serve(config, config_path.to_path_buf(), db, &addr, Some(shutdown_rx)).await?;

    // Signal all background tasks to stop
    let _ = shutdown_tx.send(true);
    for handle in bg_handles {
        handle.abort();
    }

    Ok(())
}
