use crate::models::ApiToken;
use crate::Database;
use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;
use sha2::{Digest, Sha256};

const TOKEN_PREFIX: &str = "pb_";
const TOKEN_BYTE_LENGTH: usize = 32;

/// Generate a raw random token string with the `pb_` prefix.
fn generate_raw_token() -> String {
    let mut bytes = [0u8; TOKEN_BYTE_LENGTH];
    rand::thread_rng().fill(&mut bytes);
    format!("{}{}", TOKEN_PREFIX, URL_SAFE_NO_PAD.encode(bytes))
}

/// SHA-256 hash a raw token for storage.
fn hash_token(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    hex::encode(digest)
}

/// Extract the short prefix (first 8 chars after `pb_`) for display.
fn extract_prefix(raw: &str) -> String {
    let without_prefix = raw.strip_prefix(TOKEN_PREFIX).unwrap_or(raw);
    let end = without_prefix.len().min(8);
    format!("{}{}...", TOKEN_PREFIX, &without_prefix[..end])
}

/// Create a new API token. Returns the raw token string (shown once) and the stored record.
pub fn create_token(
    db: &Database,
    name: &str,
    permissions: &str,
    created_by: Option<i64>,
    expires_at: Option<&str>,
) -> Result<(String, ApiToken)> {
    let raw_token = generate_raw_token();
    let token_hash = hash_token(&raw_token);
    let prefix = extract_prefix(&raw_token);

    let conn = db.get()?;
    conn.execute(
        "INSERT INTO api_tokens (name, token_hash, prefix, permissions, created_by, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![name, token_hash, prefix, permissions, created_by, expires_at],
    )?;

    let id = conn.last_insert_rowid();
    let token = conn.query_row(
        "SELECT id, name, prefix, permissions, created_by, last_used_at, expires_at, created_at
         FROM api_tokens WHERE id = ?",
        [id],
        row_to_token,
    )?;

    Ok((raw_token, token))
}

/// Validate a raw token string. Returns the token record if valid and not expired.
pub fn validate_token(db: &Database, raw_token: &str) -> Result<Option<ApiToken>> {
    if !raw_token.starts_with(TOKEN_PREFIX) {
        return Ok(None);
    }

    let token_hash = hash_token(raw_token);
    let conn = db.get()?;

    let token = conn
        .query_row(
            "SELECT id, name, prefix, permissions, created_by, last_used_at, expires_at, created_at
             FROM api_tokens WHERE token_hash = ?",
            [&token_hash],
            row_to_token,
        )
        .ok();

    let token = match token {
        Some(t) => t,
        None => return Ok(None),
    };

    // Check expiry
    if let Some(ref expires) = token.expires_at {
        let now = chrono::Utc::now().to_rfc3339();
        if *expires < now {
            return Ok(None);
        }
    }

    // Update last_used_at
    conn.execute(
        "UPDATE api_tokens SET last_used_at = CURRENT_TIMESTAMP WHERE id = ?",
        [token.id],
    )?;

    Ok(Some(token))
}

/// List all API tokens (without exposing hashes).
pub fn list_tokens(db: &Database) -> Result<Vec<ApiToken>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, name, prefix, permissions, created_by, last_used_at, expires_at, created_at
         FROM api_tokens ORDER BY created_at DESC",
    )?;

    let tokens = stmt
        .query_map([], row_to_token)?
        .filter_map(|r| r.ok())
        .collect();

    Ok(tokens)
}

/// Revoke (delete) an API token by ID.
pub fn revoke_token(db: &Database, id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM api_tokens WHERE id = ?", [id])?;
    Ok(())
}

fn row_to_token(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApiToken> {
    Ok(ApiToken {
        id: row.get(0)?,
        name: row.get(1)?,
        prefix: row.get(2)?,
        permissions: row.get(3)?,
        created_by: row.get(4)?,
        last_used_at: row.get(5)?,
        expires_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}
