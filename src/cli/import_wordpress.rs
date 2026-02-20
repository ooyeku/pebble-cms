use crate::models::{ContentStatus, ContentType, CreateContent};
use crate::services::{content, html_to_markdown};
use crate::Config;
use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use std::path::Path;

#[allow(dead_code)]
struct WxrItem {
    title: String,
    slug: String,
    content_html: String,
    status: String,
    post_type: String,
    published_at: Option<String>,
    tags: Vec<String>,
}

pub async fn run(config_path: &Path, file: &Path, overwrite: bool) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = crate::Database::open(&config.database.path)?;
    db.migrate()?;

    if !file.exists() {
        anyhow::bail!("WordPress export file not found: {}", file.display());
    }

    let xml_content = std::fs::read_to_string(file)?;
    let items = parse_wxr(&xml_content)?;

    tracing::info!(
        "Found {} items in WordPress export",
        items.len()
    );

    let mut posts_imported = 0;
    let mut pages_imported = 0;
    let mut skipped = 0;

    for item in items {
        let content_type = match item.post_type.as_str() {
            "post" => ContentType::Post,
            "page" => ContentType::Page,
            _ => {
                skipped += 1;
                continue;
            }
        };

        let status = match item.status.as_str() {
            "publish" => ContentStatus::Published,
            "draft" => ContentStatus::Draft,
            "private" => ContentStatus::Draft,
            _ => ContentStatus::Draft,
        };

        let markdown = html_to_markdown::convert(&item.content_html);

        let slug = if item.slug.is_empty() {
            crate::services::slug::generate_slug(&item.title)
        } else {
            item.slug.clone()
        };

        // Check for existing content
        if let Ok(Some(_)) = content::get_content_by_slug(&db, &slug) {
            if !overwrite {
                tracing::info!("Skipping existing: {}", slug);
                skipped += 1;
                continue;
            }
            // Delete existing for overwrite
            let conn = db.get()?;
            let _ = conn.execute("DELETE FROM content WHERE slug = ?", [&slug]);
        }

        let input = CreateContent {
            title: item.title,
            slug: Some(slug.clone()),
            content_type: content_type.clone(),
            body_markdown: markdown,
            status,
            scheduled_at: None,
            excerpt: None,
            featured_image: None,
            tags: item.tags,
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
        "WordPress import complete: {} posts, {} pages imported, {} skipped",
        posts_imported,
        pages_imported,
        skipped
    );
    Ok(())
}

fn parse_wxr(xml: &str) -> Result<Vec<WxrItem>> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut items = Vec::new();
    let mut buf = Vec::new();

    // State tracking
    let mut in_item = false;
    let mut current_tag = String::new();
    let mut title = String::new();
    let mut slug = String::new();
    let mut content_html = String::new();
    let mut status = String::new();
    let mut post_type = String::new();
    let mut published_at = Option::<String>::None;
    let mut tags: Vec<String> = Vec::new();
    let mut _in_content_encoded = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let local_name = e.local_name();
                let tag_name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");

                if tag_name == "item" {
                    in_item = true;
                    title.clear();
                    slug.clear();
                    content_html.clear();
                    status.clear();
                    post_type.clear();
                    published_at = None;
                    tags.clear();
                } else if in_item {
                    current_tag = tag_name.to_string();

                    // Check for wp:post_name, wp:status, wp:post_type, wp:post_date
                    // quick-xml handles namespaced elements; the local name strips prefix
                    let qname = e.name();
                    let full_name = std::str::from_utf8(qname.as_ref()).unwrap_or("");
                    if full_name.contains("post_name") {
                        current_tag = "wp:post_name".to_string();
                    } else if full_name.contains("status") && full_name.contains("wp") {
                        current_tag = "wp:status".to_string();
                    } else if full_name.contains("post_type") {
                        current_tag = "wp:post_type".to_string();
                    } else if full_name.contains("post_date") && !full_name.contains("gmt") {
                        current_tag = "wp:post_date".to_string();
                    } else if full_name.contains("encoded") {
                        _in_content_encoded = true;
                        current_tag = "content:encoded".to_string();
                    }

                    // Check for tag categories
                    if tag_name == "category" {
                        let domain = e.attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == b"domain")
                            .and_then(|a| String::from_utf8(a.value.to_vec()).ok());
                        if domain.as_deref() == Some("post_tag") {
                            current_tag = "post_tag".to_string();
                        }
                    }
                }
            }
            Ok(Event::CData(ref e)) => {
                if in_item {
                    let text = std::str::from_utf8(e.as_ref()).unwrap_or("");
                    match current_tag.as_str() {
                        "content:encoded" => content_html.push_str(text),
                        "title" => title.push_str(text),
                        _ => {}
                    }
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_item {
                    let text = e.unescape().unwrap_or_default();
                    match current_tag.as_str() {
                        "title" => title.push_str(&text),
                        "wp:post_name" => slug.push_str(&text),
                        "content:encoded" => content_html.push_str(&text),
                        "wp:status" => status.push_str(&text),
                        "wp:post_type" => post_type.push_str(&text),
                        "wp:post_date" => published_at = Some(text.to_string()),
                        "post_tag" => {
                            let tag = text.trim().to_string();
                            if !tag.is_empty() {
                                tags.push(tag);
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let local_name = e.local_name();
                let tag_name = std::str::from_utf8(local_name.as_ref()).unwrap_or("");

                if tag_name == "item" && in_item {
                    if !title.is_empty() {
                        items.push(WxrItem {
                            title: title.clone(),
                            slug: slug.clone(),
                            content_html: content_html.clone(),
                            status: status.clone(),
                            post_type: post_type.clone(),
                            published_at: published_at.clone(),
                            tags: tags.clone(),
                        });
                    }
                    in_item = false;
                }

                let end_qname = e.name();
                let full_name = std::str::from_utf8(end_qname.as_ref()).unwrap_or("");
                if full_name.contains("encoded") {
                    _in_content_encoded = false;
                }
                current_tag.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parsing error: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(items)
}
