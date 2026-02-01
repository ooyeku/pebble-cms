use crate::services::analytics::Analytics;
use crate::services::markdown::MarkdownRenderer;
use crate::web::security::{CsrfManager, RateLimiter};
use crate::{Config, Database};
use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tera::{Tera, Value};

pub struct AppState {
    pub config: RwLock<Config>,
    pub config_path: PathBuf,
    pub db: Database,
    pub templates: Tera,
    pub markdown: MarkdownRenderer,
    pub media_dir: PathBuf,
    pub production_mode: bool,
    pub csrf: Arc<CsrfManager>,
    pub rate_limiter: Arc<RateLimiter>,
    pub upload_rate_limiter: Arc<RateLimiter>,
    pub analytics: Option<Arc<Analytics>>,
    pub static_assets: HashMap<String, &'static str>,
}

impl AppState {
    pub fn new(config: Config, config_path: PathBuf, db: Database, production_mode: bool) -> Result<Self> {
        let mut templates = Tera::default();

        templates.register_filter("format_date", format_date_filter);
        templates.register_filter("truncate_str", truncate_str_filter);
        templates.register_filter("str_slice", str_slice_filter);
        templates.register_filter("strip_md", strip_markdown_filter);
        templates.register_filter("filesizeformat", filesizeformat_filter);
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
                "htmx/analytics_content.html",
                include_str!("../../templates/htmx/analytics_content.html"),
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

        let mut static_assets = HashMap::new();
        static_assets.insert(
            "theme.js".to_string(),
            include_str!("../../templates/js/theme.js"),
        );
        static_assets.insert(
            "admin.js".to_string(),
            include_str!("../../templates/js/admin.js"),
        );

        Ok(Self {
            config: RwLock::new(config),
            config_path,
            db,
            templates,
            markdown: MarkdownRenderer::new(),
            media_dir,
            production_mode,
            csrf: Arc::new(CsrfManager::default()),
            rate_limiter: Arc::new(RateLimiter::default()),
            upload_rate_limiter: Arc::new(RateLimiter::new(
                20,
                std::time::Duration::from_secs(60),
                std::time::Duration::from_secs(300),
            )),
            analytics: None,
            static_assets,
        })
    }

    /// Get a read lock on the config
    pub fn config(&self) -> std::sync::RwLockReadGuard<'_, Config> {
        self.config.read().unwrap()
    }

    /// Update the config (writes to file and updates in-memory)
    pub fn update_config(&self, new_config: Config) -> Result<()> {
        // Validate new config
        new_config.validate()?;

        // Write to file using toml_edit to preserve formatting
        let content = std::fs::read_to_string(&self.config_path)?;
        let mut doc = content.parse::<toml_edit::DocumentMut>()?;

        // Update all fields
        doc["site"]["title"] = toml_edit::value(&new_config.site.title);
        doc["site"]["description"] = toml_edit::value(&new_config.site.description);
        doc["site"]["url"] = toml_edit::value(&new_config.site.url);
        doc["site"]["language"] = toml_edit::value(&new_config.site.language);

        doc["content"]["posts_per_page"] = toml_edit::value(new_config.content.posts_per_page as i64);
        doc["content"]["excerpt_length"] = toml_edit::value(new_config.content.excerpt_length as i64);
        doc["content"]["auto_excerpt"] = toml_edit::value(new_config.content.auto_excerpt);

        doc["theme"]["name"] = toml_edit::value(&new_config.theme.name);

        // Handle theme.custom
        if !doc["theme"].as_table().map_or(false, |t| t.contains_key("custom")) {
            doc["theme"]["custom"] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        if let Some(ref v) = new_config.theme.custom.primary_color {
            doc["theme"]["custom"]["primary_color"] = toml_edit::value(v);
        }
        if let Some(ref v) = new_config.theme.custom.accent_color {
            doc["theme"]["custom"]["accent_color"] = toml_edit::value(v);
        }
        if let Some(ref v) = new_config.theme.custom.background_color {
            doc["theme"]["custom"]["background_color"] = toml_edit::value(v);
        }
        if let Some(ref v) = new_config.theme.custom.text_color {
            doc["theme"]["custom"]["text_color"] = toml_edit::value(v);
        }

        // Handle homepage section
        if !doc.contains_key("homepage") {
            doc["homepage"] = toml_edit::Item::Table(toml_edit::Table::new());
        }
        doc["homepage"]["show_hero"] = toml_edit::value(new_config.homepage.show_hero);
        doc["homepage"]["hero_layout"] = toml_edit::value(&new_config.homepage.hero_layout);
        doc["homepage"]["hero_height"] = toml_edit::value(&new_config.homepage.hero_height);
        doc["homepage"]["hero_text_align"] = toml_edit::value(&new_config.homepage.hero_text_align);
        doc["homepage"]["show_posts"] = toml_edit::value(new_config.homepage.show_posts);
        doc["homepage"]["posts_layout"] = toml_edit::value(&new_config.homepage.posts_layout);
        doc["homepage"]["posts_columns"] = toml_edit::value(new_config.homepage.posts_columns as i64);
        doc["homepage"]["show_pages"] = toml_edit::value(new_config.homepage.show_pages);
        doc["homepage"]["pages_layout"] = toml_edit::value(&new_config.homepage.pages_layout);

        std::fs::write(&self.config_path, doc.to_string())?;

        // Update in-memory config
        let mut config = self.config.write().unwrap();
        *config = new_config;

        Ok(())
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
    let char_count = s.chars().count();
    if char_count > len {
        Ok(Value::String(s.chars().take(len).collect()))
    } else {
        Ok(Value::String(s.to_string()))
    }
}

fn str_slice_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
    let s = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("str_slice requires a string"))?;
    let start = args.get("start").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let char_count = s.chars().count();
    let end = args
        .get("end")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .unwrap_or(char_count);
    let start = start.min(char_count);
    let end = end.min(char_count);
    Ok(Value::String(
        s.chars()
            .skip(start)
            .take(end.saturating_sub(start))
            .collect(),
    ))
}

fn strip_markdown_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let text = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("strip_md requires a string"))?;

    let mut result = text.to_string();

    // Remove inline code using char-safe iteration
    loop {
        let chars: Vec<char> = result.chars().collect();
        if let Some(start) = chars.iter().position(|&c| c == '`') {
            if let Some(rel_end) = chars[start + 1..].iter().position(|&c| c == '`') {
                let end = start + 1 + rel_end;
                let code_content: String = chars[start + 1..end].iter().collect();
                let before: String = chars[..start].iter().collect();
                let after: String = chars[end + 1..].iter().collect();
                result = format!("{}{}{}", before, code_content, after);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    // Remove images ![alt](url)
    while let Some(img_start) = result.find("![") {
        if let Some(bracket_end) = result[img_start + 2..].find("](") {
            let abs_bracket_end = img_start + 2 + bracket_end;
            if let Some(paren_end) = result[abs_bracket_end + 2..].find(')') {
                let before = &result[..img_start];
                let after = &result[abs_bracket_end + 3 + paren_end..];
                result = format!("{}{}", before, after);
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
                let before = &result[..bracket_start];
                let after = &result[abs_bracket_end + 3 + paren_end..];
                result = format!("{}{}{}", before, link_text, after);
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
            if trimmed.starts_with("- ") {
                trimmed.chars().skip(2).collect()
            } else if trimmed.starts_with("* ") {
                trimmed.chars().skip(2).collect()
            } else if !trimmed.is_empty() {
                if let Some(first_char) = trimmed.chars().next() {
                    if first_char.is_ascii_digit() {
                        if let Some(dot_pos) = trimmed.find(". ") {
                            return trimmed.chars().skip(dot_pos + 2).collect();
                        }
                    }
                }
                line.to_string()
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

fn filesizeformat_filter(value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
    let bytes = value
        .as_i64()
        .or_else(|| value.as_u64().map(|v| v as i64))
        .or_else(|| value.as_f64().map(|v| v as i64))
        .ok_or_else(|| tera::Error::msg("filesizeformat requires a number"))?;

    let units = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_idx = 0;

    while size >= 1024.0 && unit_idx < units.len() - 1 {
        size /= 1024.0;
        unit_idx += 1;
    }

    let formatted = if unit_idx == 0 {
        format!("{} {}", bytes, units[unit_idx])
    } else {
        format!("{:.1} {}", size, units[unit_idx])
    };

    Ok(Value::String(formatted))
}
