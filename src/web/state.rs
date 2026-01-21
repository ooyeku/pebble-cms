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
}

impl AppState {
    pub fn new(config: Config, db: Database, production_mode: bool) -> Result<Self> {
        let mut templates = Tera::default();

        templates.register_filter("format_date", format_date_filter);
        templates.register_filter("truncate_str", truncate_str_filter);
        templates.add_raw_templates(vec![
            ("css/bundle.css", include_str!("../../templates/css/bundle.css")),
            ("css/bundle-admin.css", include_str!("../../templates/css/bundle-admin.css")),
            ("base.html", include_str!("../../templates/base.html")),
            ("admin/base.html", include_str!("../../templates/admin/base.html")),
            ("admin/login.html", include_str!("../../templates/admin/login.html")),
            ("admin/setup.html", include_str!("../../templates/admin/setup.html")),
            ("admin/dashboard.html", include_str!("../../templates/admin/dashboard.html")),
            ("admin/posts/index.html", include_str!("../../templates/admin/posts/index.html")),
            ("admin/posts/form.html", include_str!("../../templates/admin/posts/form.html")),
            ("admin/pages/index.html", include_str!("../../templates/admin/pages/index.html")),
            ("admin/pages/form.html", include_str!("../../templates/admin/pages/form.html")),
            ("admin/media/index.html", include_str!("../../templates/admin/media/index.html")),
            ("admin/tags/index.html", include_str!("../../templates/admin/tags/index.html")),
            ("admin/settings/index.html", include_str!("../../templates/admin/settings/index.html")),
            ("admin/users/index.html", include_str!("../../templates/admin/users/index.html")),
            ("public/index.html", include_str!("../../templates/public/index.html")),
            ("public/post.html", include_str!("../../templates/public/post.html")),
            ("public/page.html", include_str!("../../templates/public/page.html")),
            ("public/tag.html", include_str!("../../templates/public/tag.html")),
            ("public/tags.html", include_str!("../../templates/public/tags.html")),
            ("public/search.html", include_str!("../../templates/public/search.html")),
            ("public/404.html", include_str!("../../templates/public/404.html")),
            ("htmx/preview.html", include_str!("../../templates/htmx/preview.html")),
            ("htmx/flash.html", include_str!("../../templates/htmx/flash.html")),
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
        })
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
    let s = value.as_str().ok_or_else(|| tera::Error::msg("truncate_str requires a string"))?;
    let len = args.get("len").and_then(|v| v.as_u64()).unwrap_or(16) as usize;
    if s.len() > len {
        Ok(Value::String(s[..len].to_string()))
    } else {
        Ok(Value::String(s.to_string()))
    }
}
