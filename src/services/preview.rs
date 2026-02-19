//! Draft preview service — generates signed, time-limited preview tokens
//! so authors can share links to unpublished content.

use crate::Database;
use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;

/// Duration in seconds that a preview token remains valid (1 hour).
const PREVIEW_TOKEN_LIFETIME_SECS: i64 = 3600;

/// Generate a signed preview token for a content item.
/// The token encodes the content ID and an expiration time.
pub fn generate_preview_token(db: &Database, content_id: i64) -> Result<String> {
    let mut random_bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut random_bytes);
    let token = URL_SAFE_NO_PAD.encode(random_bytes);

    let conn = db.get()?;
    conn.execute(
        "INSERT INTO preview_tokens (token, content_id, expires_at) VALUES (?, ?, datetime('now', ?||' seconds'))",
        (&token, content_id, PREVIEW_TOKEN_LIFETIME_SECS),
    )?;

    Ok(token)
}

/// Validate a preview token and return the associated content_id if valid.
pub fn validate_preview_token(db: &Database, token: &str) -> Result<Option<i64>> {
    let conn = db.get()?;
    let content_id: Option<i64> = conn
        .query_row(
            "SELECT content_id FROM preview_tokens WHERE token = ? AND expires_at > datetime('now')",
            [token],
            |row| row.get(0),
        )
        .ok();
    Ok(content_id)
}

/// Remove expired preview tokens.
pub fn cleanup_expired_tokens(db: &Database) -> Result<usize> {
    let conn = db.get()?;
    let count = conn.execute(
        "DELETE FROM preview_tokens WHERE expires_at <= datetime('now')",
        [],
    )?;
    Ok(count)
}
