use crate::models::{ContentStatus, ContentType, CreateContent};
use crate::services::content;
use crate::Config;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub async fn run(config_path: &Path, import_dir: &Path, overwrite: bool) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = crate::Database::open(&config.database.path)?;

    if !import_dir.exists() {
        anyhow::bail!("Import directory not found: {}", import_dir.display());
    }

    let posts_dir = import_dir.join("posts");
    let pages_dir = import_dir.join("pages");
    let media_dir = import_dir.join("media");

    let mut imported = 0;
    let mut skipped = 0;

    if posts_dir.exists() {
        let (i, s) = import_content_dir(
            &db,
            &posts_dir,
            ContentType::Post,
            overwrite,
            config.content.excerpt_length,
        )?;
        imported += i;
        skipped += s;
    }

    if pages_dir.exists() {
        let (i, s) = import_content_dir(
            &db,
            &pages_dir,
            ContentType::Page,
            overwrite,
            config.content.excerpt_length,
        )?;
        imported += i;
        skipped += s;
    }

    if media_dir.exists() {
        let dest_media = Path::new(&config.media.upload_dir);
        fs::create_dir_all(dest_media)?;

        for entry in fs::read_dir(&media_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let dest = dest_media.join(filename);
                    if !dest.exists() || overwrite {
                        fs::copy(&path, &dest)?;
                        tracing::info!("Copied media: {}", filename.to_string_lossy());
                    }
                }
            }
        }
    }

    tracing::info!(
        "Import complete: {} imported, {} skipped",
        imported,
        skipped
    );
    Ok(())
}

fn import_content_dir(
    db: &crate::Database,
    dir: &Path,
    content_type: ContentType,
    overwrite: bool,
    excerpt_length: usize,
) -> Result<(usize, usize)> {
    let mut imported = 0;
    let mut skipped = 0;

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().map(|e| e == "md").unwrap_or(false) {
            match import_markdown_file(db, &path, content_type.clone(), overwrite, excerpt_length) {
                Ok(true) => imported += 1,
                Ok(false) => skipped += 1,
                Err(e) => {
                    tracing::warn!("Failed to import {}: {}", path.display(), e);
                    skipped += 1;
                }
            }
        }
    }

    Ok((imported, skipped))
}

fn import_markdown_file(
    db: &crate::Database,
    path: &Path,
    content_type: ContentType,
    overwrite: bool,
    excerpt_length: usize,
) -> Result<bool> {
    let file_content = fs::read_to_string(path)?;
    let (frontmatter, body) = parse_frontmatter(&file_content)?;

    let slug = frontmatter
        .get("slug")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            path.file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let title = frontmatter
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    let status_str = frontmatter
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("draft");

    let status = match status_str {
        "published" => ContentStatus::Published,
        "archived" => ContentStatus::Archived,
        _ => ContentStatus::Draft,
    };

    let input = CreateContent {
        title,
        slug: Some(slug.clone()),
        content_type: content_type.clone(),
        body_markdown: body.to_string(),
        status,
        scheduled_at: None,
        excerpt: None,
        featured_image: None,
        tags: vec![],
        metadata: None,
    };

    // Atomically check for existing content and handle overwrite
    {
        let conn = db.get()?;
        let existing_id: Option<i64> = conn
            .query_row("SELECT id FROM content WHERE slug = ?", [&slug], |row| {
                row.get(0)
            })
            .ok();

        if let Some(id) = existing_id {
            if !overwrite {
                tracing::info!("Skipping existing: {}", slug);
                return Ok(false);
            }
            conn.execute("DELETE FROM content WHERE id = ?", [id])?;
        }
    }

    // The unique constraint on slug will catch any race condition
    match content::create_content(db, input, None, excerpt_length) {
        Ok(_) => {
            tracing::info!("Imported: {} ({})", slug, content_type);
            Ok(true)
        }
        Err(e) if e.to_string().contains("UNIQUE constraint") => {
            if !overwrite {
                tracing::info!("Skipping existing (race): {}", slug);
                return Ok(false);
            }
            Err(e)
        }
        Err(e) => Err(e),
    }
}

fn parse_frontmatter(content: &str) -> Result<(serde_json::Map<String, serde_json::Value>, &str)> {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return Ok((serde_json::Map::new(), content));
    }

    let after_first = &content[3..];
    let end_pos = after_first.find("---");

    match end_pos {
        Some(pos) => {
            let yaml_content = &after_first[..pos].trim();
            let body = &after_first[pos + 3..].trim_start();

            let mut map = serde_json::Map::new();
            for line in yaml_content.lines() {
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim().trim_matches('"').trim_matches('\'');
                    if !key.is_empty() {
                        map.insert(
                            key.to_string(),
                            serde_json::Value::String(value.to_string()),
                        );
                    }
                }
            }

            Ok((map, body))
        }
        None => Ok((serde_json::Map::new(), content)),
    }
}
