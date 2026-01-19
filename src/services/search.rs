use crate::models::ContentSummary;
use crate::Database;
use anyhow::Result;

pub fn search_content(db: &Database, query: &str, limit: usize) -> Result<Vec<ContentSummary>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT c.id, c.slug, c.title, c.excerpt, c.status, c.published_at, c.created_at
        FROM content c
        JOIN content_fts fts ON c.id = fts.rowid
        WHERE content_fts MATCH ? AND c.status = 'published'
        ORDER BY rank
        LIMIT ?
        "#,
    )?;

    let results = stmt
        .query_map((query, limit), |row| {
            Ok(ContentSummary {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                excerpt: row.get(3)?,
                status: row.get::<_, String>(4)?.parse().unwrap_or_default(),
                published_at: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}
