use crate::models::{ContentStatus, ContentType, CreateContent};
use crate::services::{content, html_to_markdown};
use crate::Config;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub async fn run(config_path: &Path, file: &Path, overwrite: bool) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = crate::Database::open(&config.database.path)?;
    db.migrate()?;

    if !file.exists() {
        anyhow::bail!("Ghost export file not found: {}", file.display());
    }

    let json_content = std::fs::read_to_string(file)?;
    let export: Value = serde_json::from_str(&json_content)?;

    // Ghost export format: { "db": [{ "data": { "posts": [...], "tags": [...], "posts_tags": [...] } }] }
    let data = export
        .get("db")
        .and_then(|db| db.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("data"))
        .ok_or_else(|| anyhow::anyhow!("Invalid Ghost export format: missing db[0].data"))?;

    let posts = data
        .get("posts")
        .and_then(|p| p.as_array())
        .cloned()
        .unwrap_or_default();

    let ghost_tags = data
        .get("tags")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    let posts_tags = data
        .get("posts_tags")
        .and_then(|pt| pt.as_array())
        .cloned()
        .unwrap_or_default();

    // Build tag lookup: ghost tag id -> tag name
    let mut tag_map: HashMap<String, String> = HashMap::new();
    for tag in &ghost_tags {
        let id = tag.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let name = tag.get("name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if !id.is_empty() && !name.is_empty() {
            tag_map.insert(id, name);
        }
    }

    // Build post -> tags mapping
    let mut post_tags_map: HashMap<String, Vec<String>> = HashMap::new();
    for pt in &posts_tags {
        let post_id = pt.get("post_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let tag_id = pt.get("tag_id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if let Some(tag_name) = tag_map.get(&tag_id) {
            post_tags_map
                .entry(post_id)
                .or_default()
                .push(tag_name.clone());
        }
    }

    tracing::info!("Found {} posts/pages in Ghost export", posts.len());

    let mut posts_imported = 0;
    let mut pages_imported = 0;
    let mut skipped = 0;

    for post in &posts {
        let title = post.get("title").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
        let slug = post.get("slug").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let ghost_status = post.get("status").and_then(|v| v.as_str()).unwrap_or("draft");
        let post_type = post.get("type").and_then(|v| v.as_str()).unwrap_or("post");
        let ghost_id = post.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();

        let content_type = match post_type {
            "post" => ContentType::Post,
            "page" => ContentType::Page,
            _ => {
                skipped += 1;
                continue;
            }
        };

        let status = match ghost_status {
            "published" => ContentStatus::Published,
            "scheduled" => ContentStatus::Scheduled,
            _ => ContentStatus::Draft,
        };

        // Get the content: prefer html, fall back to mobiledoc
        let html = post.get("html").and_then(|v| v.as_str()).unwrap_or_default();
        let body_html = if html.is_empty() {
            // Try mobiledoc
            extract_mobiledoc_text(post.get("mobiledoc").and_then(|v| v.as_str()).unwrap_or(""))
        } else {
            html.to_string()
        };

        let markdown = html_to_markdown::convert(&body_html);

        let slug = if slug.is_empty() {
            crate::services::slug::generate_slug(&title)
        } else {
            slug
        };

        // Check for existing content
        if let Ok(Some(_)) = content::get_content_by_slug(&db, &slug) {
            if !overwrite {
                tracing::info!("Skipping existing: {}", slug);
                skipped += 1;
                continue;
            }
            let conn = db.get()?;
            let _ = conn.execute("DELETE FROM content WHERE slug = ?", [&slug]);
        }

        let tags = post_tags_map.get(&ghost_id).cloned().unwrap_or_default();

        let input = CreateContent {
            title,
            slug: Some(slug.clone()),
            content_type: content_type.clone(),
            body_markdown: markdown,
            status,
            scheduled_at: None,
            excerpt: post.get("custom_excerpt").and_then(|v| v.as_str()).map(|s| s.to_string()),
            featured_image: post.get("feature_image").and_then(|v| v.as_str()).map(|s| s.to_string()),
            tags,
            metadata: None,
        };

        match content::create_content(&db, input, None, config.content.excerpt_length) {
            Ok(_) => {
                match content_type {
                    ContentType::Post => posts_imported += 1,
                    ContentType::Page => pages_imported += 1,
                    _ => {}
                }
                tracing::info!("Imported: {} ({})", slug, content_type);
            }
            Err(e) => {
                tracing::warn!("Failed to import {}: {}", slug, e);
                skipped += 1;
            }
        }
    }

    tracing::info!(
        "Ghost import complete: {} posts, {} pages imported, {} skipped",
        posts_imported,
        pages_imported,
        skipped
    );
    Ok(())
}

/// Extract plain text from Ghost's mobiledoc format.
/// Mobiledoc is a JSON-based document format used by Ghost.
fn extract_mobiledoc_text(mobiledoc_str: &str) -> String {
    if mobiledoc_str.is_empty() {
        return String::new();
    }

    let mobiledoc: Value = match serde_json::from_str(mobiledoc_str) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    let mut parts = Vec::new();

    // Extract text from sections
    if let Some(sections) = mobiledoc.get("sections").and_then(|s| s.as_array()) {
        for section in sections {
            if let Some(arr) = section.as_array() {
                // [1, "p", [[0, [], 0, "text"]]]  -- markup section
                if arr.len() >= 3 {
                    if let Some(markers) = arr.get(2).and_then(|m| m.as_array()) {
                        for marker in markers {
                            if let Some(m_arr) = marker.as_array() {
                                // Last element is the text
                                if let Some(text) = m_arr.last().and_then(|t| t.as_str()) {
                                    parts.push(text.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Extract from cards
    if let Some(cards) = mobiledoc.get("cards").and_then(|c| c.as_array()) {
        for card in cards {
            if let Some(arr) = card.as_array() {
                if arr.len() >= 2 {
                    let card_type = arr.first().and_then(|t| t.as_str()).unwrap_or("");
                    let payload = arr.get(1);
                    match card_type {
                        "html" => {
                            if let Some(html) = payload.and_then(|p| p.get("html")).and_then(|h| h.as_str()) {
                                parts.push(html.to_string());
                            }
                        }
                        "markdown" => {
                            if let Some(md) = payload.and_then(|p| p.get("markdown")).and_then(|m| m.as_str()) {
                                parts.push(md.to_string());
                            }
                        }
                        "image" => {
                            if let Some(src) = payload.and_then(|p| p.get("src")).and_then(|s| s.as_str()) {
                                let alt = payload.and_then(|p| p.get("alt")).and_then(|a| a.as_str()).unwrap_or("");
                                parts.push(format!("<img src=\"{}\" alt=\"{}\" />", src, alt));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    parts.join("\n\n")
}
