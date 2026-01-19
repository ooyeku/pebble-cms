use crate::models::Media;
use crate::Database;
use anyhow::{bail, Result};
use std::path::Path;
use uuid::Uuid;

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024; // 10MB

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
    // Validate file size
    if data.len() > MAX_FILE_SIZE {
        bail!(
            "File too large: {} bytes (max {} bytes)",
            data.len(),
            MAX_FILE_SIZE
        );
    }

    // Validate mime type
    if !ALLOWED_MIME_TYPES.contains(&mime_type) {
        bail!(
            "File type not allowed: {}. Allowed types: {}",
            mime_type,
            ALLOWED_MIME_TYPES.join(", ")
        );
    }

    let extension = Path::new(original_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let filename = if extension.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        format!("{}.{}", Uuid::new_v4(), extension)
    };

    std::fs::create_dir_all(upload_dir)?;
    let file_path = upload_dir.join(&filename);
    std::fs::write(&file_path, data)?;

    let conn = db.get()?;
    conn.execute(
        "INSERT INTO media (filename, original_name, mime_type, size_bytes, uploaded_by) VALUES (?, ?, ?, ?, ?)",
        (&filename, original_name, mime_type, data.len() as i64, uploaded_by),
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
        size_bytes: data.len() as i64,
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
    let filename: String =
        conn.query_row("SELECT filename FROM media WHERE id = ?", [id], |row| {
            row.get(0)
        })?;

    let file_path = upload_dir.join(&filename);
    if file_path.exists() {
        std::fs::remove_file(file_path)?;
    }

    conn.execute("DELETE FROM media WHERE id = ?", [id])?;
    Ok(())
}

pub fn update_media_alt(db: &Database, id: i64, alt_text: &str) -> Result<()> {
    let conn = db.get()?;
    conn.execute("UPDATE media SET alt_text = ? WHERE id = ?", (alt_text, id))?;
    Ok(())
}
