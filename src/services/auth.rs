use crate::models::{User, UserRole};
use crate::Database;
use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::Rng;

pub fn hash_password(password: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Password hashing failed: {}", e))?;
    Ok(hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed_hash = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn create_user(
    db: &Database,
    username: &str,
    email: &str,
    password: &str,
    role: UserRole,
) -> Result<i64> {
    let password_hash = hash_password(password)?;
    let conn = db.get()?;
    conn.execute(
        "INSERT INTO users (username, email, password_hash, role) VALUES (?, ?, ?, ?)",
        (username, email, &password_hash, role.to_string()),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_password(db: &Database, username: &str, password: &str) -> Result<()> {
    let password_hash = hash_password(password)?;
    let conn = db.get()?;
    conn.execute(
        "UPDATE users SET password_hash = ? WHERE username = ?",
        (&password_hash, username),
    )?;
    Ok(())
}

pub fn authenticate(db: &Database, username: &str, password: &str) -> Result<Option<User>> {
    let conn = db.get()?;
    let user: Option<User> = conn
        .query_row(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users WHERE username = ?",
            [username],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    email: row.get(2)?,
                    password_hash: row.get(3)?,
                    role: row.get::<_, String>(4)?.parse().unwrap_or(UserRole::Viewer),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .ok();

    match user {
        Some(u) if verify_password(password, &u.password_hash) => Ok(Some(u)),
        _ => Ok(None),
    }
}

pub fn create_session(db: &Database, user_id: i64, duration_days: i64) -> Result<String> {
    let token = generate_session_token();
    let conn = db.get()?;
    conn.execute(
        "INSERT INTO sessions (user_id, token, expires_at) VALUES (?, ?, datetime('now', ?||' days'))",
        (user_id, &token, duration_days),
    )?;
    Ok(token)
}

pub fn validate_session(db: &Database, token: &str) -> Result<Option<User>> {
    let conn = db.get()?;
    let user = conn
        .query_row(
            r#"
            SELECT u.id, u.username, u.email, u.password_hash, u.role, u.created_at, u.updated_at
            FROM users u
            JOIN sessions s ON s.user_id = u.id
            WHERE s.token = ? AND s.expires_at > datetime('now')
            "#,
            [token],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    email: row.get(2)?,
                    password_hash: row.get(3)?,
                    role: row.get::<_, String>(4)?.parse().unwrap_or(UserRole::Viewer),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .ok();
    Ok(user)
}

pub fn delete_session(db: &Database, token: &str) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM sessions WHERE token = ?", [token])?;
    Ok(())
}

pub fn cleanup_expired_sessions(db: &Database) -> Result<()> {
    let conn = db.get()?;
    conn.execute(
        "DELETE FROM sessions WHERE expires_at <= datetime('now')",
        [],
    )?;
    Ok(())
}

pub fn has_users(db: &Database) -> Result<bool> {
    let conn = db.get()?;
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    Ok(count > 0)
}

pub fn list_users(db: &Database) -> Result<Vec<User>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users ORDER BY created_at DESC",
    )?;
    let users = stmt
        .query_map([], |row| {
            Ok(User {
                id: row.get(0)?,
                username: row.get(1)?,
                email: row.get(2)?,
                password_hash: row.get(3)?,
                role: row.get::<_, String>(4)?.parse().unwrap_or(UserRole::Viewer),
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(users)
}

pub fn get_user(db: &Database, id: i64) -> Result<Option<User>> {
    let conn = db.get()?;
    let user = conn
        .query_row(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users WHERE id = ?",
            [id],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    username: row.get(1)?,
                    email: row.get(2)?,
                    password_hash: row.get(3)?,
                    role: row.get::<_, String>(4)?.parse().unwrap_or(UserRole::Viewer),
                    created_at: row.get(5)?,
                    updated_at: row.get(6)?,
                })
            },
        )
        .ok();
    Ok(user)
}

pub fn update_user(
    db: &Database,
    id: i64,
    email: Option<&str>,
    role: Option<UserRole>,
) -> Result<()> {
    let conn = db.get()?;
    if let Some(email) = email {
        conn.execute("UPDATE users SET email = ? WHERE id = ?", (email, id))?;
    }
    if let Some(role) = role {
        conn.execute(
            "UPDATE users SET role = ? WHERE id = ?",
            (role.to_string(), id),
        )?;
    }
    Ok(())
}

pub fn delete_user(db: &Database, id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM users WHERE id = ?", [id])?;
    Ok(())
}
