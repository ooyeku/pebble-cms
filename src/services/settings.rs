use crate::db::Database;
use anyhow::Result;
use rusqlite::OptionalExtension;

/// Get a setting value by key
pub fn get_setting(db: &Database, key: &str) -> Result<Option<String>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?")?;
    let result = stmt.query_row([key], |row| row.get(0)).optional()?;
    Ok(result)
}

/// Set a setting value (insert or update)
pub fn set_setting(db: &Database, key: &str, value: &str) -> Result<()> {
    let conn = db.get()?;
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, CURRENT_TIMESTAMP)
         ON CONFLICT(key) DO UPDATE SET value = ?2, updated_at = CURRENT_TIMESTAMP",
        [key, value],
    )?;
    Ok(())
}

fn escape_like_pattern(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Get multiple settings by prefix
pub fn get_settings_by_prefix(db: &Database, prefix: &str) -> Result<Vec<(String, String)>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare("SELECT key, value FROM settings WHERE key LIKE ? ESCAPE '\\'")?;
    let pattern = format!("{}%", escape_like_pattern(prefix));
    let rows = stmt.query_map([pattern], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    let mut result = Vec::new();
    for row in rows {
        result.push(row?);
    }
    Ok(result)
}

/// Delete a setting
pub fn delete_setting(db: &Database, key: &str) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM settings WHERE key = ?", [key])?;
    Ok(())
}

// Homepage setting keys
pub const HOMEPAGE_TITLE: &str = "homepage_title";
pub const HOMEPAGE_SUBTITLE: &str = "homepage_subtitle";
pub const HOMEPAGE_SHOW_PAGES: &str = "homepage_show_pages";
pub const HOMEPAGE_SHOW_POSTS: &str = "homepage_show_posts";
pub const HOMEPAGE_CUSTOM_CONTENT: &str = "homepage_custom_content";

/// Get homepage settings with defaults
pub fn get_homepage_settings(db: &Database) -> Result<HomepageSettings> {
    let title = get_setting(db, HOMEPAGE_TITLE)?.unwrap_or_default();
    let subtitle = get_setting(db, HOMEPAGE_SUBTITLE)?.unwrap_or_default();
    let show_pages = get_setting(db, HOMEPAGE_SHOW_PAGES)?
        .map(|v| v == "true")
        .unwrap_or(true);
    let show_posts = get_setting(db, HOMEPAGE_SHOW_POSTS)?
        .map(|v| v == "true")
        .unwrap_or(true);
    let custom_content = get_setting(db, HOMEPAGE_CUSTOM_CONTENT)?.unwrap_or_default();

    Ok(HomepageSettings {
        title,
        subtitle,
        show_pages,
        show_posts,
        custom_content,
    })
}

/// Save homepage settings
pub fn save_homepage_settings(db: &Database, settings: &HomepageSettings) -> Result<()> {
    set_setting(db, HOMEPAGE_TITLE, &settings.title)?;
    set_setting(db, HOMEPAGE_SUBTITLE, &settings.subtitle)?;
    set_setting(
        db,
        HOMEPAGE_SHOW_PAGES,
        if settings.show_pages { "true" } else { "false" },
    )?;
    set_setting(
        db,
        HOMEPAGE_SHOW_POSTS,
        if settings.show_posts { "true" } else { "false" },
    )?;
    set_setting(db, HOMEPAGE_CUSTOM_CONTENT, &settings.custom_content)?;
    Ok(())
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct HomepageSettings {
    pub title: String,
    pub subtitle: String,
    #[serde(default = "default_true")]
    pub show_pages: bool,
    #[serde(default = "default_true")]
    pub show_posts: bool,
    pub custom_content: String,
}

fn default_true() -> bool {
    true
}
