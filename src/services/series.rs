//! Content series service — ordered groups of posts (e.g., multi-part tutorials).

use crate::models::{Series, SeriesItem, SeriesNavItem, SeriesNavigation, SeriesWithItems};
use crate::services::slug::generate_slug;
use crate::Database;
use anyhow::{bail, Result};

pub fn create_series(
    db: &Database,
    title: &str,
    slug: Option<&str>,
    description: &str,
    status: &str,
) -> Result<i64> {
    if title.is_empty() {
        bail!("Series title cannot be empty");
    }
    let slug = slug
        .filter(|s| !s.is_empty())
        .map(String::from)
        .unwrap_or_else(|| generate_slug(title));
    let conn = db.get()?;
    conn.execute(
        "INSERT INTO content_series (title, slug, description, status) VALUES (?, ?, ?, ?)",
        (&title, &slug, description, status),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_series(
    db: &Database,
    id: i64,
    title: Option<&str>,
    slug: Option<&str>,
    description: Option<&str>,
    status: Option<&str>,
) -> Result<()> {
    let conn = db.get()?;

    let current = get_series_by_id(db, id)?
        .ok_or_else(|| anyhow::anyhow!("Series not found"))?;

    let title = title.unwrap_or(&current.title);
    let slug = slug.filter(|s| !s.is_empty()).unwrap_or(&current.slug);
    let description = description.unwrap_or(&current.description);
    let status = status.unwrap_or(&current.status);

    conn.execute(
        "UPDATE content_series SET title = ?, slug = ?, description = ?, status = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        (title, slug, description, status, id),
    )?;
    Ok(())
}

pub fn delete_series(db: &Database, id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM content_series WHERE id = ?", [id])?;
    Ok(())
}

pub fn get_series_by_id(db: &Database, id: i64) -> Result<Option<Series>> {
    let conn = db.get()?;
    let series = conn
        .query_row(
            "SELECT id, title, slug, description, status, created_at, updated_at FROM content_series WHERE id = ?",
            [id],
            row_to_series,
        )
        .ok();
    Ok(series)
}

pub fn get_series_by_slug(db: &Database, slug: &str) -> Result<Option<Series>> {
    let conn = db.get()?;
    let series = conn
        .query_row(
            "SELECT id, title, slug, description, status, created_at, updated_at FROM content_series WHERE slug = ?",
            [slug],
            row_to_series,
        )
        .ok();
    Ok(series)
}

pub fn list_series(db: &Database, limit: usize, offset: usize) -> Result<Vec<SeriesWithItems>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, description, status, created_at, updated_at FROM content_series ORDER BY updated_at DESC LIMIT ? OFFSET ?",
    )?;
    let series_list: Vec<Series> = stmt
        .query_map((limit, offset), row_to_series)?
        .filter_map(|r| r.ok())
        .collect();

    let mut result = Vec::new();
    for s in series_list {
        let items = list_series_items(db, s.id)?;
        result.push(SeriesWithItems { series: s, items });
    }
    Ok(result)
}

pub fn list_series_items(db: &Database, series_id: i64) -> Result<Vec<SeriesItem>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT si.id, si.content_id, si.position, c.title, c.slug, c.status
        FROM series_items si
        JOIN content c ON si.content_id = c.id
        WHERE si.series_id = ?
        ORDER BY si.position ASC
        "#,
    )?;
    let items = stmt
        .query_map([series_id], |row| {
            Ok(SeriesItem {
                id: row.get(0)?,
                content_id: row.get(1)?,
                position: row.get(2)?,
                title: row.get(3)?,
                slug: row.get(4)?,
                status: row.get(5)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(items)
}

pub fn add_item_to_series(db: &Database, series_id: i64, content_id: i64) -> Result<()> {
    let conn = db.get()?;
    // Get the next position
    let max_pos: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(position), -1) FROM series_items WHERE series_id = ?",
            [series_id],
            |row| row.get(0),
        )
        .unwrap_or(-1);
    conn.execute(
        "INSERT OR IGNORE INTO series_items (series_id, content_id, position) VALUES (?, ?, ?)",
        (series_id, content_id, max_pos + 1),
    )?;
    Ok(())
}

pub fn remove_item_from_series(db: &Database, series_id: i64, content_id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute(
        "DELETE FROM series_items WHERE series_id = ? AND content_id = ?",
        (series_id, content_id),
    )?;
    // Re-number positions to stay contiguous
    renumber_positions(db, series_id)?;
    Ok(())
}

pub fn reorder_series_items(db: &Database, series_id: i64, content_ids: &[i64]) -> Result<()> {
    let mut conn = db.get()?;
    let tx = conn.transaction()?;
    for (pos, content_id) in content_ids.iter().enumerate() {
        tx.execute(
            "UPDATE series_items SET position = ? WHERE series_id = ? AND content_id = ?",
            (pos as i32, series_id, content_id),
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Get series navigation context for a given content item (prev/next within series).
pub fn get_series_navigation(db: &Database, content_id: i64) -> Result<Option<SeriesNavigation>> {
    let conn = db.get()?;

    // Find the series this content belongs to (if any)
    let row: Option<(i64, i32)> = conn
        .query_row(
            "SELECT series_id, position FROM series_items WHERE content_id = ?",
            [content_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok();

    let (series_id, position) = match row {
        Some(r) => r,
        None => return Ok(None),
    };

    let series = get_series_by_id(db, series_id)?
        .ok_or_else(|| anyhow::anyhow!("Series not found"))?;

    // Only show navigation for published series
    if series.status != "published" {
        return Ok(None);
    }

    let items = list_series_items(db, series_id)?;
    let total_items = items.len();

    let prev = items
        .iter()
        .filter(|i| i.position < position && i.status == "published")
        .max_by_key(|i| i.position)
        .map(|i| SeriesNavItem {
            title: i.title.clone(),
            slug: i.slug.clone(),
            position: i.position,
        });

    let next = items
        .iter()
        .filter(|i| i.position > position && i.status == "published")
        .min_by_key(|i| i.position)
        .map(|i| SeriesNavItem {
            title: i.title.clone(),
            slug: i.slug.clone(),
            position: i.position,
        });

    Ok(Some(SeriesNavigation {
        series,
        current_position: position,
        total_items,
        prev,
        next,
    }))
}

/// Get all published series
pub fn list_published_series(db: &Database) -> Result<Vec<SeriesWithItems>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, title, slug, description, status, created_at, updated_at FROM content_series WHERE status = 'published' ORDER BY updated_at DESC",
    )?;
    let series_list: Vec<Series> = stmt
        .query_map([], row_to_series)?
        .filter_map(|r| r.ok())
        .collect();

    let mut result = Vec::new();
    for s in series_list {
        let items = list_series_items(db, s.id)?;
        result.push(SeriesWithItems { series: s, items });
    }
    Ok(result)
}

fn renumber_positions(db: &Database, series_id: i64) -> Result<()> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT content_id FROM series_items WHERE series_id = ? ORDER BY position ASC",
    )?;
    let ids: Vec<i64> = stmt
        .query_map([series_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    for (pos, content_id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE series_items SET position = ? WHERE series_id = ? AND content_id = ?",
            (pos as i32, series_id, content_id),
        )?;
    }
    Ok(())
}

fn row_to_series(row: &rusqlite::Row) -> rusqlite::Result<Series> {
    Ok(Series {
        id: row.get(0)?,
        title: row.get(1)?,
        slug: row.get(2)?,
        description: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::content;
    use crate::models::{ContentType, ContentStatus, CreateContent};
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn setup_test_db() -> Database {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let db = Database::open_memory(&format!("series_test_{}", id)).unwrap();
        db.migrate().unwrap();
        db
    }

    #[test]
    fn test_create_and_list_series() {
        let db = setup_test_db();
        let id = create_series(&db, "Rust Tutorial", None, "A multi-part series", "published").unwrap();
        assert!(id > 0);

        let series = get_series_by_id(&db, id).unwrap().unwrap();
        assert_eq!(series.title, "Rust Tutorial");
        assert_eq!(series.slug, "rust-tutorial");
        assert_eq!(series.status, "published");

        let all = list_series(&db, 50, 0).unwrap();
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_series_items_and_navigation() {
        let db = setup_test_db();
        let series_id = create_series(&db, "Build a CMS", None, "", "published").unwrap();

        // Create 3 published posts
        let mut post_ids = Vec::new();
        for i in 1..=3 {
            let id = content::create_content(
                &db,
                CreateContent {
                    title: format!("Part {}", i),
                    slug: Some(format!("part-{}", i)),
                    content_type: ContentType::Post,
                    body_markdown: format!("Content for part {}", i),
                    excerpt: None,
                    featured_image: None,
                    status: ContentStatus::Published,
                    scheduled_at: None,
                    tags: vec![],
                    metadata: None,
                },
                None,
                200,
            ).unwrap();
            post_ids.push(id);
        }

        // Add items to series
        for &pid in &post_ids {
            add_item_to_series(&db, series_id, pid).unwrap();
        }

        let items = list_series_items(&db, series_id).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].position, 0);
        assert_eq!(items[2].position, 2);

        // Test navigation for middle post
        let nav = get_series_navigation(&db, post_ids[1]).unwrap().unwrap();
        assert_eq!(nav.current_position, 1);
        assert!(nav.prev.is_some());
        assert!(nav.next.is_some());
        assert_eq!(nav.prev.unwrap().slug, "part-1");
        assert_eq!(nav.next.unwrap().slug, "part-3");

        // Test navigation for first post
        let nav = get_series_navigation(&db, post_ids[0]).unwrap().unwrap();
        assert!(nav.prev.is_none());
        assert!(nav.next.is_some());

        // Test navigation for last post
        let nav = get_series_navigation(&db, post_ids[2]).unwrap().unwrap();
        assert!(nav.prev.is_some());
        assert!(nav.next.is_none());
    }

    #[test]
    fn test_remove_and_reorder() {
        let db = setup_test_db();
        let series_id = create_series(&db, "Test Series", None, "", "draft").unwrap();

        // Create 3 posts
        let mut ids = Vec::new();
        for i in 1..=3 {
            let id = content::create_content(
                &db,
                CreateContent {
                    title: format!("Post {}", i),
                    slug: Some(format!("post-{}", i)),
                    content_type: ContentType::Post,
                    body_markdown: String::new(),
                    excerpt: None,
                    featured_image: None,
                    status: ContentStatus::Draft,
                    scheduled_at: None,
                    tags: vec![],
                    metadata: None,
                },
                None,
                200,
            ).unwrap();
            ids.push(id);
            add_item_to_series(&db, series_id, id).unwrap();
        }

        // Remove middle item
        remove_item_from_series(&db, series_id, ids[1]).unwrap();
        let items = list_series_items(&db, series_id).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].position, 0);
        assert_eq!(items[1].position, 1);

        // Reorder: swap the two remaining items
        reorder_series_items(&db, series_id, &[ids[2], ids[0]]).unwrap();
        let items = list_series_items(&db, series_id).unwrap();
        assert_eq!(items[0].content_id, ids[2]);
        assert_eq!(items[1].content_id, ids[0]);
    }
}
