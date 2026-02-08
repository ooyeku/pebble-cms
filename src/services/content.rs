use crate::models::{
    Content, ContentStatus, ContentSummary, ContentType, ContentWithTags, CreateContent, Tag,
    UpdateContent, UserSummary,
};
use crate::services::markdown::MarkdownRenderer;
use crate::services::slug::{generate_slug, validate_slug};
use crate::Database;
use anyhow::{bail, Result};

const MAX_TITLE_LENGTH: usize = 500;
const MAX_BODY_LENGTH: usize = 500_000;
const MAX_EXCERPT_LENGTH: usize = 2000;

fn validate_content_input(title: &str, body: &str, excerpt: Option<&str>) -> Result<()> {
    if title.is_empty() {
        bail!("Title cannot be empty");
    }
    if title.len() > MAX_TITLE_LENGTH {
        bail!("Title must be {} characters or less", MAX_TITLE_LENGTH);
    }
    if body.len() > MAX_BODY_LENGTH {
        bail!(
            "Content body must be {} characters or less",
            MAX_BODY_LENGTH
        );
    }
    if let Some(exc) = excerpt {
        if exc.len() > MAX_EXCERPT_LENGTH {
            bail!("Excerpt must be {} characters or less", MAX_EXCERPT_LENGTH);
        }
    }
    Ok(())
}

pub fn create_content(
    db: &Database,
    input: CreateContent,
    author_id: Option<i64>,
    excerpt_length: usize,
) -> Result<i64> {
    validate_content_input(&input.title, &input.body_markdown, input.excerpt.as_deref())?;

    let renderer = MarkdownRenderer::new();
    let slug = input.slug.unwrap_or_else(|| generate_slug(&input.title));

    if !validate_slug(&slug) {
        bail!(
            "Invalid slug: must be 1-200 characters, lowercase letters, numbers, and hyphens only"
        );
    }

    let mut conn = db.get()?;
    let tx = conn.transaction()?;

    // Check for slug uniqueness within transaction to prevent race conditions
    let existing: Option<i64> = tx
        .query_row("SELECT id FROM content WHERE slug = ?", [&slug], |row| {
            row.get(0)
        })
        .ok();
    if existing.is_some() {
        bail!("A post or page with the slug '{}' already exists", slug);
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
        match &input.scheduled_at {
            Some(dt) if !dt.is_empty() => {
                // Validate the timestamp format and ensure it's in the future
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(dt) {
                    if parsed <= chrono::Utc::now() {
                        bail!("Scheduled time must be in the future");
                    }
                    Some(dt.clone())
                } else if let Ok(parsed) =
                    chrono::NaiveDateTime::parse_from_str(dt, "%Y-%m-%dT%H:%M")
                {
                    // Handle datetime-local format from HTML forms
                    let utc_time = parsed.and_utc();
                    if utc_time <= chrono::Utc::now() {
                        bail!("Scheduled time must be in the future");
                    }
                    Some(utc_time.to_rfc3339())
                } else {
                    bail!("Invalid scheduled_at timestamp format. Use ISO 8601 format (e.g., 2024-01-15T10:30:00Z)");
                }
            }
            _ => bail!("Scheduled status requires a scheduled_at timestamp"),
        }
    } else {
        None
    };

    // Calculate reading time and merge with provided metadata
    let reading_time = renderer.calculate_reading_time(&input.body_markdown);
    let mut metadata = input.metadata.unwrap_or(serde_json::json!({}));
    metadata["reading_time_minutes"] = serde_json::json!(reading_time);

    tx.execute(
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

    let content_id = tx.last_insert_rowid();

    for tag_name in input.tags {
        let tag_slug = generate_slug(&tag_name);
        tx.execute(
            "INSERT OR IGNORE INTO tags (name, slug) VALUES (?, ?)",
            (&tag_name, &tag_slug),
        )?;
        tx.execute(
            "INSERT OR IGNORE INTO content_tags (content_id, tag_id) SELECT ?, id FROM tags WHERE slug = ?",
            (content_id, &tag_slug),
        )?;
    }

    tx.commit()?;
    Ok(content_id)
}

pub fn update_content(
    db: &Database,
    id: i64,
    input: UpdateContent,
    _excerpt_length: usize, // Preserved for API compatibility; excerpt is now only updated when explicitly provided
    user_id: Option<i64>,
    version_retention: usize,
) -> Result<()> {
    // Create a version snapshot BEFORE applying changes
    if let Err(e) = super::versions::create_version(db, id, user_id) {
        tracing::warn!("Failed to create version snapshot: {}", e);
        // Continue with update even if versioning fails
    }

    let renderer = MarkdownRenderer::new();
    let mut conn = db.get()?;

    let current: Content = conn.query_row(
        "SELECT id, slug, title, content_type, body_markdown, body_html, excerpt, featured_image, status, scheduled_at, published_at, author_id, metadata, created_at, updated_at FROM content WHERE id = ?",
        [id],
        row_to_content,
    )?;

    let title = input.title.unwrap_or(current.title);
    let original_slug = current.slug.clone();
    let slug = input.slug.unwrap_or(current.slug);
    let body_markdown = input.body_markdown.unwrap_or(current.body_markdown);

    validate_content_input(&title, &body_markdown, input.excerpt.as_deref())?;

    if !validate_slug(&slug) {
        bail!(
            "Invalid slug: must be 1-200 characters, lowercase letters, numbers, and hyphens only"
        );
    }

    // Check for slug uniqueness (excluding current content) for better error messages
    if slug != original_slug {
        let existing: Option<i64> = conn
            .query_row("SELECT id FROM content WHERE slug = ?", [&slug], |row| {
                row.get(0)
            })
            .ok();
        if existing.is_some() {
            bail!("A post or page with the slug '{}' already exists", slug);
        }
    }

    let body_html = renderer.render(&body_markdown);
    // Only regenerate excerpt if explicitly provided in input, otherwise keep current
    let excerpt = match input.excerpt {
        Some(new_excerpt) => Some(new_excerpt),
        None => current.excerpt, // Preserve existing excerpt
    };
    let featured_image = input.featured_image.or(current.featured_image);
    let status = input.status.unwrap_or(current.status);

    // Calculate reading time and merge with provided metadata
    let reading_time = renderer.calculate_reading_time(&body_markdown);
    let mut metadata = current.metadata;
    if let Some(input_meta) = input.metadata {
        if let (Some(base), Some(updates)) = (metadata.as_object_mut(), input_meta.as_object()) {
            for (key, value) in updates {
                base.insert(key.clone(), value.clone());
            }
        }
    }
    metadata["reading_time_minutes"] = serde_json::json!(reading_time);

    let published_at = if status == ContentStatus::Published && current.published_at.is_none() {
        Some(chrono::Utc::now().to_rfc3339())
    } else {
        current.published_at
    };

    let scheduled_at = if status == ContentStatus::Scheduled {
        match input.scheduled_at.or(current.scheduled_at) {
            Some(dt) if !dt.is_empty() => {
                // Validate the timestamp format and ensure it's in the future
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&dt) {
                    if parsed <= chrono::Utc::now() {
                        bail!("Scheduled time must be in the future");
                    }
                    Some(dt)
                } else if let Ok(parsed) =
                    chrono::NaiveDateTime::parse_from_str(&dt, "%Y-%m-%dT%H:%M")
                {
                    // Handle datetime-local format from HTML forms
                    let utc_time = parsed.and_utc();
                    if utc_time <= chrono::Utc::now() {
                        bail!("Scheduled time must be in the future");
                    }
                    Some(utc_time.to_rfc3339())
                } else {
                    bail!("Invalid scheduled_at timestamp format. Use ISO 8601 format (e.g., 2024-01-15T10:30:00Z)");
                }
            }
            _ => bail!("Scheduled status requires a scheduled_at timestamp"),
        }
    } else {
        None
    };

    let tx = conn.transaction()?;

    tx.execute(
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
        tx.execute("DELETE FROM content_tags WHERE content_id = ?", [id])?;
        for tag_name in tags {
            let tag_slug = generate_slug(&tag_name);
            tx.execute(
                "INSERT OR IGNORE INTO tags (name, slug) VALUES (?, ?)",
                (&tag_name, &tag_slug),
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO content_tags (content_id, tag_id) SELECT ?, id FROM tags WHERE slug = ?",
                (id, &tag_slug),
            )?;
        }
    }

    tx.commit()?;

    // Cleanup old versions based on retention policy
    if version_retention > 0 {
        if let Err(e) = super::versions::cleanup_old_versions(db, id, version_retention) {
            tracing::warn!("Failed to cleanup old versions: {}", e);
        }
    }

    Ok(())
}

pub fn delete_content(db: &Database, id: i64) -> Result<()> {
    let conn = db.get()?;
    conn.execute("DELETE FROM content WHERE id = ?", [id])?;
    let _ = crate::services::tags::cleanup_orphaned_tags(db);
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

    enrich_content_batch(db, content)
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

pub fn ensure_metadata_defaults(mut metadata: serde_json::Value) -> serde_json::Value {
    // Ensure custom code fields have default values for template compatibility
    // use_custom_code: "none" (default), "only" (custom code only), "both" (markdown + custom)
    if metadata.get("use_custom_code").is_none() {
        metadata["use_custom_code"] = serde_json::json!("none");
    } else if let Some(val) = metadata.get("use_custom_code") {
        // Normalize empty string to "none" for consistency
        if val.as_str() == Some("") {
            metadata["use_custom_code"] = serde_json::json!("none");
        }
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

/// Batch enrich multiple content items to avoid N+1 queries.
/// Fetches all tags and authors in bulk queries instead of per-item.
fn enrich_content_batch(db: &Database, contents: Vec<Content>) -> Result<Vec<ContentWithTags>> {
    if contents.is_empty() {
        return Ok(vec![]);
    }

    let conn = db.get()?;

    // Collect all content IDs and author IDs
    let content_ids: Vec<i64> = contents.iter().map(|c| c.id).collect();
    let author_ids: Vec<i64> = contents
        .iter()
        .filter_map(|c| c.author_id)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    // Batch fetch all tags for all content items
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

    let mut tags_by_content: std::collections::HashMap<i64, Vec<Tag>> =
        std::collections::HashMap::new();

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
        let (content_id, tag) = row?;
        tags_by_content.entry(content_id).or_default().push(tag);
    }

    // Batch fetch all authors
    let mut authors_by_id: std::collections::HashMap<i64, UserSummary> =
        std::collections::HashMap::new();

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
            let author = row?;
            authors_by_id.insert(author.id, author);
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

pub fn publish_scheduled(db: &Database) -> Result<usize> {
    let mut conn = db.get()?;
    let now = chrono::Utc::now().to_rfc3339();

    let tx = conn.transaction()?;

    let mut stmt = tx.prepare(
        "SELECT id FROM content WHERE status = 'scheduled' AND scheduled_at IS NOT NULL AND scheduled_at <= ?"
    )?;
    let ids: Vec<i64> = stmt
        .query_map([&now], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    if ids.is_empty() {
        return Ok(0);
    }

    for id in &ids {
        tx.execute(
            "UPDATE content SET status = 'published', published_at = ?, scheduled_at = NULL WHERE id = ?",
            (&now, id),
        )?;
        tracing::info!("Auto-published scheduled content id={}", id);
    }

    tx.commit()?;
    Ok(ids.len())
}

/// Re-render all content HTML from markdown.
/// Useful after updating the markdown renderer to apply changes to existing content.
pub fn rerender_all_content(db: &Database) -> Result<usize> {
    let renderer = super::markdown::MarkdownRenderer::new();
    let mut conn = db.get()?;

    // Get all content IDs and markdown
    let items: Vec<(i64, String)> = {
        let mut stmt = conn.prepare("SELECT id, body_markdown FROM content")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<Vec<_>, _>>()?
    };

    let count = items.len();

    let tx = conn.transaction()?;
    for (id, markdown) in items {
        let html = renderer.render(&markdown);
        tx.execute("UPDATE content SET body_html = ? WHERE id = ?", (&html, id))?;
    }
    tx.commit()?;

    Ok(count)
}
