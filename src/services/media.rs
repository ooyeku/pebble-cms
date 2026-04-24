use crate::models::Media;
use crate::services::image as img_service;
use crate::Database;
use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::Path;
use uuid::Uuid;

pub const MAX_FILE_SIZE: usize = 50 * 1024 * 1024;

pub const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "application/pdf",
    "video/mp4",
    "video/webm",
    "audio/mpeg",
    "audio/ogg",
];

static SVG_NUMERIC_ENTITY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"&#(x[0-9a-fA-F]+|\d+);?").expect("valid SVG entity regex"));
static SVG_DANGEROUS_TAG: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"<\s*/?\s*(script|foreignobject|iframe|object|embed|link|meta|base)\b")
        .expect("valid SVG tag regex")
});
static SVG_EVENT_ATTR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s+on[a-z0-9_-]+\s*=").expect("valid SVG event regex"));
static SVG_DANGEROUS_URI: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"\b(href|xlink:href|src)\s*=\s*['"]?\s*(javascript|vbscript|data)\s*:"#)
        .expect("valid SVG URI regex")
});
static SVG_DANGEROUS_CSS_URL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"url\s*\(\s*['"]?\s*(javascript|vbscript|data)\s*:"#)
        .expect("valid SVG CSS URL regex")
});

fn detect_mime_type(data: &[u8], claimed_mime: &str) -> Option<String> {
    if let Some(kind) = infer::get(data) {
        return Some(kind.mime_type().to_string());
    }

    if claimed_mime == "image/svg+xml" && data.len() > 5 {
        let start = String::from_utf8_lossy(&data[..data.len().min(1000)]);
        if start.contains("<svg") || start.contains("<?xml") {
            return Some("image/svg+xml".to_string());
        }
    }

    None
}

fn normalize_svg_for_checks(content: &str) -> String {
    let decoded = SVG_NUMERIC_ENTITY
        .replace_all(content, |caps: &regex::Captures| {
            let raw = &caps[1];
            let code = raw
                .strip_prefix('x')
                .or_else(|| raw.strip_prefix('X'))
                .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                .or_else(|| raw.parse::<u32>().ok());

            code.and_then(char::from_u32)
                .map(|ch| ch.to_string())
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string();

    decoded
        .to_lowercase()
        .replace("&colon;", ":")
        .replace("&tab;", "\t")
        .replace("&newline;", "\n")
        .replace("&#x09;", "\t")
        .replace("&#x0a;", "\n")
        .replace("&#x0d;", "\r")
}

fn sanitize_svg(data: &[u8]) -> Result<Vec<u8>> {
    let content = std::str::from_utf8(data)?;
    let normalized = normalize_svg_for_checks(content);

    if SVG_DANGEROUS_TAG.is_match(&normalized)
        || SVG_EVENT_ATTR.is_match(&normalized)
        || SVG_DANGEROUS_URI.is_match(&normalized)
        || SVG_DANGEROUS_CSS_URL.is_match(&normalized)
        || normalized.contains("<?xml-stylesheet")
        || normalized.contains("expression(")
        || normalized.contains("-moz-binding")
    {
        bail!("SVG contains potentially dangerous content");
    }

    Ok(data.to_vec())
}

fn get_safe_extension(detected_mime: &str) -> Option<&'static str> {
    match detected_mime {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "application/pdf" => Some("pdf"),
        "video/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        "audio/mpeg" => Some("mp3"),
        "audio/ogg" => Some("ogg"),
        _ => None,
    }
}

pub fn upload_media(
    db: &Database,
    upload_dir: &Path,
    original_name: &str,
    mime_type: &str,
    data: &[u8],
    uploaded_by: Option<i64>,
) -> Result<Media> {
    if data.len() > MAX_FILE_SIZE {
        bail!(
            "File too large: {} bytes (max {} bytes)",
            data.len(),
            MAX_FILE_SIZE
        );
    }

    let detected_mime = detect_mime_type(data, mime_type);
    let actual_mime = detected_mime.as_deref().unwrap_or(mime_type);

    let is_svg = actual_mime == "image/svg+xml";
    if !ALLOWED_MIME_TYPES.contains(&actual_mime) && !is_svg {
        bail!(
            "File type not allowed: {}. Allowed types: {}",
            actual_mime,
            ALLOWED_MIME_TYPES.join(", ")
        );
    }

    let final_data = if is_svg {
        sanitize_svg(data)?
    } else {
        data.to_vec()
    };

    std::fs::create_dir_all(upload_dir)?;

    let base_uuid = Uuid::new_v4();

    let (filename, webp_filename, width, height, stored_data) =
        if img_service::is_optimizable_image(actual_mime) {
            match img_service::optimize_image(&final_data, actual_mime, None) {
                Ok(optimized) => {
                    let ext = match optimized.original_format {
                        image::ImageFormat::Jpeg => "jpg",
                        image::ImageFormat::Png => "png",
                        image::ImageFormat::Gif => "gif",
                        image::ImageFormat::WebP => "webp",
                        _ => "bin",
                    };

                    let filename = format!("{}.{}", base_uuid, ext);
                    let webp_name = format!("{}.webp", base_uuid);

                    std::fs::write(upload_dir.join(&filename), &optimized.original)?;
                    std::fs::write(upload_dir.join(&webp_name), &optimized.webp)?;

                    if let Ok(thumb_data) =
                        img_service::generate_thumbnail(&optimized.original, None)
                    {
                        let thumb_name = format!("{}-thumb.webp", base_uuid);
                        std::fs::write(upload_dir.join(&thumb_name), thumb_data)?;
                    }

                    // Generate responsive srcset variants (400w, 800w, 1200w, 1600w)
                    if let Ok(variants) = img_service::generate_srcset_variants(&optimized.original)
                    {
                        for variant in variants {
                            let variant_name = format!("{}{}.webp", base_uuid, variant.suffix);
                            let _ = std::fs::write(upload_dir.join(&variant_name), &variant.data);
                        }
                    }

                    (
                        filename,
                        Some(webp_name),
                        Some(optimized.width),
                        Some(optimized.height),
                        optimized.original,
                    )
                }
                Err(_) => {
                    let extension = get_safe_extension(actual_mime).unwrap_or("bin");
                    let filename = format!("{}.{}", base_uuid, extension);
                    std::fs::write(upload_dir.join(&filename), &final_data)?;
                    (filename, None, None, None, final_data.clone())
                }
            }
        } else {
            let extension = get_safe_extension(actual_mime).unwrap_or("bin");
            let filename = format!("{}.{}", base_uuid, extension);
            std::fs::write(upload_dir.join(&filename), &final_data)?;
            (filename, None, None, None, final_data.clone())
        };

    let conn = db.get()?;
    conn.execute(
        "INSERT INTO media (filename, original_name, mime_type, size_bytes, uploaded_by, webp_filename, width, height) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (&filename, original_name, actual_mime, stored_data.len() as i64, uploaded_by, &webp_filename, width, height),
    )?;

    let id = conn.last_insert_rowid();
    let created_at: String =
        conn.query_row("SELECT created_at FROM media WHERE id = ?", [id], |row| {
            row.get(0)
        })?;

    Ok(Media {
        id,
        filename,
        original_name: original_name.to_string(),
        mime_type: actual_mime.to_string(),
        size_bytes: stored_data.len() as i64,
        alt_text: String::new(),
        uploaded_by,
        created_at,
    })
}

pub fn count_media(db: &Database) -> Result<i64> {
    let conn = db.get()?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM media", [], |row| row.get(0))?;
    Ok(count)
}

pub fn list_media(db: &Database, limit: usize, offset: usize) -> Result<Vec<Media>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, filename, original_name, mime_type, size_bytes, alt_text, uploaded_by, created_at FROM media ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )?;
    let media = stmt
        .query_map((limit, offset), |row| {
            Ok(Media {
                id: row.get(0)?,
                filename: row.get(1)?,
                original_name: row.get(2)?,
                mime_type: row.get(3)?,
                size_bytes: row.get(4)?,
                alt_text: row.get(5)?,
                uploaded_by: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(media)
}

pub fn get_media_by_filename(db: &Database, filename: &str) -> Result<Option<Media>> {
    let conn = db.get()?;
    let media = conn
        .query_row(
            "SELECT id, filename, original_name, mime_type, size_bytes, alt_text, uploaded_by, created_at FROM media WHERE filename = ?",
            [filename],
            |row| {
                Ok(Media {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    original_name: row.get(2)?,
                    mime_type: row.get(3)?,
                    size_bytes: row.get(4)?,
                    alt_text: row.get(5)?,
                    uploaded_by: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
        .ok();
    Ok(media)
}

pub fn delete_media(db: &Database, upload_dir: &Path, id: i64) -> Result<()> {
    let conn = db.get()?;

    let (filename, webp_filename): (String, Option<String>) = conn.query_row(
        "SELECT filename, webp_filename FROM media WHERE id = ?",
        [id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    let file_path = upload_dir.join(&filename);
    if file_path.exists() {
        std::fs::remove_file(file_path)?;
    }

    if let Some(webp) = webp_filename {
        let webp_path = upload_dir.join(&webp);
        if webp_path.exists() {
            std::fs::remove_file(webp_path)?;
        }
    }

    let base_name = filename
        .rsplit_once('.')
        .map(|(n, _)| n)
        .unwrap_or(&filename);
    remove_generated_derivatives(upload_dir, base_name)?;

    conn.execute("DELETE FROM media WHERE id = ?", [id])?;
    Ok(())
}

pub fn update_media_alt(db: &Database, id: i64, alt_text: &str) -> Result<()> {
    let conn = db.get()?;
    conn.execute("UPDATE media SET alt_text = ? WHERE id = ?", (alt_text, id))?;
    Ok(())
}

fn remove_generated_derivatives(upload_dir: &Path, base_name: &str) -> Result<()> {
    let derivative_prefix = format!("{}-", base_name);
    let entries = match std::fs::read_dir(upload_dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err.into()),
    };

    for entry in entries {
        let entry = entry?;
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };

        if file_name.starts_with(&derivative_prefix) && file_name.ends_with(".webp") {
            let path = entry.path();
            if path.is_file() {
                std::fs::remove_file(path)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{count_media, delete_media, sanitize_svg, upload_media};
    use crate::Database;
    use image::{DynamicImage, ImageFormat};
    use std::io::Cursor;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_name(prefix: &str) -> String {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        format!("{}_{}", prefix, unique)
    }

    #[test]
    fn delete_media_removes_generated_variants() {
        let db = Database::open_memory(&unique_name("media_delete_variants")).unwrap();
        db.migrate().unwrap();

        let upload_dir = std::env::temp_dir().join(unique_name("media_uploads"));
        std::fs::create_dir_all(&upload_dir).unwrap();

        let image = DynamicImage::new_rgba8(800, 600);
        let mut buffer = Cursor::new(Vec::new());
        image.write_to(&mut buffer, ImageFormat::Png).unwrap();
        let data = buffer.into_inner();

        let media =
            upload_media(&db, &upload_dir, "example.png", "image/png", &data, None).unwrap();

        let base_name = media
            .filename
            .rsplit_once('.')
            .map(|(name, _)| name.to_string())
            .unwrap();

        let derivative_count_before = std::fs::read_dir(&upload_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with(&format!("{}-", base_name)) && name.ends_with(".webp"))
            .count();
        assert!(derivative_count_before > 0);

        delete_media(&db, &upload_dir, media.id).unwrap();

        let derivative_count_after = std::fs::read_dir(&upload_dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with(&format!("{}-", base_name)) && name.ends_with(".webp"))
            .count();
        assert_eq!(derivative_count_after, 0);

        std::fs::remove_dir_all(&upload_dir).ok();
    }

    #[test]
    fn sanitize_svg_rejects_obfuscated_script_vectors() {
        assert!(sanitize_svg(br#"<svg onload = "alert(1)"></svg>"#).is_err());
        assert!(sanitize_svg(br#"<svg><a href="java&#x73;cript:alert(1)">x</a></svg>"#).is_err());
    }

    #[test]
    fn delete_media_removes_row_when_upload_dir_is_missing() {
        let db = Database::open_memory(&unique_name("media_delete_missing_dir")).unwrap();
        db.migrate().unwrap();
        let upload_dir = std::env::temp_dir().join(unique_name("missing_media_uploads"));

        let conn = db.get().unwrap();
        conn.execute(
            "INSERT INTO media (filename, original_name, mime_type, size_bytes) VALUES (?1, ?2, ?3, ?4)",
            ("stale.pdf", "stale.pdf", "application/pdf", 12_i64),
        )
        .unwrap();
        let media_id = conn.last_insert_rowid();
        drop(conn);

        assert!(!upload_dir.exists());
        delete_media(&db, &upload_dir, media_id).unwrap();
        assert_eq!(count_media(&db).unwrap(), 0);
    }
}
