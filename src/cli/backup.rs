use crate::cli::BackupCommand;
use crate::Config;
use anyhow::Result;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

fn safe_archive_path(base: &Path, archive_name: &str) -> Option<PathBuf> {
    let path = Path::new(archive_name);
    if path.is_absolute() {
        return None;
    }

    let mut candidate = base.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => candidate.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    Some(candidate)
}

fn extract_file_atomically<R: Read>(reader: &mut R, destination: &Path) -> Result<()> {
    let parent = destination.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;

    let mut temp_name = destination
        .file_name()
        .map(|name| name.to_os_string())
        .unwrap_or_default();
    temp_name.push(".tmp");
    let temp_path = parent.join(temp_name);

    {
        let mut outfile = File::create(&temp_path)?;
        std::io::copy(reader, &mut outfile)?;
        outfile.sync_all()?;
    }

    fs::rename(&temp_path, destination)?;
    Ok(())
}

pub async fn run(config_path: &Path, command: BackupCommand) -> Result<()> {
    let config = Config::load(config_path)?;

    match command {
        BackupCommand::Create { output } => {
            create_backup(&config, &output)?;
        }
        BackupCommand::Restore { file } => {
            restore_backup(&file, &config)?;
        }
        BackupCommand::List { dir } => {
            list_backups(&dir)?;
        }
    }

    Ok(())
}

pub fn create_backup(config: &Config, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("pebble-backup-{}.zip", timestamp);
    let backup_path = output_dir.join(&backup_name);

    let file = File::create(&backup_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let db_path = Path::new(&config.database.path);
    if db_path.exists() {
        let snapshot_path = output_dir.join(format!("pebble-backup-{}.db.tmp", timestamp));
        if snapshot_path.exists() {
            fs::remove_file(&snapshot_path)?;
        }

        {
            let db = crate::Database::open(&config.database.path)?;
            let conn = db.get()?;
            let snapshot = snapshot_path.to_string_lossy().to_string();
            conn.execute("VACUUM INTO ?1", [&snapshot])?;
        }

        let mut db_data = Vec::new();
        File::open(&snapshot_path)?.read_to_end(&mut db_data)?;
        zip.start_file("pebble.db", options)?;
        zip.write_all(&db_data)?;
        fs::remove_file(&snapshot_path).ok();
        tracing::info!("Added database: {} bytes", db_data.len());
    }

    let media_dir = Path::new(&config.media.upload_dir);
    if media_dir.exists() {
        let mut media_count = 0;
        for entry in fs::read_dir(media_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let filename = path
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("Invalid filename"))?
                    .to_string_lossy();
                let archive_path = format!("media/{}", filename);

                let mut file_data = Vec::new();
                File::open(&path)?.read_to_end(&mut file_data)?;
                zip.start_file(archive_path, options)?;
                zip.write_all(&file_data)?;
                media_count += 1;
            }
        }
        tracing::info!("Added {} media files", media_count);
    }

    let manifest = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "created_at": chrono::Utc::now().to_rfc3339(),
        "site_title": config.site.title,
    });
    zip.start_file("manifest.json", options)?;
    zip.write_all(manifest.to_string().as_bytes())?;

    zip.finish()?;
    tracing::info!("Backup created: {}", backup_path.display());
    Ok(())
}

fn restore_backup(archive_path: &Path, config: &Config) -> Result<()> {
    if !archive_path.exists() {
        anyhow::bail!("Backup file not found: {}", archive_path.display());
    }

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    let db_path = Path::new(&config.database.path);
    let db_dir = db_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(db_dir)?;

    let media_dir = Path::new(&config.media.upload_dir);
    fs::create_dir_all(media_dir)?;

    let canonical_db_dir = db_dir.canonicalize()?;
    let canonical_media_dir = media_dir.canonicalize()?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if name == "manifest.json" {
            continue;
        }

        let outpath = if name == "pebble.db" {
            db_path.to_path_buf()
        } else if let Some(relative) = name.strip_prefix("media/") {
            match safe_archive_path(media_dir, relative) {
                Some(path) => path,
                None => {
                    tracing::warn!("Skipping suspicious path in archive: {}", name);
                    continue;
                }
            }
        } else {
            continue;
        };

        let canonical_parent = outpath
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .unwrap_or_else(|| canonical_media_dir.clone());
        let file_name = match outpath.file_name() {
            Some(file_name) => file_name,
            None => {
                tracing::warn!("Skipping invalid output path for archive entry: {}", name);
                continue;
            }
        };
        let canonical_outpath = canonical_parent.join(file_name);

        let canonical_base = if name == "pebble.db" {
            &canonical_db_dir
        } else {
            &canonical_media_dir
        };

        if !canonical_outpath.starts_with(canonical_base) {
            tracing::warn!("Path traversal attempt blocked: {}", name);
            continue;
        }

        extract_file_atomically(&mut file, &outpath)?;
        tracing::info!("Restored: {}", outpath.display());
    }

    tracing::info!("Backup restored from: {}", archive_path.display());
    Ok(())
}

fn list_backups(dir: &Path) -> Result<()> {
    if !dir.exists() {
        tracing::info!("No backups directory found at {}", dir.display());
        return Ok(());
    }

    let mut backups: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "zip")
                .unwrap_or(false)
        })
        .collect();

    backups.sort_by_key(|e| e.path());
    backups.reverse();

    if backups.is_empty() {
        tracing::info!("No backups found in {}", dir.display());
        return Ok(());
    }

    println!("Available backups:");
    for entry in backups {
        let path = entry.path();
        let metadata = fs::metadata(&path)?;
        let size_mb = metadata.len() as f64 / (1024.0 * 1024.0);
        if let Some(filename) = path.file_name() {
            println!("  {} ({:.2} MB)", filename.to_string_lossy(), size_mb);
        }
    }

    Ok(())
}

/// Enforce backup retention by removing the oldest backups beyond the keep count.
pub fn enforce_retention(backup_dir: &Path, keep: usize) -> Result<()> {
    if !backup_dir.exists() || keep == 0 {
        return Ok(());
    }

    let mut backups: Vec<_> = fs::read_dir(backup_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let path = e.path();
            path.extension().map(|ext| ext == "zip").unwrap_or(false)
                && path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("pebble-backup-"))
                    .unwrap_or(false)
        })
        .collect();

    // Sort by filename (which includes timestamp) ascending
    backups.sort_by_key(|e| e.path());

    if backups.len() > keep {
        let to_remove = backups.len() - keep;
        for entry in backups.iter().take(to_remove) {
            let path = entry.path();
            if let Err(e) = fs::remove_file(&path) {
                tracing::warn!("Failed to remove old backup {}: {}", path.display(), e);
            } else {
                tracing::info!("Removed old backup: {}", path.display());
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{create_backup, restore_backup};
    use crate::{Config, Database};
    use std::io::Read;

    fn unique_path(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("{}_{}", prefix, nanos))
    }

    fn test_config(db_path: &std::path::Path, media_dir: &std::path::Path) -> Config {
        toml::from_str(&format!(
            r#"
[site]
title = "Test"
description = "Test"
url = "http://localhost:3000"

[server]
host = "127.0.0.1"
port = 3000

[database]
path = "{}"

[content]
posts_per_page = 10
excerpt_length = 200

[media]
upload_dir = "{}"

[theme]
name = "default"

[auth]
session_lifetime = "7d"
"#,
            db_path.display(),
            media_dir.display()
        ))
        .unwrap()
    }

    #[test]
    fn backup_includes_committed_wal_changes() {
        let site_dir = unique_path("pebble_backup_wal");
        let data_dir = site_dir.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        let db_path = data_dir.join("pebble.db");
        let media_dir = data_dir.join("media");
        let backup_dir = site_dir.join("backups");
        let config = test_config(&db_path, &media_dir);

        let db = Database::open(db_path.to_str().unwrap()).unwrap();
        db.migrate().unwrap();
        let conn = db.get().unwrap();
        conn.execute(
            "INSERT INTO content (slug, title, content_type, body_markdown, body_html, status)
             VALUES ('wal-post', 'WAL Post', 'post', 'body', '<p>body</p>', 'published')",
            [],
        )
        .unwrap();

        create_backup(&config, &backup_dir).unwrap();

        let backup_path = std::fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "zip"))
            .unwrap();
        let file = std::fs::File::open(backup_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        let mut db_file = archive.by_name("pebble.db").unwrap();
        let mut db_bytes = Vec::new();
        db_file.read_to_end(&mut db_bytes).unwrap();

        let restored_db_path = site_dir.join("restored.db");
        std::fs::write(&restored_db_path, db_bytes).unwrap();
        let restored = rusqlite::Connection::open(restored_db_path).unwrap();
        let count: i64 = restored
            .query_row(
                "SELECT COUNT(*) FROM content WHERE slug = 'wal-post'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
        drop(conn);
        drop(db);
        std::fs::remove_dir_all(site_dir).ok();
    }

    #[test]
    fn restore_places_media_at_upload_dir_root() {
        let site_dir = unique_path("pebble_restore_media");
        let data_dir = site_dir.join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        let db_path = data_dir.join("pebble.db");
        let media_dir = data_dir.join("media");
        std::fs::create_dir_all(&media_dir).unwrap();
        let backup_dir = site_dir.join("backups");
        let config = test_config(&db_path, &media_dir);

        // Seed a database and a single media file.
        let db = Database::open(db_path.to_str().unwrap()).unwrap();
        db.migrate().unwrap();
        drop(db);
        std::fs::write(media_dir.join("photo.png"), b"img-bytes").unwrap();

        create_backup(&config, &backup_dir).unwrap();

        // Wipe media to simulate restoring onto a clean target.
        std::fs::remove_dir_all(&media_dir).unwrap();

        let backup_path = std::fs::read_dir(&backup_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "zip"))
            .unwrap();

        restore_backup(&backup_path, &config).unwrap();

        // Media must land at <upload_dir>/photo.png — NOT nested under an extra media/ dir.
        assert!(
            media_dir.join("photo.png").exists(),
            "media restored to the wrong location"
        );
        assert!(
            !media_dir.join("media").exists(),
            "media incorrectly nested under an extra media/ directory"
        );
        assert_eq!(
            std::fs::read(media_dir.join("photo.png")).unwrap(),
            b"img-bytes"
        );

        std::fs::remove_dir_all(site_dir).ok();
    }
}
