use crate::models::{ContentStatus, ContentType};
use crate::services::content;
use crate::Config;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub async fn run(
    config_path: &Path,
    output_dir: &Path,
    include_drafts: bool,
    include_media: bool,
    format: &str,
) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = crate::Database::open(&config.database.path)?;

    match format {
        "hugo" => export_hugo(&db, &config, output_dir, include_drafts, include_media),
        "zola" => export_zola(&db, &config, output_dir, include_drafts, include_media),
        _ => export_pebble(&db, &config, output_dir, include_drafts, include_media),
    }
}

fn export_pebble(
    db: &crate::Database,
    config: &Config,
    output_dir: &Path,
    include_drafts: bool,
    include_media: bool,
) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(output_dir.join("posts"))?;
    fs::create_dir_all(output_dir.join("pages"))?;

    let status = if include_drafts {
        None
    } else {
        Some(ContentStatus::Published)
    };

    let posts = content::list_content(db, Some(ContentType::Post), status.clone(), 10000, 0)?;
    let pages = content::list_content(db, Some(ContentType::Page), status, 10000, 0)?;

    tracing::info!("Exporting {} posts and {} pages", posts.len(), pages.len());

    for post in posts {
        let full_content = content::get_content_by_id(db, post.id)?;
        if let Some(c) = full_content {
            let filename = format!("{}.md", c.content.slug);
            let filepath = output_dir.join("posts").join(&filename);

            let frontmatter = format!(
                r#"---
title: "{}"
slug: "{}"
status: "{}"
published_at: {}
created_at: "{}"
---

"#,
                c.content.title.replace('"', r#"\""#),
                c.content.slug,
                c.content.status,
                c.content
                    .published_at
                    .as_deref()
                    .map(|d| format!("\"{}\"", d))
                    .unwrap_or_else(|| "null".to_string()),
                c.content.created_at,
            );

            let content_str = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content_str)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    for page in pages {
        let full_content = content::get_content_by_id(db, page.id)?;
        if let Some(c) = full_content {
            let filename = format!("{}.md", c.content.slug);
            let filepath = output_dir.join("pages").join(&filename);

            let frontmatter = format!(
                r#"---
title: "{}"
slug: "{}"
status: "{}"
created_at: "{}"
---

"#,
                c.content.title.replace('"', r#"\""#),
                c.content.slug,
                c.content.status,
                c.content.created_at,
            );

            let content_str = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content_str)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    if include_media {
        copy_media(config, output_dir, "media")?;
    }

    tracing::info!("Export complete to {}", output_dir.display());
    Ok(())
}

fn export_hugo(
    db: &crate::Database,
    config: &Config,
    output_dir: &Path,
    include_drafts: bool,
    include_media: bool,
) -> Result<()> {
    let posts_dir = output_dir.join("content").join("posts");
    let pages_dir = output_dir.join("content");
    fs::create_dir_all(&posts_dir)?;
    fs::create_dir_all(&pages_dir)?;

    let status = if include_drafts {
        None
    } else {
        Some(ContentStatus::Published)
    };

    let posts = content::list_content(db, Some(ContentType::Post), status.clone(), 10000, 0)?;
    let pages = content::list_content(db, Some(ContentType::Page), status, 10000, 0)?;

    tracing::info!("Exporting {} posts and {} pages (Hugo format)", posts.len(), pages.len());

    for post in posts {
        let full_content = content::get_content_by_id(db, post.id)?;
        if let Some(c) = full_content {
            let filename = format!("{}.md", c.content.slug);
            let filepath = posts_dir.join(&filename);

            let date = c.content.published_at.as_deref()
                .or(Some(&c.content.created_at))
                .unwrap_or("");

            let is_draft = c.content.status != ContentStatus::Published;
            let tag_names: Vec<&str> = c.tags.iter().map(|t| t.name.as_str()).collect();

            let mut frontmatter = format!(
                "+++\ntitle = \"{}\"\nslug = \"{}\"\ndate = \"{}\"\ndraft = {}\n",
                c.content.title.replace('"', "\\\""),
                c.content.slug,
                date,
                is_draft,
            );

            if !tag_names.is_empty() {
                let tags_str = tag_names.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ");
                frontmatter.push_str(&format!("tags = [{}]\n", tags_str));
            }

            if let Some(ref excerpt) = c.content.excerpt {
                frontmatter.push_str(&format!("description = \"{}\"\n", excerpt.replace('"', "\\\"")));
            }

            if let Some(ref img) = c.content.featured_image {
                frontmatter.push_str(&format!("images = [\"{}\"]\n", img));
            }

            frontmatter.push_str("+++\n\n");

            let content_str = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content_str)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    for page in pages {
        let full_content = content::get_content_by_id(db, page.id)?;
        if let Some(c) = full_content {
            let filename = format!("{}.md", c.content.slug);
            let filepath = pages_dir.join(&filename);

            let date = c.content.published_at.as_deref()
                .or(Some(&c.content.created_at))
                .unwrap_or("");

            let is_draft = c.content.status != ContentStatus::Published;

            let frontmatter = format!(
                "+++\ntitle = \"{}\"\nslug = \"{}\"\ndate = \"{}\"\ndraft = {}\n+++\n\n",
                c.content.title.replace('"', "\\\""),
                c.content.slug,
                date,
                is_draft,
            );

            let content_str = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content_str)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    if include_media {
        copy_media(config, output_dir, "static/media")?;
    }

    tracing::info!("Hugo export complete to {}", output_dir.display());
    Ok(())
}

fn export_zola(
    db: &crate::Database,
    config: &Config,
    output_dir: &Path,
    include_drafts: bool,
    include_media: bool,
) -> Result<()> {
    let posts_dir = output_dir.join("content").join("blog");
    let pages_dir = output_dir.join("content");
    fs::create_dir_all(&posts_dir)?;
    fs::create_dir_all(&pages_dir)?;

    let status = if include_drafts {
        None
    } else {
        Some(ContentStatus::Published)
    };

    let posts = content::list_content(db, Some(ContentType::Post), status.clone(), 10000, 0)?;
    let pages = content::list_content(db, Some(ContentType::Page), status, 10000, 0)?;

    tracing::info!("Exporting {} posts and {} pages (Zola format)", posts.len(), pages.len());

    for post in posts {
        let full_content = content::get_content_by_id(db, post.id)?;
        if let Some(c) = full_content {
            let filename = format!("{}.md", c.content.slug);
            let filepath = posts_dir.join(&filename);

            let date = c.content.published_at.as_deref()
                .or(Some(&c.content.created_at))
                .unwrap_or("");

            let is_draft = c.content.status != ContentStatus::Published;
            let tag_names: Vec<&str> = c.tags.iter().map(|t| t.name.as_str()).collect();

            let mut frontmatter = format!(
                "+++\ntitle = \"{}\"\nslug = \"{}\"\ndate = \"{}\"\ndraft = {}\n",
                c.content.title.replace('"', "\\\""),
                c.content.slug,
                date,
                is_draft,
            );

            if let Some(ref excerpt) = c.content.excerpt {
                frontmatter.push_str(&format!("description = \"{}\"\n", excerpt.replace('"', "\\\"")));
            }

            if !tag_names.is_empty() {
                let tags_str = tag_names.iter().map(|t| format!("\"{}\"", t)).collect::<Vec<_>>().join(", ");
                frontmatter.push_str(&format!("\n[taxonomies]\ntags = [{}]\n", tags_str));
            }

            if let Some(ref img) = c.content.featured_image {
                frontmatter.push_str(&format!("\n[extra]\nimage = \"{}\"\n", img));
            }

            frontmatter.push_str("+++\n\n");

            let content_str = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content_str)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    for page in pages {
        let full_content = content::get_content_by_id(db, page.id)?;
        if let Some(c) = full_content {
            let filename = format!("{}.md", c.content.slug);
            let filepath = pages_dir.join(&filename);

            let date = c.content.published_at.as_deref()
                .or(Some(&c.content.created_at))
                .unwrap_or("");

            let is_draft = c.content.status != ContentStatus::Published;

            let frontmatter = format!(
                "+++\ntitle = \"{}\"\nslug = \"{}\"\ndate = \"{}\"\ndraft = {}\n+++\n\n",
                c.content.title.replace('"', "\\\""),
                c.content.slug,
                date,
                is_draft,
            );

            let content_str = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content_str)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    if include_media {
        copy_media(config, output_dir, "static/media")?;
    }

    tracing::info!("Zola export complete to {}", output_dir.display());
    Ok(())
}

fn copy_media(config: &Config, output_dir: &Path, subdir: &str) -> Result<()> {
    let media_src = Path::new(&config.media.upload_dir);
    if media_src.exists() {
        let media_dest = output_dir.join(subdir);
        fs::create_dir_all(&media_dest)?;

        let mut count = 0;
        for entry in fs::read_dir(media_src)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    fs::copy(&path, media_dest.join(filename))?;
                    count += 1;
                }
            }
        }
        tracing::info!("Exported {} media files", count);
    }
    Ok(())
}
