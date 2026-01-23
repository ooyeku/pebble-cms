use crate::models::Media;
use crate::services::image as img_service;
use crate::Database;
use anyhow::{bail, Result};
use std::path::Path;
use uuid::Uuid;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

const ALLOWED_MIME_TYPES: &[&str] = &[
    "image/jpeg",
    "image/png",
    "image/gif",
    "image/webp",
    "image/svg+xml",
    "application/pdf",
    "video/mp4",
    "video/webm",
    "audio/mpeg",
    "audio/ogg",
];

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

    if !ALLOWED_MIME_TYPES.contains(&mime_type) {
        bail!(
            "File type not allowed: {}. Allowed types: {}",
            mime_type,
            ALLOWED_MIME_TYPES.join(", ")
        );
    }

    std::fs::create_dir_all(upload_dir)?;

    let base_uuid = Uuid::new_v4();

    let (filename, webp_filename, width, height, final_data) =
        if img_service::is_optimizable_image(mime_type) {
            match img_service::optimize_image(data, mime_type, None) {
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
                    let extension = Path::new(original_name)
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap_or("");
                    let filename = if extension.is_empty() {
                        base_uuid.to_string()
                    } else {
                        format!("{}.{}", base_uuid, extension)
                    };
                    std::fs::write(upload_dir.join(&filename), data)?;
                    (filename, None, None, None, data.to_vec())
                }
            }
        } else {
            let extension = Path::new(original_name)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let filename = if extension.is_empty() {
                base_uuid.to_string()
            } else {
                format!("{}.{}", base_uuid, extension)
            };
            std::fs::write(upload_dir.join(&filename), data)?;
            (filename, None, None, None, data.to_vec())
        };

    let conn = db.get()?;
    conn.execute(
        "INSERT INTO media (filename, original_name, mime_type, size_bytes, uploaded_by, webp_filename, width, height) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        (&filename, original_name, mime_type, final_data.len() as i64, uploaded_by, &webp_filename, width, height),
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
        mime_type: mime_type.to_string(),
        size_bytes: final_data.len() as i64,
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
