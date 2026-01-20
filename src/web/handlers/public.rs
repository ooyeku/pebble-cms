use crate::models::{ContentType, User};
use crate::services::{content, search, tags};
use crate::web::error::AppResult;
use crate::web::extractors::OptionalUser;
use crate::web::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

fn make_context(state: &AppState, user: &Option<User>) -> Context {
    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("theme", &state.config.theme);
    ctx.insert("user", user);
    ctx.insert("production_mode", &state.production_mode);
    ctx
}

const MAX_PAGE: usize = 10000;

#[derive(Deserialize)]
pub struct Pagination {
    #[serde(default = "default_page")]
    page: usize,
}

fn default_page() -> usize {
    1
}

fn clamp_page(page: usize) -> usize {
    page.max(1).min(MAX_PAGE)
}

pub async fn index(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
) -> AppResult<Html<String>> {
    let posts = content::list_published_content(
        &state.db,
        ContentType::Post,
        state.config.content.posts_per_page,
        0,
    )?;

    let mut ctx = make_context(&state, &user);
    ctx.insert("posts", &posts);

    let html = state.templates.render("public/index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn posts(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
    Query(pagination): Query<Pagination>,
) -> AppResult<Html<String>> {
    let per_page = state.config.content.posts_per_page;
    let page = clamp_page(pagination.page);
    let offset = (page - 1) * per_page;
    let posts = content::list_published_content(&state.db, ContentType::Post, per_page, offset)?;
    let total = content::count_content(
        &state.db,
        Some(ContentType::Post),
        Some(crate::models::ContentStatus::Published),
    )?;
    let total_pages = (total as usize + per_page - 1) / per_page;

    let mut ctx = make_context(&state, &user);
    ctx.insert("posts", &posts);
    ctx.insert("page", &page);
    ctx.insert("total_pages", &total_pages);

    let html = state.templates.render("public/index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn post(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
    Path(slug): Path<String>,
) -> AppResult<Response> {
    let post = content::get_content_by_slug(&state.db, &slug)?;

    match post {
        Some(p)
            if p.content.content_type == ContentType::Post
                && p.content.status == crate::models::ContentStatus::Published =>
        {
            let mut ctx = make_context(&state, &user);
            ctx.insert("content", &p);

            let html = state.templates.render("public/post.html", &ctx)?;
            Ok(Html(html).into_response())
        }
        _ => {
            let ctx = make_context(&state, &user);
            let html = state.templates.render("public/404.html", &ctx)?;
            Ok((StatusCode::NOT_FOUND, Html(html)).into_response())
        }
    }
}

pub async fn page(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
    Path(slug): Path<String>,
) -> AppResult<Response> {
    let page = content::get_content_by_slug(&state.db, &slug)?;

    match page {
        Some(p)
            if p.content.content_type == ContentType::Page
                && p.content.status == crate::models::ContentStatus::Published =>
        {
            let mut ctx = make_context(&state, &user);
            ctx.insert("content", &p);

            let html = state.templates.render("public/page.html", &ctx)?;
            Ok(Html(html).into_response())
        }
        _ => {
            let ctx = make_context(&state, &user);
            let html = state.templates.render("public/404.html", &ctx)?;
            Ok((StatusCode::NOT_FOUND, Html(html)).into_response())
        }
    }
}

pub async fn tags_page(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
) -> AppResult<Html<String>> {
    let tags_list = tags::list_tags_with_counts(&state.db)?;

    let mut ctx = make_context(&state, &user);
    ctx.insert("tags", &tags_list);

    let html = state.templates.render("public/tags.html", &ctx)?;
    Ok(Html(html))
}

pub async fn tags(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
) -> AppResult<Html<String>> {
    tags_page(State(state), OptionalUser(user)).await
}

pub async fn tag(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
    Path(slug): Path<String>,
) -> AppResult<Response> {
    let tag = tags::get_tag_by_slug(&state.db, &slug)?;

    match tag {
        Some(t) => {
            let conn = state.db.get()?;
            let mut stmt = conn.prepare(
                r#"
                SELECT c.id, c.slug, c.title, c.content_type, c.body_markdown, c.body_html, c.excerpt, c.featured_image, c.status, c.published_at, c.author_id, c.metadata, c.created_at, c.updated_at
                FROM content c
                JOIN content_tags ct ON c.id = ct.content_id
                WHERE ct.tag_id = ? AND c.status = 'published'
                ORDER BY c.published_at DESC
                "#,
            )?;

            let posts: Vec<crate::models::ContentWithTags> = stmt
                .query_map([t.id], |row| {
                    Ok(crate::models::Content {
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
                        status: row.get::<_, String>(8)?.parse().unwrap_or_default(),
                        published_at: row.get(9)?,
                        author_id: row.get(10)?,
                        metadata: serde_json::from_str(&row.get::<_, String>(11)?)
                            .unwrap_or_default(),
                        created_at: row.get(12)?,
                        updated_at: row.get(13)?,
                    })
                })?
                .filter_map(|c| c.ok())
                .map(|c| crate::models::ContentWithTags {
                    content: c,
                    tags: vec![],
                    author: None,
                })
                .collect();

            let mut ctx = make_context(&state, &user);
            ctx.insert("tag", &t);
            ctx.insert("posts", &posts);

            let html = state.templates.render("public/tag.html", &ctx)?;
            Ok(Html(html).into_response())
        }
        None => {
            let ctx = make_context(&state, &user);
            let html = state.templates.render("public/404.html", &ctx)?;
            Ok((StatusCode::NOT_FOUND, Html(html)).into_response())
        }
    }
}

#[derive(Deserialize)]
pub struct SearchQuery {
    q: Option<String>,
}

pub async fn search(
    State(state): State<Arc<AppState>>,
    OptionalUser(user): OptionalUser,
    Query(query): Query<SearchQuery>,
) -> AppResult<Html<String>> {
    let results = match &query.q {
        Some(q) if !q.is_empty() => search::search_content(&state.db, q, 50)?,
        _ => vec![],
    };

    let mut ctx = make_context(&state, &user);
    ctx.insert("query", &query.q.clone().unwrap_or_default());
    ctx.insert("results", &results);

    let html = state.templates.render("public/search.html", &ctx)?;
    Ok(Html(html))
}

pub async fn rss_feed(State(state): State<Arc<AppState>>) -> AppResult<Response> {
    let posts = content::list_published_content(&state.db, ContentType::Post, 20, 0)?;
    let site = &state.config.site;

    let mut items = String::new();
    for post in posts {
        items.push_str(&format!(
            r#"
    <item>
      <title>{}</title>
      <link>{}/posts/{}</link>
      <description><![CDATA[{}]]></description>
      <pubDate>{}</pubDate>
      <guid>{}/posts/{}</guid>
    </item>"#,
            html_escape(&post.content.title),
            site.url,
            post.content.slug,
            post.content.excerpt.as_deref().unwrap_or(""),
            post.content.published_at.as_deref().unwrap_or(""),
            site.url,
            post.content.slug
        ));
    }

    let rss = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{}</title>
    <link>{}</link>
    <description>{}</description>
    <language>{}</language>
    <atom:link href="{}/feed.xml" rel="self" type="application/rss+xml"/>
    {}
  </channel>
</rss>"#,
        html_escape(&site.title),
        site.url,
        html_escape(&site.description),
        site.language,
        site.url,
        items
    );

    Ok((
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        rss,
    )
        .into_response())
}

pub async fn serve_media(
    State(state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> AppResult<Response> {
    // Prevent path traversal attacks
    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let file_path = state.media_dir.join(&filename);

    // Ensure the resolved path is still within media_dir
    let canonical_media = state.media_dir.canonicalize().unwrap_or_default();
    let canonical_file = match file_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return Ok(StatusCode::NOT_FOUND.into_response()),
    };

    if !canonical_file.starts_with(&canonical_media) {
        return Ok(StatusCode::NOT_FOUND.into_response());
    }

    let content = tokio::fs::read(&file_path).await?;
    let mime = mime_guess::from_path(&filename).first_or_octet_stream();

    Ok(([(header::CONTENT_TYPE, mime.as_ref())], content).into_response())
}

pub async fn json_feed(State(state): State<Arc<AppState>>) -> AppResult<Response> {
    let posts = content::list_published_content(&state.db, ContentType::Post, 20, 0)?;
    let site = &state.config.site;

    let items: Vec<serde_json::Value> = posts
        .iter()
        .map(|post| {
            serde_json::json!({
                "id": format!("{}/posts/{}", site.url, post.content.slug),
                "url": format!("{}/posts/{}", site.url, post.content.slug),
                "title": post.content.title,
                "content_html": post.content.body_html,
                "summary": post.content.excerpt,
                "date_published": post.content.published_at,
                "authors": post.author.as_ref().map(|a| vec![serde_json::json!({"name": a.username})]).unwrap_or_default(),
                "tags": post.tags.iter().map(|t| &t.name).collect::<Vec<_>>()
            })
        })
        .collect();

    let feed = serde_json::json!({
        "version": "https://jsonfeed.org/version/1.1",
        "title": site.title,
        "home_page_url": site.url,
        "feed_url": format!("{}/feed.json", site.url),
        "description": site.description,
        "language": site.language,
        "items": items
    });

    Ok((
        [(header::CONTENT_TYPE, "application/feed+json; charset=utf-8")],
        serde_json::to_string_pretty(&feed).unwrap_or_default(),
    )
        .into_response())
}

pub async fn sitemap(State(state): State<Arc<AppState>>) -> AppResult<Response> {
    let posts = content::list_published_content(&state.db, ContentType::Post, 1000, 0)?;
    let pages = content::list_published_content(&state.db, ContentType::Page, 100, 0)?;
    let tags_list = tags::list_tags_with_counts(&state.db)?;
    let site = &state.config.site;

    let mut urls = String::new();

    urls.push_str(&format!(
        r#"  <url>
    <loc>{}</loc>
    <changefreq>daily</changefreq>
    <priority>1.0</priority>
  </url>
"#,
        site.url
    ));

    for post in posts {
        urls.push_str(&format!(
            r#"  <url>
    <loc>{}/posts/{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
"#,
            site.url,
            post.content.slug,
            post.content
                .updated_at
                .split('T')
                .next()
                .unwrap_or(&post.content.updated_at)
        ));
    }

    for page in pages {
        urls.push_str(&format!(
            r#"  <url>
    <loc>{}/pages/{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>monthly</changefreq>
    <priority>0.6</priority>
  </url>
"#,
            site.url,
            page.content.slug,
            page.content
                .updated_at
                .split('T')
                .next()
                .unwrap_or(&page.content.updated_at)
        ));
    }

    for tag in tags_list {
        urls.push_str(&format!(
            r#"  <url>
    <loc>{}/tags/{}</loc>
    <changefreq>weekly</changefreq>
    <priority>0.5</priority>
  </url>
"#,
            site.url, tag.tag.slug
        ));
    }

    let sitemap = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{}
</urlset>"#,
        urls
    );

    Ok((
        [(header::CONTENT_TYPE, "application/xml; charset=utf-8")],
        sitemap,
    )
        .into_response())
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
