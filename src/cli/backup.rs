use crate::cli::BackupCommand;
use crate::Config;
use anyhow::Result;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

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

fn create_backup(config: &Config, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let backup_name = format!("pebble-backup-{}.zip", timestamp);
    let backup_path = output_dir.join(&backup_name);

    let file = File::create(&backup_path)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let db_path = Path::new(&config.database.path);
    if db_path.exists() {
        let mut db_data = Vec::new();
        File::open(db_path)?.read_to_end(&mut db_data)?;
        zip.start_file("pebble.db", options)?;
        zip.write_all(&db_data)?;
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

        if name.contains("..") {
            tracing::warn!("Skipping suspicious path in archive: {}", name);
            continue;
        }

        let (outpath, canonical_base) = if name == "pebble.db" {
            (db_path.to_path_buf(), &canonical_db_dir)
        } else if name.starts_with("media/") {
            let filename = name.strip_prefix("media/").unwrap_or(&name);
            if filename.contains('/') || filename.contains('\\') {
                tracing::warn!("Skipping nested media path: {}", name);
                continue;
            }
            (media_dir.join(filename), &canonical_media_dir)
        } else {
            continue;
        };

        if let Some(parent) = outpath.parent() {
            fs::create_dir_all(parent)?;
        }

        let canonical_outpath = if outpath.exists() {
            outpath.canonicalize()?
        } else if let Some(parent) = outpath.parent() {
            parent
                .canonicalize()?
                .join(outpath.file_name().unwrap_or_default())
        } else {
            continue;
        };

        if !canonical_outpath.starts_with(canonical_base) {
            tracing::warn!("Path traversal attempt blocked: {}", name);
            continue;
        }

        let mut outfile = File::create(&outpath)?;
        std::io::copy(&mut file, &mut outfile)?;
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
