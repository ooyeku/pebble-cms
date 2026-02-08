//! Content versioning service
//!
//! Provides functionality for tracking content revisions, viewing history,
//! comparing versions, and restoring previous versions.

use crate::Database;
use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

/// A full snapshot of content at a point in time
#[derive(Debug, Clone, Serialize)]
pub struct ContentVersion {
    pub id: i64,
    pub content_id: i64,
    pub version_number: i64,
    pub title: String,
    pub slug: String,
    pub body_markdown: String,
    pub excerpt: Option<String>,
    pub featured_image: Option<String>,
    pub metadata: serde_json::Value,
    pub tags: Vec<String>,
    pub created_by: Option<i64>,
    pub created_at: String,
}

/// Summary information for version history lists
#[derive(Debug, Clone, Serialize)]
pub struct VersionSummary {
    pub id: i64,
    pub version_number: i64,
    pub title: String,
    pub created_by_username: Option<String>,
    pub created_at: String,
    pub changes_summary: String,
}

/// Diff between two versions
#[derive(Debug, Clone, Serialize)]
pub struct VersionDiff {
    pub old_version: ContentVersion,
    pub new_version: ContentVersion,
    pub title_changed: bool,
    pub slug_changed: bool,
    pub excerpt_changed: bool,
    pub tags_changed: bool,
    pub body_diff: Vec<DiffLine>,
}

/// A single line in a diff
#[derive(Debug, Clone, Serialize)]
pub struct DiffLine {
    pub line_type: DiffLineType,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiffLineType {
    Same,
    Added,
    Removed,
}

/// Create a version snapshot of the current content state.
/// Call this BEFORE applying updates to preserve the previous state.
pub fn create_version(db: &Database, content_id: i64, user_id: Option<i64>) -> Result<i64> {
    let conn = db.get()?;

    // Fetch current content state
    let (title, slug, body_markdown, excerpt, featured_image, metadata): (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        String,
    ) = conn.query_row(
        "SELECT title, slug, body_markdown, excerpt, featured_image, metadata FROM content WHERE id = ?",
        [content_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    )?;

    // Fetch current tags
    let tags = get_content_tags(&conn, content_id)?;
    let tags_json = serde_json::to_string(&tags)?;

    // Get next version number
    let version_number = next_version_number(&conn, content_id)?;

    // Insert version
    conn.execute(
        r#"
        INSERT INTO content_versions
            (content_id, version_number, title, slug, body_markdown, excerpt, featured_image, metadata, tags_json, created_by)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
        "#,
        rusqlite::params![
            content_id,
            version_number,
            title,
            slug,
            body_markdown,
            excerpt,
            featured_image,
            metadata,
            tags_json,
            user_id,
        ],
    )?;

    let version_id = conn.last_insert_rowid();
    tracing::debug!(
        "Created version {} (v{}) for content {}",
        version_id,
        version_number,
        content_id
    );

    Ok(version_id)
}

/// List versions for a content item, newest first
pub fn list_versions(
    db: &Database,
    content_id: i64,
    limit: usize,
    offset: usize,
) -> Result<Vec<VersionSummary>> {
    let conn = db.get()?;

    let mut stmt = conn.prepare(
        r#"
        SELECT
            cv.id,
            cv.version_number,
            cv.title,
            u.username,
            cv.created_at,
            cv.body_markdown,
            cv.slug,
            cv.excerpt,
            cv.tags_json
        FROM content_versions cv
        LEFT JOIN users u ON cv.created_by = u.id
        WHERE cv.content_id = ?1
        ORDER BY cv.version_number DESC
        LIMIT ?2 OFFSET ?3
        "#,
    )?;

    let versions: Vec<VersionSummary> = stmt
        .query_map(rusqlite::params![content_id, limit, offset], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(
            |(id, version_number, title, username, created_at, body, _slug, _excerpt, _tags)| {
                let line_count = body.lines().count();
                let changes_summary = format!("{} lines", line_count);

                VersionSummary {
                    id,
                    version_number,
                    title,
                    created_by_username: username,
                    created_at,
                    changes_summary,
                }
            },
        )
        .collect();

    Ok(versions)
}

/// Get full version details by version ID
pub fn get_version(db: &Database, version_id: i64) -> Result<ContentVersion> {
    let conn = db.get()?;

    let version = conn.query_row(
        r#"
        SELECT id, content_id, version_number, title, slug, body_markdown,
               excerpt, featured_image, metadata, tags_json, created_by, created_at
        FROM content_versions
        WHERE id = ?1
        "#,
        [version_id],
        |row| {
            Ok(ContentVersion {
                id: row.get(0)?,
                content_id: row.get(1)?,
                version_number: row.get(2)?,
                title: row.get(3)?,
                slug: row.get(4)?,
                body_markdown: row.get(5)?,
                excerpt: row.get(6)?,
                featured_image: row.get(7)?,
                metadata: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                created_by: row.get(10)?,
                created_at: row.get(11)?,
            })
        },
    )?;

    Ok(version)
}

/// Get version by content ID and version number
pub fn get_version_by_number(
    db: &Database,
    content_id: i64,
    version_number: i64,
) -> Result<ContentVersion> {
    let conn = db.get()?;

    let version = conn.query_row(
        r#"
        SELECT id, content_id, version_number, title, slug, body_markdown,
               excerpt, featured_image, metadata, tags_json, created_by, created_at
        FROM content_versions
        WHERE content_id = ?1 AND version_number = ?2
        "#,
        [content_id, version_number],
        |row| {
            Ok(ContentVersion {
                id: row.get(0)?,
                content_id: row.get(1)?,
                version_number: row.get(2)?,
                title: row.get(3)?,
                slug: row.get(4)?,
                body_markdown: row.get(5)?,
                excerpt: row.get(6)?,
                featured_image: row.get(7)?,
                metadata: serde_json::from_str(&row.get::<_, String>(8)?).unwrap_or_default(),
                tags: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                created_by: row.get(10)?,
                created_at: row.get(11)?,
            })
        },
    )?;

    Ok(version)
}

/// Restore content to a previous version.
/// Creates a backup version of current state first, then applies the old version.
pub fn restore_version(
    db: &Database,
    content_id: i64,
    version_id: i64,
    user_id: Option<i64>,
) -> Result<()> {
    // First, create a backup of current state
    create_version(db, content_id, user_id)?;

    // Get the version to restore
    let version = get_version(db, version_id)?;

    if version.content_id != content_id {
        anyhow::bail!("Version does not belong to this content");
    }

    let mut conn = db.get()?;
    let tx = conn.transaction()?;

    // Update the content with the old version's data
    tx.execute(
        r#"
        UPDATE content
        SET title = ?1, slug = ?2, body_markdown = ?3, excerpt = ?4,
            featured_image = ?5, metadata = ?6
        WHERE id = ?7
        "#,
        rusqlite::params![
            version.title,
            version.slug,
            version.body_markdown,
            version.excerpt,
            version.featured_image,
            serde_json::to_string(&version.metadata)?,
            content_id,
        ],
    )?;

    // Re-render the markdown to HTML
    let renderer = crate::services::markdown::MarkdownRenderer::new();
    let body_html = renderer.render(&version.body_markdown);

    tx.execute(
        "UPDATE content SET body_html = ?1 WHERE id = ?2",
        rusqlite::params![body_html, content_id],
    )?;

    // Restore tags
    tx.execute(
        "DELETE FROM content_tags WHERE content_id = ?1",
        [content_id],
    )?;

    for tag_name in &version.tags {
        // Get or create tag
        let tag_id: i64 =
            match tx.query_row("SELECT id FROM tags WHERE name = ?1", [tag_name], |row| {
                row.get(0)
            }) {
                Ok(id) => id,
                Err(_) => {
                    let slug = crate::services::slug::generate_slug(tag_name);
                    tx.execute(
                        "INSERT INTO tags (name, slug) VALUES (?1, ?2)",
                        rusqlite::params![tag_name, slug],
                    )?;
                    tx.last_insert_rowid()
                }
            };

        tx.execute(
            "INSERT OR IGNORE INTO content_tags (content_id, tag_id) VALUES (?1, ?2)",
            [content_id, tag_id],
        )?;
    }

    tx.commit()?;

    tracing::info!(
        "Restored content {} to version {} (v{})",
        content_id,
        version_id,
        version.version_number
    );

    Ok(())
}

/// Generate a diff between two versions
pub fn diff_versions(
    db: &Database,
    old_version_id: i64,
    new_version_id: i64,
) -> Result<VersionDiff> {
    let old_version = get_version(db, old_version_id)?;
    let new_version = get_version(db, new_version_id)?;

    let title_changed = old_version.title != new_version.title;
    let slug_changed = old_version.slug != new_version.slug;
    let excerpt_changed = old_version.excerpt != new_version.excerpt;
    let tags_changed = old_version.tags != new_version.tags;

    let body_diff = compute_line_diff(&old_version.body_markdown, &new_version.body_markdown);

    Ok(VersionDiff {
        old_version,
        new_version,
        title_changed,
        slug_changed,
        excerpt_changed,
        tags_changed,
        body_diff,
    })
}

/// Clean up old versions, keeping only the most recent `keep_count` versions
pub fn cleanup_old_versions(db: &Database, content_id: i64, keep_count: usize) -> Result<usize> {
    if keep_count == 0 {
        return Ok(0); // Unlimited retention
    }

    let conn = db.get()?;

    // Get version IDs to delete (all except the most recent keep_count)
    let mut stmt = conn.prepare(
        r#"
        SELECT id FROM content_versions
        WHERE content_id = ?1
        ORDER BY version_number DESC
        LIMIT -1 OFFSET ?2
        "#,
    )?;

    let ids_to_delete: Vec<i64> = stmt
        .query_map(rusqlite::params![content_id, keep_count], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    if ids_to_delete.is_empty() {
        return Ok(0);
    }

    let deleted = ids_to_delete.len();

    for id in ids_to_delete {
        conn.execute("DELETE FROM content_versions WHERE id = ?1", [id])?;
    }

    tracing::debug!(
        "Cleaned up {} old versions for content {}",
        deleted,
        content_id
    );

    Ok(deleted)
}

/// Count total versions for a content item
pub fn count_versions(db: &Database, content_id: i64) -> Result<i64> {
    let conn = db.get()?;
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM content_versions WHERE content_id = ?1",
        [content_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

/// Get the latest version number for a content item
pub fn get_latest_version_number(db: &Database, content_id: i64) -> Result<Option<i64>> {
    let conn = db.get()?;
    let version: Option<i64> = conn
        .query_row(
            "SELECT MAX(version_number) FROM content_versions WHERE content_id = ?1",
            [content_id],
            |row| row.get(0),
        )
        .ok();
    Ok(version)
}

// Helper functions

fn next_version_number(conn: &Connection, content_id: i64) -> Result<i64> {
    let max: Option<i64> = conn
        .query_row(
            "SELECT MAX(version_number) FROM content_versions WHERE content_id = ?1",
            [content_id],
            |row| row.get(0),
        )
        .unwrap_or(None);

    Ok(max.unwrap_or(0) + 1)
}

fn get_content_tags(conn: &Connection, content_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tags t JOIN content_tags ct ON t.id = ct.tag_id WHERE ct.content_id = ?1",
    )?;

    let tags: Vec<String> = stmt
        .query_map([content_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(tags)
}

/// Compute a simple line-by-line diff between two texts
fn compute_line_diff(old_text: &str, new_text: &str) -> Vec<DiffLine> {
    let old_lines: Vec<&str> = old_text.lines().collect();
    let new_lines: Vec<&str> = new_text.lines().collect();

    let m = old_lines.len();
    let n = new_lines.len();

    // Build LCS table
    let mut lcs = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old_lines[i - 1] == new_lines[j - 1] {
                lcs[i][j] = lcs[i - 1][j - 1] + 1;
            } else {
                lcs[i][j] = lcs[i - 1][j].max(lcs[i][j - 1]);
            }
        }
    }

    // Backtrack to build diff
    let mut i = m;
    let mut j = n;
    let mut result = Vec::new();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_lines[i - 1] == new_lines[j - 1] {
            result.push(DiffLine {
                line_type: DiffLineType::Same,
                content: old_lines[i - 1].to_string(),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || lcs[i][j - 1] >= lcs[i - 1][j]) {
            result.push(DiffLine {
                line_type: DiffLineType::Added,
                content: new_lines[j - 1].to_string(),
            });
            j -= 1;
        } else {
            result.push(DiffLine {
                line_type: DiffLineType::Removed,
                content: old_lines[i - 1].to_string(),
            });
            i -= 1;
        }
    }

    result.reverse();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_line_diff_no_changes() {
        let text = "line 1\nline 2\nline 3";
        let diff = compute_line_diff(text, text);

        assert_eq!(diff.len(), 3);
        assert!(diff.iter().all(|d| d.line_type == DiffLineType::Same));
    }

    #[test]
    fn test_compute_line_diff_added_line() {
        let old = "line 1\nline 2";
        let new = "line 1\nline 2\nline 3";
        let diff = compute_line_diff(old, new);

        assert_eq!(diff.len(), 3);
        assert_eq!(diff[0].line_type, DiffLineType::Same);
        assert_eq!(diff[1].line_type, DiffLineType::Same);
        assert_eq!(diff[2].line_type, DiffLineType::Added);
        assert_eq!(diff[2].content, "line 3");
    }

    #[test]
    fn test_compute_line_diff_removed_line() {
        let old = "line 1\nline 2\nline 3";
        let new = "line 1\nline 3";
        let diff = compute_line_diff(old, new);

        let removed_count = diff
            .iter()
            .filter(|d| d.line_type == DiffLineType::Removed)
            .count();
        assert_eq!(removed_count, 1);
    }

    #[test]
    fn test_compute_line_diff_modified_line() {
        let old = "hello world";
        let new = "hello rust";
        let diff = compute_line_diff(old, new);

        // Should show old removed and new added
        assert!(diff
            .iter()
            .any(|d| d.line_type == DiffLineType::Removed && d.content == "hello world"));
        assert!(diff
            .iter()
            .any(|d| d.line_type == DiffLineType::Added && d.content == "hello rust"));
    }
}
