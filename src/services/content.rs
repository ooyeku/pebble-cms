use crate::models::{
    Content, ContentStatus, ContentSummary, ContentType, ContentWithTags, CreateContent, Tag,
    UpdateContent, UserSummary,
};
use crate::services::markdown::MarkdownRenderer;
use crate::services::slug::{generate_slug, validate_slug};
use crate::Database;
use anyhow::{bail, Result};

pub fn create_content(
    db: &Database,
    input: CreateContent,
    author_id: Option<i64>,
    excerpt_length: usize,
) -> Result<i64> {
    let renderer = MarkdownRenderer::new();
    let slug = input.slug.unwrap_or_else(|| generate_slug(&input.title));

    if !validate_slug(&slug) {
        bail!(
            "Invalid slug: must be 1-200 characters, lowercase letters, numbers, and hyphens only"
        );
    }

    let body_html = renderer.render(&input.body_markdown);
    let excerpt = input.excerpt.or_else(|| {
        if input.body_markdown.is_empty() {
            None
        } else {
            Some(renderer.generate_excerpt(&input.body_markdown, excerpt_length))
        }
    });

    let published_at = if input.status == ContentStatus::Published {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        None
    };

    let scheduled_at = if input.status == ContentStatus::Scheduled {
        input.scheduled_at.clone()
    } else {
        None
    };

    // Calculate reading time and merge with provided metadata
    let reading_time = renderer.calculate_reading_time(&input.body_markdown);
    let mut metadata = input.metadata.unwrap_or(serde_json::json!({}));
    metadata["reading_time_minutes"] = serde_json::json!(reading_time);

    let conn = db.get()?;
    conn.execute(
        r#"
        INSERT INTO content (slug, title, content_type, body_markdown, body_html, excerpt, featured_image, status, scheduled_at, published_at, author_id, metadata)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        (
            &slug,
            &input.title,
            input.content_type.to_string(),
            &input.body_markdown,
            &body_html,
            &excerpt,
            &input.featured_image,
            input.status.to_string(),
            &scheduled_at,
            &published_at,
            author_id,
            serde_json::to_string(&metadata)?,
        ),
    )?;

    let content_id = conn.last_insert_rowid();

    for tag_name in input.tags {
        let tag_slug = generate_slug(&tag_name);
        conn.execute(
            "INSERT OR IGNORE INTO tags (name, slug) VALUES (?, ?)",
            (&tag_name, &tag_slug),
        )?;
        conn.execute(
            "INSERT OR IGNORE INTO content_tags (content_id, tag_id) SELECT ?, id FROM tags WHERE slug = ?",
            (content_id, &tag_slug),
        )?;
    }

    Ok(content_id)
}

pub fn update_content(
    db: &Database,
    id: i64,
    input: UpdateContent,
    excerpt_length: usize,
) -> Result<()> {
    let renderer = MarkdownRenderer::new();
    let conn = db.get()?;

    let current: Content = conn.query_row(
        "SELECT id, slug, title, content_type, body_markdown, body_html, excerpt, featured_image, status, scheduled_at, published_at, author_id, metadata, created_at, updated_at FROM content WHERE id = ?",
        [id],
        row_to_content,
    )?;

    let title = input.title.unwrap_or(current.title);
    let slug = input.slug.unwrap_or(current.slug);

    if !validate_slug(&slug) {
        bail!(
            "Invalid slug: must be 1-200 characters, lowercase letters, numbers, and hyphens only"
        );
    }
    let body_markdown = input.body_markdown.unwrap_or(current.body_markdown);
    let body_html = renderer.render(&body_markdown);
    let excerpt = input
        .excerpt
        .or_else(|| Some(renderer.generate_excerpt(&body_markdown, excerpt_length)));
    let featured_image = input.featured_image.or(current.featured_image);
    let status = input.status.unwrap_or(current.status);

    // Calculate reading time and merge with provided metadata
    let reading_time = renderer.calculate_reading_time(&body_markdown);
    let mut metadata = input.metadata.unwrap_or(current.metadata);
    metadata["reading_time_minutes"] = serde_json::json!(reading_time);

    let published_at = if status == ContentStatus::Published && current.published_at.is_none() {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        current.published_at
    };

    let scheduled_at = if status == ContentStatus::Scheduled {
        input.scheduled_at.or(current.scheduled_at)
    } else {
        None
    };

    conn.execute(
        r#"
        UPDATE content SET slug = ?, title = ?, body_markdown = ?, body_html = ?, excerpt = ?, featured_image = ?, status = ?, scheduled_at = ?, published_at = ?, metadata = ?
        WHERE id = ?
        "#,
        (
            &slug,
            &title,
            &body_markdown,
            &body_html,
            &excerpt,
            &featured_image,
            status.to_string(),
            &scheduled_at,
            &published_at,
            serde_json::to_string(&metadata)?,
            id,
        ),
    )?;

    if let Some(tags) = input.tags {
        conn.execute("DELETE FROM content_tags WHERE content_id = ?", [id])?;
        for tag_name in tags {
            let tag_slug = generate_slug(&tag_name);
            conn.execute(
                "INSERT OR IGNORE INTO tags (name, slug) VALUES (?, ?)",
                (&tag_name, &tag_slug),
            )?;
            conn.execute(
                "INSERT OR IGNORE INTO content_tags (content_id, tag_id) SELECT ?, id FROM tags WHERE slug = ?",
                (id, &tag_slug),
            )?;
        }
    }

    Ok(())
}

pub fn delete_content(db: &Database, id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM content WHERE id = ?", [id])?;
    Ok(())
}

pub fn get_content_by_id(db: &Database, id: i64) -> Result<Option<ContentWithTags>> {
    let conn = db.get()?;
    let content: Option<Content> = conn
        .query_row(
            "SELECT id, slug, title, content_type, body_markdown, body_html, excerpt, featured_image, status, scheduled_at, published_at, author_id, metadata, created_at, updated_at FROM content WHERE id = ?",
            [id],
            row_to_content,
        )
        .ok();

    match content {
        Some(c) => Ok(Some(enrich_content(db, c)?)),
        None => Ok(None),
    }
}

pub fn get_content_by_slug(db: &Database, slug: &str) -> Result<Option<ContentWithTags>> {
    let conn = db.get()?;
    let content: Option<Content> = conn
        .query_row(
            "SELECT id, slug, title, content_type, body_markdown, body_html, excerpt, featured_image, status, scheduled_at, published_at, author_id, metadata, created_at, updated_at FROM content WHERE slug = ?",
            [slug],
            row_to_content,
        )
        .ok();

    match content {
        Some(c) => Ok(Some(enrich_content(db, c)?)),
        None => Ok(None),
    }
}

pub fn list_content(
    db: &Database,
    content_type: Option<ContentType>,
    status: Option<ContentStatus>,
    limit: usize,
    offset: usize,
) -> Result<Vec<ContentSummary>> {
    let conn = db.get()?;

    let mut sql = String::from(
        "SELECT id, slug, title, content_type, excerpt, status, scheduled_at, published_at, created_at FROM content WHERE 1=1",
    );
    let mut params: Vec<String> = Vec::new();

    if let Some(ct) = content_type {
        sql.push_str(" AND content_type = ?");
        params.push(ct.to_string());
    }
    if let Some(s) = status {
        sql.push_str(" AND status = ?");
        params.push(s.to_string());
    }

    sql.push_str(" ORDER BY created_at DESC LIMIT ? OFFSET ?");

    let mut stmt = conn.prepare(&sql)?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = params
        .iter()
        .map(|s| s as &dyn rusqlite::ToSql)
        .chain(std::iter::once(&limit as &dyn rusqlite::ToSql))
        .chain(std::iter::once(&offset as &dyn rusqlite::ToSql))
        .collect();

    let content = stmt
        .query_map(param_refs.as_slice(), |row| {
            Ok(ContentSummary {
                id: row.get(0)?,
                slug: row.get(1)?,
                title: row.get(2)?,
                content_type: row
                    .get::<_, String>(3)?
                    .parse()
                    .unwrap_or(ContentType::Post),
                excerpt: row.get(4)?,
                status: row
                    .get::<_, String>(5)?
                    .parse()
                    .unwrap_or(ContentStatus::Draft),
                scheduled_at: row.get(6)?,
                published_at: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(content)
}

pub fn list_published_content(
    db: &Database,
    content_type: ContentType,
    limit: usize,
    offset: usize,
) -> Result<Vec<ContentWithTags>> {
    let conn = db.get()?;
    let mut stmt = conn.prepare(
        "SELECT id, slug, title, content_type, body_markdown, body_html, excerpt, featured_image, status, scheduled_at, published_at, author_id, metadata, created_at, updated_at
         FROM content WHERE content_type = ? AND status = 'published' ORDER BY published_at DESC LIMIT ? OFFSET ?",
    )?;

    let content = stmt
        .query_map((content_type.to_string(), limit, offset), row_to_content)?
        .collect::<Result<Vec<_>, _>>()?;

    content.into_iter().map(|c| enrich_content(db, c)).collect()
}

pub fn count_content(
    db: &Database,
    content_type: Option<ContentType>,
    status: Option<ContentStatus>,
) -> Result<i64> {
    let conn = db.get()?;
    let mut sql = String::from("SELECT COUNT(*) FROM content WHERE 1=1");
    let mut params: Vec<String> = Vec::new();

    if let Some(ct) = content_type {
        sql.push_str(" AND content_type = ?");
        params.push(ct.to_string());
    }
    if let Some(s) = status {
        sql.push_str(" AND status = ?");
        params.push(s.to_string());
    }

    let param_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    let count: i64 = conn.query_row(&sql, param_refs.as_slice(), |row| row.get(0))?;
    Ok(count)
}

fn ensure_metadata_defaults(mut metadata: serde_json::Value) -> serde_json::Value {
    // Ensure custom code fields have default values for template compatibility
    if metadata.get("use_custom_code").is_none() {
        metadata["use_custom_code"] = serde_json::json!("");
    }
    if metadata.get("custom_html").is_none() {
        metadata["custom_html"] = serde_json::json!("");
    }
    if metadata.get("custom_css").is_none() {
        metadata["custom_css"] = serde_json::json!("");
    }
    if metadata.get("custom_js").is_none() {
        metadata["custom_js"] = serde_json::json!("");
    }
    metadata
}

fn row_to_content(row: &rusqlite::Row) -> rusqlite::Result<Content> {
    let raw_metadata: serde_json::Value =
        serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or(serde_json::json!({}));
    let metadata = ensure_metadata_defaults(raw_metadata);

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
}

fn enrich_content(db: &Database, content: Content) -> Result<ContentWithTags> {
    let conn = db.get()?;

    let mut tag_stmt = conn.prepare(
        "SELECT t.id, t.name, t.slug, t.created_at FROM tags t JOIN content_tags ct ON t.id = ct.tag_id WHERE ct.content_id = ?",
    )?;
    let tags: Vec<Tag> = tag_stmt
        .query_map([content.id], |row| {
            Ok(Tag {
                id: row.get(0)?,
                name: row.get(1)?,
                slug: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let author = content.author_id.and_then(|aid| {
        conn.query_row(
            "SELECT id, username FROM users WHERE id = ?",
            [aid],
            |row| {
                Ok(UserSummary {
                    id: row.get(0)?,
                    username: row.get(1)?,
                })
            },
        )
        .ok()
    });

    Ok(ContentWithTags {
        content,
        tags,
        author,
    })
}

pub fn publish_scheduled(db: &Database) -> Result<usize> {
    let conn = db.get()?;
    let now = chrono::Utc::now().to_rfc3339();

    // Find all scheduled content that should be published
    let mut stmt = conn.prepare(
        "SELECT id FROM content WHERE status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= ?"
    )?;

    let ids: Vec<i64> = stmt
        .query_map([&now], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    if ids.is_empty() {
        return Ok(0);
    }

    // Update each post to published
    for id in &ids {
        conn.execute(
            "UPDATE content SET status = 'published', published_at = ?, scheduled_at = NULL WHERE id = ?",
            (&now, id),
        )?;
        tracing::info!("Auto-published scheduled content id={}", id);
    }

    Ok(ids.len())
}
