use crate::models::{Tag, TagWithCount};
use crate::services::slug::generate_slug;
use crate::Database;
use anyhow::Result;

pub fn create_tag(db: &Database, name: &str, slug: Option<&str>) -> Result<i64> {
    let slug = slug
        .map(String::from)
        .unwrap_or_else(|| generate_slug(name));
    let conn = db.get()?;
    conn.execute(
        "INSERT INTO tags (name, slug) VALUES (?, ?)",
        (&name, &slug),
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn get_tag_by_slug(db: &Database, slug: &str) -> Result<Option<Tag>> {
    let conn = db.get()?;
    let tag = conn
        .query_row(
            "SELECT id, name, slug, created_at FROM tags WHERE slug = ?",
            [slug],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slug: row.get(2)?,
                    created_at: row.get(3)?,
                })
            },
        )
        .ok();
    Ok(tag)
}

pub fn list_tags(db: &Database) -> Result<Vec<Tag>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare("SELECT id, name, slug, created_at FROM tags ORDER BY name")?;
    let tags = stmt
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub fn list_tags_with_counts(db: &Database) -> Result<Vec<TagWithCount>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        r#"
        SELECT t.id, t.name, t.slug, t.created_at, COUNT(ct.content_id) as count
        FROM tags t
        LEFT JOIN content_tags ct ON t.id = ct.tag_id
        LEFT JOIN content c ON ct.content_id = c.id AND c.status = 'published'
        GROUP BY t.id
        ORDER BY count DESC, t.name
        "#,
    )?;
    let tags = stmt
        .query_map([], |row| {
            Ok(TagWithCount {
                tag: Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slug: row.get(2)?,
                    created_at: row.get(3)?,
                },
                count: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub fn delete_tag(db: &Database, id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM tags WHERE id = ?", [id])?;
    Ok(())
}

pub fn cleanup_orphaned_tags(db: &Database) -> Result<usize> {
    let conn = db.get()?;
    let deleted = conn.execute(
        "DELETE FROM tags WHERE id NOT IN (SELECT DISTINCT tag_id FROM content_tags)",
        [],
    )?;
    Ok(deleted)
}

pub fn update_tag(db: &Database, id: i64, name: &str, slug: Option<&str>) -> Result<()> {
    let slug = slug
        .map(String::from)
        .unwrap_or_else(|| generate_slug(name));
    let conn = db.get()?;
    conn.execute(
        "UPDATE tags SET name = ?, slug = ? WHERE id = ?",
        (name, &slug, id),
    )?;
    Ok(())
}

pub fn get_posts_by_tag(
    db: &Database,
    tag_slug: &str,
) -> Result<Vec<crate::models::ContentWithTags>> {
    use crate::models::{Content, ContentStatus, ContentType, ContentWithTags, Tag, UserSummary};
    use std::collections::{HashMap, HashSet};

    let conn = db.get()?;

    // Fetch all content in a single query
    let mut stmt = conn.prepare(
        r#"
        SELECT c.id, c.slug, c.title, c.content_type, c.body_markdown, c.body_html,
               c.excerpt, c.featured_image, c.status, c.scheduled_at, c.published_at,
               c.author_id, c.metadata, c.created_at, c.updated_at
        FROM content c
        JOIN content_tags ct ON c.id = ct.content_id
        JOIN tags t ON ct.tag_id = t.id
        WHERE t.slug = ? AND c.status = 'published'
        ORDER BY c.published_at DESC, c.created_at DESC
        "#,
    )?;

    let contents: Vec<Content> = stmt
        .query_map([tag_slug], |row| {
            let raw_metadata: serde_json::Value =
                serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or(serde_json::json!({}));
            let metadata = crate::services::content::ensure_metadata_defaults(raw_metadata);

            Ok(Content {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                content_type: row
                    .get::<_, String>(3)?
                    .parse()
                    .unwrap_or(ContentType::Post),
                body_markdown: row.get(4)?,
                body_html: row.get(5)?,
                excerpt: row.get(6)?,
                featured_image: row.get(7)?,
                status: row
                    .get::<_, String>(8)?
                    .parse()
                    .unwrap_or(ContentStatus::Draft),
                scheduled_at: row.get(9)?,
                published_at: row.get(10)?,
                author_id: row.get(11)?,
                metadata,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    if contents.is_empty() {
        return Ok(vec![]);
    }

    // Batch fetch all tags for all content items
    let content_ids: Vec<i64> = contents.iter().map(|c| c.id).collect();
    let placeholders: String = content_ids
        .iter()
        .map(|_| "?")
        .collect::<Vec<_>>()
        .join(",");
    let tag_sql = format!(
        "SELECT ct.content_id, t.id, t.name, t.slug, t.created_at
         FROM tags t
         JOIN content_tags ct ON t.id = ct.tag_id
         WHERE ct.content_id IN ({})",
        placeholders
    );

    let mut tag_stmt = conn.prepare(&tag_sql)?;
    let params: Vec<&dyn rusqlite::ToSql> = content_ids
        .iter()
        .map(|id| id as &dyn rusqlite::ToSql)
        .collect();

    let mut tags_by_content: HashMap<i64, Vec<Tag>> = HashMap::new();
    let tag_rows = tag_stmt.query_map(params.as_slice(), |row| {
        Ok((
            row.get::<_, i64>(0)?,
            Tag {
                id: row.get(1)?,
                name: row.get(2)?,
                slug: row.get(3)?,
                created_at: row.get(4)?,
            },
        ))
    })?;

    for row in tag_rows {
        if let Ok((content_id, tag)) = row {
            tags_by_content.entry(content_id).or_default().push(tag);
        }
    }

    // Batch fetch all authors
    let author_ids: Vec<i64> = contents
        .iter()
        .filter_map(|c| c.author_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let mut authors_by_id: HashMap<i64, UserSummary> = HashMap::new();

    if !author_ids.is_empty() {
        let author_placeholders: String =
            author_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let author_sql = format!(
            "SELECT id, username FROM users WHERE id IN ({})",
            author_placeholders
        );
        let mut author_stmt = conn.prepare(&author_sql)?;
        let author_params: Vec<&dyn rusqlite::ToSql> = author_ids
            .iter()
            .map(|id| id as &dyn rusqlite::ToSql)
            .collect();

        let author_rows = author_stmt.query_map(author_params.as_slice(), |row| {
            Ok(UserSummary {
                id: row.get(0)?,
                username: row.get(1)?,
            })
        })?;

        for row in author_rows {
            if let Ok(author) = row {
                authors_by_id.insert(author.id, author);
            }
        }
    }

    // Build the enriched content
    let result: Vec<ContentWithTags> = contents
        .into_iter()
        .map(|content| {
            let tags = tags_by_content.remove(&content.id).unwrap_or_default();
            let author = content
                .author_id
                .and_then(|aid| authors_by_id.get(&aid).cloned());
            ContentWithTags {
                content,
                tags,
                author,
            }
        })
        .collect();

    Ok(result)
}
