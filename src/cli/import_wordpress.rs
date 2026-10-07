use crate::models::{ContentStatus, ContentType, CreateContent};
use crate::services::{content, html_to_markdown};
use crate::Config;
use anyhow::Result;
use quick_xml::escape::resolve_predefined_entity;
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

    tracing::info!("Found {} items in WordPress export", items.len());

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
            content_type,
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

    let mut items = Vec::new();

    // State tracking. Text for the current element is accumulated in `text`
    // and assigned on its end tag, because entity references (`&amp;`) arrive
    // as separate events that split a single text node into several pieces.
    let mut in_item = false;
    let mut current_tag = String::new();
    let mut text = String::new();
    let mut title = String::new();
    let mut slug = String::new();
    let mut content_html = String::new();
    let mut status = String::new();
    let mut post_type = String::new();
    let mut published_at = Option::<String>::None;
    let mut tags: Vec<String> = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let full_name = qname.as_ref();

                if e.local_name().as_ref() == "item" {
                    in_item = true;
                    title.clear();
                    slug.clear();
                    content_html.clear();
                    status.clear();
                    post_type.clear();
                    published_at = None;
                    tags.clear();
                } else if in_item {
                    current_tag = full_name.to_string();

                    // Only `<category domain="post_tag">` elements are tags
                    if full_name == "category" {
                        let is_tag = e
                            .attributes()
                            .filter_map(|a| a.ok())
                            .find(|a| a.key.as_ref() == "domain")
                            .is_some_and(|a| a.value.as_ref() == "post_tag");
                        if is_tag {
                            current_tag = "post_tag".to_string();
                        }
                    }
                }
                text.clear();
            }
            Ok(Event::CData(ref e)) => {
                if in_item {
                    text.push_str(e.as_ref());
                }
            }
            Ok(Event::Text(ref e)) => {
                if in_item {
                    text.push_str(&e.xml10_content());
                }
            }
            Ok(Event::GeneralRef(ref e)) => {
                if in_item {
                    if let Ok(Some(ch)) = e.resolve_char_ref() {
                        text.push(ch);
                    } else if let Some(resolved) = resolve_predefined_entity(e.as_ref()) {
                        text.push_str(resolved);
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                if in_item {
                    let value = text.trim();
                    match current_tag.as_str() {
                        "title" => title = value.to_string(),
                        "wp:post_name" => slug = value.to_string(),
                        "content:encoded" => content_html = value.to_string(),
                        "wp:status" => status = value.to_string(),
                        "wp:post_type" => post_type = value.to_string(),
                        "wp:post_date" if !value.is_empty() => {
                            published_at = Some(value.to_string())
                        }
                        "post_tag" if !value.is_empty() => tags.push(value.to_string()),
                        _ => {}
                    }
                }

                if e.local_name().as_ref() == "item" && in_item {
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

                current_tag.clear();
                text.clear();
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                tracing::warn!("XML parsing error: {}", e);
                break;
            }
            _ => {}
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_WXR: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0"
    xmlns:excerpt="http://wordpress.org/export/1.2/excerpt/"
    xmlns:content="http://purl.org/rss/1.0/modules/content/"
    xmlns:wp="http://wordpress.org/export/1.2/">
<channel>
    <title>Site Title</title>
    <item>
        <title>Fish &amp; Chips &#8211; A Guide</title>
        <content:encoded><![CDATA[<p>Hello <strong>world</strong></p>]]></content:encoded>
        <excerpt:encoded><![CDATA[Not the body]]></excerpt:encoded>
        <wp:post_date><![CDATA[2024-01-15 10:30:00]]></wp:post_date>
        <wp:post_date_gmt><![CDATA[2024-01-15 15:30:00]]></wp:post_date_gmt>
        <wp:comment_status><![CDATA[open]]></wp:comment_status>
        <wp:ping_status><![CDATA[closed]]></wp:ping_status>
        <wp:post_name><![CDATA[fish-and-chips]]></wp:post_name>
        <wp:status><![CDATA[publish]]></wp:status>
        <wp:post_type><![CDATA[post]]></wp:post_type>
        <category domain="category" nicename="food"><![CDATA[Food]]></category>
        <category domain="post_tag" nicename="rock-roll"><![CDATA[Rock & Roll]]></category>
        <category domain="post_tag" nicename="uk">UK &amp; Ireland</category>
        <wp:postmeta>
            <wp:meta_key><![CDATA[_edit_last]]></wp:meta_key>
            <wp:meta_value><![CDATA[1]]></wp:meta_value>
        </wp:postmeta>
    </item>
    <item>
        <title>About</title>
        <content:encoded><![CDATA[About page]]></content:encoded>
        <wp:status>draft</wp:status>
        <wp:post_type>page</wp:post_type>
    </item>
</channel>
</rss>"#;

    #[test]
    fn parses_items_with_exact_fields() {
        let items = parse_wxr(SAMPLE_WXR).unwrap();
        assert_eq!(items.len(), 2);

        let post = &items[0];
        assert_eq!(post.title, "Fish & Chips \u{2013} A Guide");
        assert_eq!(post.slug, "fish-and-chips");
        assert_eq!(post.content_html, "<p>Hello <strong>world</strong></p>");
        assert_eq!(post.status, "publish");
        assert_eq!(post.post_type, "post");
        assert_eq!(post.published_at.as_deref(), Some("2024-01-15 10:30:00"));
        assert_eq!(post.tags, vec!["Rock & Roll", "UK & Ireland"]);

        let page = &items[1];
        assert_eq!(page.title, "About");
        assert_eq!(page.status, "draft");
        assert_eq!(page.post_type, "page");
        assert!(page.slug.is_empty());
        assert!(page.tags.is_empty());
    }

    #[test]
    fn ignores_channel_title_outside_items() {
        let items = parse_wxr(SAMPLE_WXR).unwrap();
        assert!(items.iter().all(|i| i.title != "Site Title"));
    }
}
