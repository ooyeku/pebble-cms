use crate::services::analytics::Analytics;
use crate::services::markdown::MarkdownRenderer;
use crate::web::security::{CsrfManager, RateLimiter};
use crate::{Config, Database};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tera::{Tera, Value};

pub struct AppState {
    pub config: Config,
    pub db: Database,
    pub templates: Tera,
    pub markdown: MarkdownRenderer,
    pub media_dir: PathBuf,
    pub production_mode: bool,
    pub csrf: Arc<CsrfManager>,
    pub rate_limiter: Arc<RateLimiter>,
    pub analytics: Option<Arc<Analytics>>,
}

impl AppState {
    pub fn new(config: Config, db: Database, production_mode: bool) -> Result<Self> {
        let mut templates = Tera::default();

        templates.register_filter("format_date", format_date_filter);
        templates.register_filter("truncate_str", truncate_str_filter);
        templates.register_filter("str_slice", str_slice_filter);
        templates.register_filter("strip_md", strip_markdown_filter);
        templates.add_raw_templates(vec![
            (
                "css/bundle.css",
                include_str!("../../templates/css/bundle.css"),
            ),
            (
                "css/bundle-admin.css",
                include_str!("../../templates/css/bundle-admin.css"),
            ),
            ("base.html", include_str!("../../templates/base.html")),
            (
                "admin/base.html",
                include_str!("../../templates/admin/base.html"),
            ),
            (
                "admin/login.html",
                include_str!("../../templates/admin/login.html"),
            ),
            (
                "admin/setup.html",
                include_str!("../../templates/admin/setup.html"),
            ),
            (
                "admin/dashboard.html",
                include_str!("../../templates/admin/dashboard.html"),
            ),
            (
                "admin/posts/index.html",
                include_str!("../../templates/admin/posts/index.html"),
            ),
            (
                "admin/posts/form.html",
                include_str!("../../templates/admin/posts/form.html"),
            ),
            (
                "admin/pages/index.html",
                include_str!("../../templates/admin/pages/index.html"),
            ),
            (
                "admin/pages/form.html",
                include_str!("../../templates/admin/pages/form.html"),
            ),
            (
                "admin/media/index.html",
                include_str!("../../templates/admin/media/index.html"),
            ),
            (
                "admin/tags/index.html",
                include_str!("../../templates/admin/tags/index.html"),
            ),
            (
                "admin/settings/index.html",
                include_str!("../../templates/admin/settings/index.html"),
            ),
            (
                "admin/users/index.html",
                include_str!("../../templates/admin/users/index.html"),
            ),
            (
                "public/index.html",
                include_str!("../../templates/public/index.html"),
            ),
            (
                "public/posts.html",
                include_str!("../../templates/public/posts.html"),
            ),
            (
                "public/post.html",
                include_str!("../../templates/public/post.html"),
            ),
            (
                "public/page.html",
                include_str!("../../templates/public/page.html"),
            ),
            (
                "public/tag.html",
                include_str!("../../templates/public/tag.html"),
            ),
            (
                "public/tags.html",
                include_str!("../../templates/public/tags.html"),
            ),
            (
                "public/search.html",
                include_str!("../../templates/public/search.html"),
            ),
            (
                "public/404.html",
                include_str!("../../templates/public/404.html"),
            ),
            (
                "htmx/preview.html",
                include_str!("../../templates/htmx/preview.html"),
            ),
            (
                "htmx/flash.html",
                include_str!("../../templates/htmx/flash.html"),
            ),
            (
                "htmx/analytics_realtime.html",
                include_str!("../../templates/htmx/analytics_realtime.html"),
            ),
            (
                "admin/analytics/index.html",
                include_str!("../../templates/admin/analytics/index.html"),
            ),
            (
                "admin/database/index.html",
                include_str!("../../templates/admin/database/index.html"),
            ),
        ])?;

        let media_dir = PathBuf::from(&config.media.upload_dir);

        Ok(Self {
            config,
            db,
            templates,
            markdown: MarkdownRenderer::new(),
            media_dir,
            production_mode,
            csrf: Arc::new(CsrfManager::default()),
            rate_limiter: Arc::new(RateLimiter::default()),
            analytics: None,
        })
    }

    pub fn with_analytics(mut self, analytics: Arc<Analytics>) -> Self {
        self.analytics = Some(analytics);
        self
    }
}

fn format_date_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let date_str = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("format_date requires a string"))?;

    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("%B %d, %Y");

    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
        return Ok(Value::String(dt.format(format).to_string()));
    }

    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%dT%H:%M:%S%.f") {
        return Ok(Value::String(dt.format(format).to_string()));
    }

    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, "%Y-%m-%d %H:%M:%S") {
        return Ok(Value::String(dt.format(format).to_string()));
    }

    Ok(Value::String(date_str.to_string()))
}

fn truncate_str_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("truncate_str requires a string"))?;
    let len = args.get("len").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
    if s.len() > len {
        Ok(Value::String(s[..len].to_string()))
    } else {
        Ok(Value::String(s.to_string()))
    }
}

fn str_slice_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("str_slice requires a string"))?;
    let start = args.get("start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let end = args
        .get("end")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(s.len());
    let start = start.min(s.len());
    let end = end.min(s.len());
    Ok(Value::String(s[start..end].to_string()))
}

fn strip_markdown_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let text = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("strip_md requires a string"))?;

    let mut result = text.to_string();

    // Remove inline code
    while let Some(start) = result.find('`') {
        if let Some(end) = result[start + 1..].find('`') {
            let code_content = &result[start + 1..start + 1 + end];
            result = format!(
                "{}{}{}",
                &result[..start],
                code_content,
                &result[start + 2 + end..]
            );
        } else {
            break;
        }
    }

    // Remove images ![alt](url)
    while let Some(img_start) = result.find("![") {
        if let Some(bracket_end) = result[img_start + 2..].find("](") {
            let abs_bracket_end = img_start + 2 + bracket_end;
            if let Some(paren_end) = result[abs_bracket_end + 2..].find(')') {
                result = format!(
                    "{}{}",
                    &result[..img_start],
                    &result[abs_bracket_end + 3 + paren_end..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Remove links [text](url) -> text
    while let Some(bracket_start) = result.find('[') {
        if let Some(bracket_end) = result[bracket_start..].find("](") {
            let abs_bracket_end = bracket_start + bracket_end;
            if let Some(paren_end) = result[abs_bracket_end + 2..].find(')') {
                let link_text = &result[bracket_start + 1..abs_bracket_end];
                result = format!(
                    "{}{}{}",
                    &result[..bracket_start],
                    link_text,
                    &result[abs_bracket_end + 3 + paren_end..]
                );
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Remove bold/italic markers
    result = result.replace("***", "");
    result = result.replace("**", "");
    result = result.replace("__", "");
    result = result.replace('*', "");
    result = result.replace('_', " ");

    // Remove list markers
    result = result
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                trimmed[2..].to_string()
            } else if trimmed.len() > 3 && trimmed.chars().next().unwrap().is_ascii_digit() {
                if let Some(pos) = trimmed.find(". ") {
                    trimmed[pos + 2..].to_string()
                } else {
                    line.to_string()
                }
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    // Clean up multiple spaces
    while result.contains("  ") {
        result = result.replace("  ", " ");
    }

    Ok(Value::String(result.trim().to_string()))
}
