use crate::models::ContentType;
use crate::services::content;
use crate::Config;
use anyhow::Result;
use std::fs;
use std::path::Path;

pub async fn run(config_path: &Path, output_dir: &Path, include_drafts: bool) -> Result<()> {
    let config = Config::load(config_path)?;
    let db = crate::Database::open(&config.database.path)?;

    fs::create_dir_all(output_dir)?;
    fs::create_dir_all(output_dir.join("posts"))?;
    fs::create_dir_all(output_dir.join("pages"))?;

    let status = if include_drafts {
        None
    } else {
        Some(crate::models::ContentStatus::Published)
    };

    let posts = content::list_content(&db, Some(ContentType::Post), status, 10000, 0)?;
    let pages = content::list_content(&db, Some(ContentType::Page), status, 10000, 0)?;

    tracing::info!("Exporting {} posts and {} pages", posts.len(), pages.len());

    for post in posts {
        let full_content = content::get_content_by_id(&db, post.id)?;
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

            let content = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    for page in pages {
        let full_content = content::get_content_by_id(&db, page.id)?;
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

            let content = format!("{}{}", frontmatter, c.content.body_markdown);
            fs::write(&filepath, content)?;
            tracing::info!("Exported: {}", filepath.display());
        }
    }

    tracing::info!("Export complete to {}", output_dir.display());
    Ok(())
}
