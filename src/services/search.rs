use crate::models::ContentSummary;
use crate::Database;
use anyhow::Result;

pub fn search_content(db: &Database, query: &str, limit: usize) -> Result<Vec<ContentSummary>> {
    let conn = db.get()?;

    let fts_query = build_fts_query(query);

    let mut stmt = conn.prepare(
        r#"
        SELECT c.id, c.slug, c.title, c.content_type, c.excerpt, c.status, c.published_at, c.created_at
        FROM content c
        JOIN content_fts fts ON c.id = fts.rowid
        WHERE content_fts MATCH ? AND c.status = 'published'
        ORDER BY rank
        LIMIT ?
        "#,
    )?;

    let results = stmt
        .query_map((&fts_query, limit), |row| {
            Ok(ContentSummary {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                content_type: row.get::<_, String>(3)?.parse().unwrap_or_default(),
                excerpt: row.get(4)?,
                status: row.get::<_, String>(5)?.parse().unwrap_or_default(),
                scheduled_at: None,
                published_at: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}

pub fn build_fts_query(query: &str) -> String {
    let terms: Vec<&str> = query.split_whitespace().collect();
    if terms.is_empty() {
        return String::new();
    }
    terms
        .iter()
        .map(|t| format!("\"{}\"*", t.replace('"', "")))
        .collect::<Vec<_>>()
        .join(" OR ")
}

pub fn rebuild_fts_index(db: &Database) -> Result<usize> {
    let conn = db.get()?;

    conn.execute("DELETE FROM content_fts", [])?;

    let count = conn.execute(
        r#"
        INSERT INTO content_fts(rowid, title, body, tags)
        SELECT c.id, c.title, c.body_markdown,
               COALESCE((SELECT GROUP_CONCAT(t.name, ' ') FROM tags t
                         JOIN content_tags ct ON t.id = ct.tag_id
                         WHERE ct.content_id = c.id), '')
        FROM content c
        "#,
        [],
    )?;

    Ok(count)
}
