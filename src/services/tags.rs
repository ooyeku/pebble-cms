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
