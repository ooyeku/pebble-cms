use crate::models::Media;
use crate::services::image as img_service;
use crate::Database;
use anyhow::{bail, Result};
use std::path::Path;
use uuid::Uuid;

pub const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

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

fn sanitize_svg(data: &[u8]) -> Result<Vec<u8>> {
    let content = String::from_utf8_lossy(data);

    let dangerous_patterns = [
        "<script",
        "javascript:",
        "onload=",
        "onerror=",
        "onclick=",
        "onmouseover=",
        "onfocus=",
        "onblur=",
        "onchange=",
        "onsubmit=",
        "eval(",
        "expression(",
        "url(data:",
        "xlink:href=\"javascript",
        "xlink:href='javascript",
    ];

    let lower_content = content.to_lowercase();
    for pattern in dangerous_patterns {
        if lower_content.contains(pattern) {
            bail!("SVG contains potentially dangerous content: {}", pattern);
        }
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
    let thumb_path = upload_dir.join(format!("{}-thumb.webp", base_name));
    if thumb_path.exists() {
        std::fs::remove_file(thumb_path)?;
    }

    conn.execute("DELETE FROM media WHERE id = ?", [id])?;
    Ok(())
}

pub fn update_media_alt(db: &Database, id: i64, alt_text: &str) -> Result<()> {
    let conn = db.get()?;
    conn.execute("UPDATE media SET alt_text = ? WHERE id = ?", (alt_text, id))?;
    Ok(())
}
