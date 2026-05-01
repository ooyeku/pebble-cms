use crate::services::{search, tags};
use crate::web::error::AppResult;
use crate::web::extractors::CurrentUser;
use crate::web::state::AppState;
use axum::extract::{Query, State};
use axum::response::Html;
use axum::Form;
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

#[derive(Deserialize)]
pub struct PreviewForm {
    body_markdown: String,
}

pub async fn preview(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user): CurrentUser,
    Form(form): Form<PreviewForm>,
) -> AppResult<Html<String>> {
    let html = state.markdown.render(&form.body_markdown);

    let mut ctx = Context::new();
    ctx.insert("html", &html);

    let rendered = state.templates.render("htmx/preview.html", &ctx)?;
    Ok(Html(rendered))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> AppResult<Html<String>> {
    let results = match &query.q {
        Some(q) if !q.is_empty() => search::search_content(&state.db, q, 10)?,
        _ => vec![],
    };

    let mut html = String::from("<ul class=\"search-results\">");
    for result in results {
        html.push_str(&format!(
            r#"<li><a href="/posts/{}">{}</a></li>"#,
            escape_html(&result.slug),
            escape_html(&result.title)
        ));
    }
    html.push_str("</ul>");

    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct TagQuery {
    q: Option<String>,
}

pub async fn tag_autocomplete(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user): CurrentUser,
    Query(query): Query<TagQuery>,
) -> AppResult<Html<String>> {
    let all_tags = tags::list_tags(&state.db)?;

    let filtered: Vec<_> = match &query.q {
        Some(q) if !q.is_empty() => {
            let q_lower = q.to_lowercase();
            all_tags
                .into_iter()
                .filter(|t| t.name.to_lowercase().contains(&q_lower))
                .take(10)
                .collect()
        }
        _ => all_tags.into_iter().take(10).collect(),
    };

    let mut html = String::from("<ul class=\"tag-suggestions\">");
    for tag in filtered {
        html.push_str(&format!(
            r#"<li><button type="button" class="tag-suggestion" data-tag="{}">{}</button></li>"#,
            escape_html(&tag.name),
            escape_html(&tag.name)
        ));
    }
    html.push_str("</ul>");

    Ok(Html(html))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::escape_html;

    #[test]
    fn escape_html_escapes_text_and_attribute_delimiters() {
        assert_eq!(
            escape_html(r#"<script>"&'</script>"#),
            "&lt;script&gt;&quot;&amp;&#39;&lt;/script&gt;"
        );
    }
}
