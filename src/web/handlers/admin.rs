// Role guards return a full `Response` as the error so handlers can `?` them;
// boxing it would only add noise at every call site.
#![allow(clippy::result_large_err)]

use crate::models::{
    ContentStatus, ContentType, ContentWithTags, CreateContent, UpdateContent, User, UserRole,
};
use crate::services::audit::{AuditAction, AuditCategory, AuditLogBuilder};
use crate::services::{
    api_token, audit, auth, content, database, media, preview, series, settings, tags, webhook,
};
use crate::web::error::AppResult;
use crate::web::extractors::{AuditInfo, CurrentUser, HxRequest};
use crate::web::state::AppState;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

fn make_admin_context(state: &AppState, user: &User) -> Context {
    let config = state.config();
    let mut ctx = Context::new();
    ctx.insert("site", &config.site);
    ctx.insert("user", user);
    ctx.insert("theme", &config.theme);
    ctx.insert("version", env!("CARGO_PKG_VERSION"));
    if config.theme.custom.has_customizations() {
        ctx.insert("theme_custom_css", &config.theme.custom.to_css_variables());
    }
    ctx
}

fn require_admin(user: &User) -> Result<(), Response> {
    if user.role != UserRole::Admin {
        Err((StatusCode::FORBIDDEN, "Admin access required").into_response())
    } else {
        Ok(())
    }
}

fn require_author_or_admin(user: &User) -> Result<(), Response> {
    if user.role == UserRole::Viewer {
        Err((StatusCode::FORBIDDEN, "Author or admin access required").into_response())
    } else {
        Ok(())
    }
}

fn require_content_owner_or_admin(user: &User, content: &ContentWithTags) -> Result<(), Response> {
    match user.role {
        UserRole::Admin => Ok(()),
        UserRole::Author if content.content.author_id == Some(user.id) => Ok(()),
        UserRole::Author => Err((
            StatusCode::FORBIDDEN,
            "You can only manage your own content",
        )
            .into_response()),
        UserRole::Viewer => {
            Err((StatusCode::FORBIDDEN, "Author or admin access required").into_response())
        }
    }
}

fn require_allowed_role_change(
    current_user: &User,
    target_user: &User,
    new_role: UserRole,
    admin_count: usize,
) -> Result<(), Response> {
    if current_user.id == target_user.id && new_role != UserRole::Admin {
        return Err((StatusCode::BAD_REQUEST, "Cannot remove your own admin role").into_response());
    }

    if target_user.role == UserRole::Admin && new_role != UserRole::Admin && admin_count <= 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Cannot demote the last admin account",
        )
            .into_response());
    }

    Ok(())
}

const MAX_ADMIN_PAGE: usize = 10_000;

fn content_author_filter(user: &User) -> Option<i64> {
    if user.role == UserRole::Author {
        Some(user.id)
    } else {
        None
    }
}

fn admin_page_offset(page: usize, per_page: usize) -> (usize, usize) {
    let page = page.clamp(1, MAX_ADMIN_PAGE);
    let offset = page.saturating_sub(1).saturating_mul(per_page);
    (page, offset)
}

pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let author_id = content_author_filter(&user);
    let recent_posts = content::list_content_for_author(
        &state.db,
        Some(ContentType::Post),
        None,
        author_id,
        5,
        0,
    )?;
    let post_count =
        content::count_content_for_author(&state.db, Some(ContentType::Post), None, author_id)?;
    let page_count =
        content::count_content_for_author(&state.db, Some(ContentType::Page), None, author_id)?;
    let published_count = content::count_content_for_author(
        &state.db,
        None,
        Some(ContentStatus::Published),
        author_id,
    )?;
    let snippet_count =
        content::count_content_for_author(&state.db, Some(ContentType::Snippet), None, author_id)?;
    let series_count = if user.role == UserRole::Admin {
        series::list_series(&state.db, 1000, 0)
            .map(|s| s.len() as i64)
            .unwrap_or(0)
    } else {
        0
    };

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("recent_posts", &recent_posts);
    ctx.insert("post_count", &post_count);
    ctx.insert("page_count", &page_count);
    ctx.insert("published_count", &published_count);
    ctx.insert("snippet_count", &snippet_count);
    ctx.insert("series_count", &series_count);

    let html = state.templates.render("admin/dashboard.html", &ctx)?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct AdminPagination {
    #[serde(default = "default_admin_page")]
    page: usize,
}

fn default_admin_page() -> usize {
    1
}

pub async fn posts(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Query(pagination): Query<AdminPagination>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let per_page = 50;
    let (page, offset) = admin_page_offset(pagination.page, per_page);
    let author_id = content_author_filter(&user);
    let posts = content::list_content_for_author(
        &state.db,
        Some(ContentType::Post),
        None,
        author_id,
        per_page,
        offset,
    )?;
    let total =
        content::count_content_for_author(&state.db, Some(ContentType::Post), None, author_id)?;
    let total_pages = (total as usize).div_ceil(per_page);

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("posts", &posts);
    ctx.insert("page", &page);
    ctx.insert("total_pages", &total_pages);

    let html = state.templates.render("admin/posts/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn new_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let all_tags = tags::list_tags(&state.db)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &Option::<crate::models::ContentWithTags>::None);
    ctx.insert("all_tags", &all_tags);
    ctx.insert("is_new", &true);
    ctx.insert("content_type", "post");

    let html = state.templates.render("admin/posts/form.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct ContentForm {
    title: String,
    slug: Option<String>,
    body_markdown: String,
    excerpt: Option<String>,
    status: String,
    scheduled_at: Option<String>,
    #[serde(default)]
    tags: String,
    // SEO fields
    meta_title: Option<String>,
    meta_description: Option<String>,
    canonical_url: Option<String>,
    // Custom code fields (for pages)
    #[serde(default)]
    custom_html: Option<String>,
    #[serde(default)]
    custom_css: Option<String>,
    #[serde(default)]
    custom_js: Option<String>,
    #[serde(default)]
    use_custom_code: Option<String>,
}

fn metadata_string_value(value: Option<&String>) -> serde_json::Value {
    match value.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        Some(value) => serde_json::json!(value),
        None => serde_json::Value::Null,
    }
}

fn build_seo_metadata(form: &ContentForm) -> serde_json::Value {
    let mut metadata = serde_json::json!({});
    metadata["meta_title"] = metadata_string_value(form.meta_title.as_ref());
    metadata["meta_description"] = metadata_string_value(form.meta_description.as_ref());
    metadata["canonical_url"] = metadata_string_value(form.canonical_url.as_ref());
    metadata
}

fn build_page_metadata(form: &ContentForm) -> serde_json::Value {
    let mut metadata = build_seo_metadata(form);

    metadata["custom_html"] = metadata_string_value(form.custom_html.as_ref());
    metadata["custom_css"] = metadata_string_value(form.custom_css.as_ref());
    metadata["custom_js"] = metadata_string_value(form.custom_js.as_ref());
    // use_custom_code: "only" = only custom code, empty/none = markdown only
    metadata["use_custom_code"] = metadata_string_value(form.use_custom_code.as_ref());

    metadata
}

pub async fn create_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    HxRequest(_is_htmx): HxRequest,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let tags: Vec<String> = form
        .tags
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let input = CreateContent {
        title: form.title.clone(),
        slug: form.slug.clone().filter(|s| !s.is_empty()),
        content_type: ContentType::Post,
        body_markdown: form.body_markdown.clone(),
        excerpt: form.excerpt.clone().filter(|s| !s.is_empty()),
        featured_image: None,
        status: form.status.parse().unwrap_or(ContentStatus::Draft),
        scheduled_at: form.scheduled_at.clone().filter(|s| !s.is_empty()),
        tags,
        metadata: Some(build_seo_metadata(&form)),
    };

    let content_id = content::create_content(
        &state.db,
        input,
        Some(user.id),
        state.config().content.excerpt_length,
    )?;

    // Fire webhooks
    if form.status == "published" {
        webhook::fire_webhooks(
            &state.db,
            "content.published",
            serde_json::json!({ "id": content_id, "title": form.title, "type": "post" }),
        );
    }

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Create, AuditCategory::Content).entity(
            "post",
            content_id,
            Some(&form.title),
        ),
    );

    Ok(Redirect::to("/admin/posts").into_response())
}

pub async fn edit_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let post = content::get_content_by_id(&state.db, id)?;

    match post {
        Some(p) if p.content.content_type == ContentType::Post => {
            if let Err(e) = require_content_owner_or_admin(&user, &p) {
                return Ok(e);
            }

            let all_tags = tags::list_tags(&state.db)?;

            let mut ctx = make_admin_context(&state, &user);
            ctx.insert("content", &p);
            ctx.insert("all_tags", &all_tags);
            ctx.insert("is_new", &false);
            ctx.insert("content_type", "post");

            let html = state.templates.render("admin/posts/form.html", &ctx)?;
            Ok(Html(html).into_response())
        }
        _ => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

pub async fn update_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    HxRequest(_is_htmx): HxRequest,
    Path(id): Path<i64>,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let post = content::get_content_by_id(&state.db, id)?;
    match post {
        Some(ref p) if p.content.content_type == ContentType::Post => {
            if let Err(e) = require_content_owner_or_admin(&user, p) {
                return Ok(e);
            }
        }
        _ => return Ok(StatusCode::NOT_FOUND.into_response()),
    }

    let tags: Vec<String> = form
        .tags
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    let input = UpdateContent {
        title: Some(form.title.clone()),
        slug: form.slug.clone().filter(|s| !s.is_empty()),
        body_markdown: Some(form.body_markdown.clone()),
        excerpt: form.excerpt.clone(),
        featured_image: None,
        status: Some(form.status.parse().unwrap_or(ContentStatus::Draft)),
        scheduled_at: form.scheduled_at.clone().filter(|s| !s.is_empty()),
        tags: Some(tags),
        metadata: Some(build_seo_metadata(&form)),
    };

    let config = state.config();
    content::update_content(
        &state.db,
        id,
        input,
        config.content.excerpt_length,
        Some(user.id),
        config.content.version_retention,
    )?;

    // Fire webhooks
    webhook::fire_webhooks(
        &state.db,
        "content.updated",
        serde_json::json!({ "id": id, "title": form.title, "type": "post" }),
    );

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Update, AuditCategory::Content).entity(
            "post",
            id,
            Some(&form.title),
        ),
    );

    Ok(Redirect::to("/admin/posts").into_response())
}

pub async fn delete_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let post = content::get_content_by_id(&state.db, id)?;
    let title = match post {
        Some(p) if p.content.content_type == ContentType::Post => {
            if let Err(e) = require_content_owner_or_admin(&user, &p) {
                return Ok(e);
            }
            p.content.title
        }
        _ => return Ok(StatusCode::NOT_FOUND.into_response()),
    };

    content::delete_content(&state.db, id)?;

    // Fire webhooks
    webhook::fire_webhooks(
        &state.db,
        "content.deleted",
        serde_json::json!({ "id": id, "title": title, "type": "post" }),
    );

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Delete, AuditCategory::Content).entity(
            "post",
            id,
            Some(&title),
        ),
    );

    if is_htmx {
        Ok((
            [(
                header::HeaderName::from_static("hx-redirect"),
                "/admin/posts".to_string(),
            )],
            "",
        )
            .into_response())
    } else {
        Ok(Redirect::to("/admin/posts").into_response())
    }
}

pub async fn pages(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Query(pagination): Query<AdminPagination>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let per_page = 50;
    let (page, offset) = admin_page_offset(pagination.page, per_page);
    let author_id = content_author_filter(&user);
    let pages = content::list_content_for_author(
        &state.db,
        Some(ContentType::Page),
        None,
        author_id,
        per_page,
        offset,
    )?;
    let total =
        content::count_content_for_author(&state.db, Some(ContentType::Page), None, author_id)?;
    let total_pages = (total as usize).div_ceil(per_page);

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("pages", &pages);
    ctx.insert("page", &page);
    ctx.insert("total_pages", &total_pages);

    let html = state.templates.render("admin/pages/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn new_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &Option::<crate::models::ContentWithTags>::None);
    ctx.insert("is_new", &true);
    ctx.insert("content_type", "page");

    let html = state.templates.render("admin/pages/form.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn create_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    HxRequest(is_htmx): HxRequest,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let input = CreateContent {
        title: form.title.clone(),
        slug: form.slug.clone().filter(|s| !s.is_empty()),
        content_type: ContentType::Page,
        body_markdown: form.body_markdown.clone(),
        excerpt: form.excerpt.clone().filter(|s| !s.is_empty()),
        featured_image: None,
        status: form.status.parse().unwrap_or(ContentStatus::Draft),
        scheduled_at: form.scheduled_at.clone().filter(|s| !s.is_empty()),
        tags: vec![],
        metadata: Some(build_page_metadata(&form)),
    };

    let id = content::create_content(
        &state.db,
        input,
        Some(user.id),
        state.config().content.excerpt_length,
    )?;

    // Fire webhooks
    if form.status == "published" {
        webhook::fire_webhooks(
            &state.db,
            "content.published",
            serde_json::json!({ "id": id, "title": form.title, "type": "page" }),
        );
    }

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Create, AuditCategory::Content).entity(
            "page",
            id,
            Some(&form.title),
        ),
    );

    if is_htmx {
        Ok((
            [(
                header::HeaderName::from_static("hx-redirect"),
                format!("/admin/pages/{}/edit", id),
            )],
            "",
        )
            .into_response())
    } else {
        Ok(Redirect::to(&format!("/admin/pages/{}/edit", id)).into_response())
    }
}

pub async fn edit_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let page = content::get_content_by_id(&state.db, id)?;

    match page {
        Some(p) if p.content.content_type == ContentType::Page => {
            if let Err(e) = require_content_owner_or_admin(&user, &p) {
                return Ok(e);
            }

            let mut ctx = make_admin_context(&state, &user);
            ctx.insert("content", &p);
            ctx.insert("is_new", &false);
            ctx.insert("content_type", "page");

            let html = state.templates.render("admin/pages/form.html", &ctx)?;
            Ok(Html(html).into_response())
        }
        _ => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

pub async fn update_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let page = content::get_content_by_id(&state.db, id)?;
    match page {
        Some(ref p) if p.content.content_type == ContentType::Page => {
            if let Err(e) = require_content_owner_or_admin(&user, p) {
                return Ok(e);
            }
        }
        _ => return Ok(StatusCode::NOT_FOUND.into_response()),
    }

    let input = UpdateContent {
        title: Some(form.title.clone()),
        slug: form.slug.clone().filter(|s| !s.is_empty()),
        body_markdown: Some(form.body_markdown.clone()),
        excerpt: form.excerpt.clone(),
        featured_image: None,
        status: Some(form.status.parse().unwrap_or(ContentStatus::Draft)),
        scheduled_at: form.scheduled_at.clone().filter(|s| !s.is_empty()),
        tags: None,
        metadata: Some(build_page_metadata(&form)),
    };

    let config = state.config();
    content::update_content(
        &state.db,
        id,
        input,
        config.content.excerpt_length,
        Some(user.id),
        config.content.version_retention,
    )?;

    // Fire webhooks
    webhook::fire_webhooks(
        &state.db,
        "content.updated",
        serde_json::json!({ "id": id, "title": form.title, "type": "page" }),
    );

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Update, AuditCategory::Content).entity(
            "page",
            id,
            Some(&form.title),
        ),
    );

    if is_htmx {
        let mut ctx = Context::new();
        ctx.insert("message", "Page saved successfully");
        ctx.insert("type", "success");
        let html = state.templates.render("htmx/flash.html", &ctx)?;
        Ok(Html(html).into_response())
    } else {
        Ok(Redirect::to(&format!("/admin/pages/{}/edit", id)).into_response())
    }
}

pub async fn delete_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let page = content::get_content_by_id(&state.db, id)?;
    let title = match page {
        Some(p) if p.content.content_type == ContentType::Page => {
            if let Err(e) = require_content_owner_or_admin(&user, &p) {
                return Ok(e);
            }
            p.content.title
        }
        _ => return Ok(StatusCode::NOT_FOUND.into_response()),
    };

    content::delete_content(&state.db, id)?;

    // Fire webhooks
    webhook::fire_webhooks(
        &state.db,
        "content.deleted",
        serde_json::json!({ "id": id, "title": title, "type": "page" }),
    );

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Delete, AuditCategory::Content).entity(
            "page",
            id,
            Some(&title),
        ),
    );

    if is_htmx {
        Ok((
            [(
                header::HeaderName::from_static("hx-redirect"),
                "/admin/pages".to_string(),
            )],
            "",
        )
            .into_response())
    } else {
        Ok(Redirect::to("/admin/pages").into_response())
    }
}

pub async fn media_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let media_list = media::list_media(&state.db, 100, 0)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("media", &media_list);

    let html = state.templates.render("admin/media/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn media(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    media_page(State(state), CurrentUser(user)).await
}

pub async fn upload_media(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    mut multipart: Multipart,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let rate_key = format!("upload:{}", user.id);
    let max_upload = state.config().media.max_upload_bytes();

    while let Some(mut field) = multipart.next_field().await? {
        if !state.upload_rate_limiter.check(&rate_key) {
            return Ok((
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                "Too many uploads. Please wait before uploading more files.",
            )
                .into_response());
        }
        state.upload_rate_limiter.record_attempt(&rate_key);

        let name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let mut data = Vec::new();

        while let Some(chunk) = field.chunk().await? {
            if data.len().saturating_add(chunk.len()) > max_upload {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    format!("File '{}' exceeds the maximum upload size", name),
                )
                    .into_response());
            }
            data.extend_from_slice(&chunk);
        }

        if data.len() > max_upload {
            return Ok((
                StatusCode::BAD_REQUEST,
                format!("File '{}' exceeds the maximum upload size", name),
            )
                .into_response());
        }

        media::upload_media(
            &state.db,
            &state.media_dir,
            &name,
            &content_type,
            &data,
            Some(user.id),
        )?;

        // Fire webhooks
        webhook::fire_webhooks(
            &state.db,
            "media.uploaded",
            serde_json::json!({ "filename": name, "content_type": content_type }),
        );
    }

    Ok(Redirect::to("/admin/media").into_response())
}

pub async fn delete_media(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    media::delete_media(&state.db, &state.media_dir, id)?;

    // Fire webhooks
    webhook::fire_webhooks(&state.db, "media.deleted", serde_json::json!({ "id": id }));

    if is_htmx {
        Ok((
            [(
                header::HeaderName::from_static("hx-redirect"),
                "/admin/media".to_string(),
            )],
            "",
        )
            .into_response())
    } else {
        Ok(Redirect::to("/admin/media").into_response())
    }
}

pub async fn tags_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let tags_list = tags::list_tags_with_counts(&state.db)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("tags", &tags_list);

    let html = state.templates.render("admin/tags/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn tags(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    tags_page(State(state), CurrentUser(user)).await
}

#[derive(Deserialize)]
pub struct TagForm {
    name: String,
}

pub async fn create_tag(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<TagForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    tags::create_tag(&state.db, &form.name, None)?;
    Ok(Redirect::to("/admin/tags").into_response())
}

pub async fn delete_tag(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    tags::delete_tag(&state.db, id)?;

    if is_htmx {
        Ok((
            [(
                header::HeaderName::from_static("hx-redirect"),
                "/admin/tags".to_string(),
            )],
            "",
        )
            .into_response())
    } else {
        Ok(Redirect::to("/admin/tags").into_response())
    }
}

pub async fn settings(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let homepage_settings = settings::get_homepage_settings(&state.db).unwrap_or_default();
    let config = state.config();

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("config", &*config);
    ctx.insert("homepage", &homepage_settings);
    ctx.insert(
        "available_themes",
        &crate::config::ThemeConfig::AVAILABLE_THEMES,
    );

    let html = state.templates.render("admin/settings/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct HomepageSettingsForm {
    homepage_title: String,
    homepage_subtitle: String,
    #[serde(default)]
    show_pages: Option<String>,
    #[serde(default)]
    show_posts: Option<String>,
    custom_content: String,
}

pub async fn save_homepage_settings(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<HomepageSettingsForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let homepage = settings::HomepageSettings {
        title: form.homepage_title,
        subtitle: form.homepage_subtitle,
        show_pages: form.show_pages.as_ref().is_some_and(|v| !v.is_empty()),
        show_posts: form.show_posts.as_ref().is_some_and(|v| !v.is_empty()),
        custom_content: form.custom_content,
    };

    settings::save_homepage_settings(&state.db, &homepage)?;

    Ok(Redirect::to("/admin/settings").into_response())
}

#[derive(Deserialize)]
pub struct SiteSettingsForm {
    // Site
    site_title: String,
    site_description: String,
    site_url: String,
    site_language: String,
    // Content
    posts_per_page: usize,
    excerpt_length: usize,
    #[serde(default)]
    auto_excerpt: Option<String>,
    // Theme
    theme_name: String,
    #[serde(default)]
    theme_primary_color: Option<String>,
    #[serde(default)]
    theme_accent_color: Option<String>,
    #[serde(default)]
    theme_background_color: Option<String>,
    #[serde(default)]
    theme_text_color: Option<String>,
    // Homepage
    #[serde(default)]
    homepage_show_hero: Option<String>,
    homepage_hero_layout: String,
    homepage_hero_height: String,
    homepage_hero_text_align: String,
    #[serde(default)]
    homepage_show_posts: Option<String>,
    homepage_posts_layout: String,
    homepage_posts_columns: u8,
    #[serde(default)]
    homepage_show_pages: Option<String>,
    homepage_pages_layout: String,
}

pub async fn save_settings(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<SiteSettingsForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    // Get current config and update it
    let current = state.config();

    let new_config = crate::Config {
        site: crate::config::SiteConfig {
            title: form.site_title,
            description: form.site_description,
            url: form.site_url,
            language: form.site_language,
        },
        server: current.server.clone(),
        database: current.database.clone(),
        content: crate::config::ContentConfig {
            posts_per_page: form.posts_per_page.clamp(1, 100),
            excerpt_length: form.excerpt_length.clamp(1, 10000),
            auto_excerpt: form.auto_excerpt.is_some(),
            version_retention: current.content.version_retention,
        },
        media: current.media.clone(),
        theme: crate::config::ThemeConfig {
            name: form.theme_name,
            custom: crate::config::CustomThemeOptions {
                primary_color: form.theme_primary_color.filter(|s| !s.is_empty()),
                accent_color: form.theme_accent_color.filter(|s| !s.is_empty()),
                background_color: form.theme_background_color.filter(|s| !s.is_empty()),
                text_color: form.theme_text_color.filter(|s| !s.is_empty()),
                ..current.theme.custom.clone()
            },
        },
        auth: current.auth.clone(),
        homepage: crate::config::HomepageConfig {
            show_hero: form.homepage_show_hero.is_some(),
            hero_layout: form.homepage_hero_layout,
            hero_height: form.homepage_hero_height,
            hero_text_align: form.homepage_hero_text_align,
            hero_image: current.homepage.hero_image.clone(),
            show_posts: form.homepage_show_posts.is_some(),
            posts_layout: form.homepage_posts_layout,
            posts_columns: form.homepage_posts_columns,
            show_pages: form.homepage_show_pages.is_some(),
            pages_layout: form.homepage_pages_layout,
            sections_order: current.homepage.sections_order.clone(),
        },
        audit: current.audit.clone(),
        api: current.api.clone(),
        backup: current.backup.clone(),
    };

    // Drop the read lock before updating
    drop(current);

    // Update config (writes to file and updates in-memory)
    if let Err(e) = state.update_config(new_config) {
        let mut ctx = make_admin_context(&state, &user);
        ctx.insert("error", &e.to_string());
        ctx.insert("config", &*state.config());
        ctx.insert(
            "homepage",
            &settings::get_homepage_settings(&state.db).unwrap_or_default(),
        );
        ctx.insert(
            "available_themes",
            &crate::config::ThemeConfig::AVAILABLE_THEMES,
        );
        let html = state.templates.render("admin/settings/index.html", &ctx)?;
        return Ok((StatusCode::BAD_REQUEST, Html(html)).into_response());
    }

    Ok(Redirect::to("/admin/settings").into_response())
}

pub async fn users(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let users_list = auth::list_users(&state.db)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("users", &users_list);

    let html = state.templates.render("admin/users/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct CreateUserForm {
    username: String,
    email: String,
    password: String,
    role: String,
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Form(form): Form<CreateUserForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let render_with_error =
        |state: &Arc<AppState>, user: &User, error: &str| -> AppResult<Response> {
            let users_list = auth::list_users(&state.db)?;
            let mut ctx = make_admin_context(state, user);
            ctx.insert("users", &users_list);
            ctx.insert("error", error);
            let html = state.templates.render("admin/users/index.html", &ctx)?;
            Ok((StatusCode::BAD_REQUEST, Html(html)).into_response())
        };

    if let Err(e) = auth::validate_username(&form.username) {
        return render_with_error(&state, &user, &e.to_string());
    }

    if let Err(e) = auth::validate_email(&form.email) {
        return render_with_error(&state, &user, &e.to_string());
    }

    if let Err(e) = auth::validate_password(&form.password) {
        return render_with_error(&state, &user, &e.to_string());
    }

    let role: UserRole = form.role.parse().unwrap_or(UserRole::Author);
    let new_user_id =
        match auth::create_user(&state.db, &form.username, &form.email, &form.password, role) {
            Ok(id) => id,
            Err(e) => {
                let msg = e.to_string();
                let display = if msg.contains("UNIQUE constraint failed") {
                    "A user with that username or email already exists".to_string()
                } else {
                    "Could not create user. Please check your input and try again.".to_string()
                };
                return render_with_error(&state, &user, &display);
            }
        };

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::UserCreate, AuditCategory::User)
            .entity("user", new_user_id, Some(&form.username))
            .metadata_value("role", serde_json::json!(format!("{:?}", role))),
    );

    Ok(Redirect::to("/admin/users").into_response())
}

#[derive(Deserialize)]
pub struct UpdateUserForm {
    email: Option<String>,
    role: Option<String>,
}

pub async fn update_user(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<UpdateUserForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let render_with_error =
        |state: &Arc<AppState>, user: &User, error: &str| -> AppResult<Response> {
            let users_list = auth::list_users(&state.db)?;
            let mut ctx = make_admin_context(state, user);
            ctx.insert("users", &users_list);
            ctx.insert("error", error);
            let html = state.templates.render("admin/users/index.html", &ctx)?;
            Ok((StatusCode::BAD_REQUEST, Html(html)).into_response())
        };

    if let Some(ref email) = form.email {
        if let Err(e) = auth::validate_email(email) {
            return render_with_error(&state, &user, &e.to_string());
        }
    }

    let target_user = match auth::get_user(&state.db, id)? {
        Some(target) => target,
        None => return render_with_error(&state, &user, "User not found"),
    };

    let role = form.role.and_then(|r| r.parse().ok());
    let new_role = role.unwrap_or(target_user.role);
    let admin_count = auth::list_users(&state.db)?
        .into_iter()
        .filter(|u| u.role == UserRole::Admin)
        .count();

    if let Err(e) = require_allowed_role_change(&user, &target_user, new_role, admin_count) {
        return Ok(e);
    }

    if let Err(e) = auth::update_user(&state.db, id, form.email.as_deref(), role) {
        let msg = e.to_string();
        let display = if msg.contains("UNIQUE constraint failed") {
            "A user with that email already exists".to_string()
        } else {
            "Could not update user. Please check your input and try again.".to_string()
        };
        return render_with_error(&state, &user, &display);
    }

    Ok(Redirect::to("/admin/users").into_response())
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    if user.id == id {
        return Ok((StatusCode::BAD_REQUEST, "Cannot delete yourself").into_response());
    }

    // Prevent deleting the last admin
    let target_user = auth::get_user(&state.db, id)?;
    if let Some(ref target) = target_user {
        if target.role == UserRole::Admin {
            let all_users = auth::list_users(&state.db)?;
            let admin_count = all_users
                .iter()
                .filter(|u| u.role == UserRole::Admin)
                .count();
            if admin_count <= 1 {
                return Ok((
                    StatusCode::BAD_REQUEST,
                    "Cannot delete the last admin account",
                )
                    .into_response());
            }
        }
    }

    let deleted_username = target_user.map(|u| u.username).unwrap_or_default();

    auth::delete_user(&state.db, id)?;

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::UserDelete, AuditCategory::User).entity(
            "user",
            id,
            Some(&deleted_username),
        ),
    );

    Ok(Redirect::to("/admin/users").into_response())
}

pub async fn update_tag(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<TagForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    tags::update_tag(&state.db, id, &form.name, None)?;
    Ok(Redirect::to("/admin/tags").into_response())
}

#[derive(Deserialize)]
pub struct AnalyticsQuery {
    #[serde(default = "default_days")]
    days: i64,
}

fn default_days() -> i64 {
    7
}

pub async fn analytics(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<AnalyticsQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let mut ctx = make_admin_context(&state, &user);

    if let Some(ref analytics) = state.analytics {
        let summary = analytics.get_summary(query.days)?;
        let realtime = analytics.get_realtime()?;

        tracing::info!(
            "Analytics: {} pageviews, {} sessions",
            summary.total_pageviews,
            summary.unique_sessions
        );

        ctx.insert("summary", &summary);
        ctx.insert("realtime", &realtime);
        ctx.insert("days", &query.days);
        ctx.insert("has_data", &(summary.total_pageviews > 0));
    } else {
        tracing::warn!("Analytics not available in state");
        ctx.insert("has_data", &false);
        ctx.insert("days", &query.days);
    }

    let html = state.templates.render("admin/analytics/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn analytics_realtime(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let mut ctx = Context::new();

    if let Some(ref analytics) = state.analytics {
        let realtime = analytics.get_realtime()?;
        ctx.insert("realtime", &realtime);
    }

    let html = state
        .templates
        .render("htmx/analytics_realtime.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn database_dashboard(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let db_path = &state.config().database.path;
    let stats = database::get_database_stats(&state.db, db_path)?;
    let analysis = database::analyze_database(&state.db, db_path)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("stats", &stats);
    ctx.insert("analysis", &analysis);

    let html = state.templates.render("admin/database/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct DatabaseActionForm {
    action: String,
}

pub async fn database_action(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<DatabaseActionForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    match form.action.as_str() {
        "vacuum" => {
            database::run_vacuum(&state.db)?;
        }
        "analyze" => {
            database::run_analyze(&state.db)?;
        }
        _ => {}
    }

    Ok(Redirect::to("/admin/database").into_response())
}

/// Get content performance data for analytics dashboard
pub async fn analytics_content(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<AnalyticsQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let mut ctx = Context::new();

    let content_performance: Vec<crate::services::analytics::ContentPerformance> =
        if let Some(ref analytics) = state.analytics {
            analytics
                .get_content_performance(query.days, 20)
                .unwrap_or_default()
        } else {
            vec![]
        };

    ctx.insert("content", &content_performance);
    ctx.insert("days", &query.days);

    let html = state
        .templates
        .render("htmx/analytics_content.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct ExportQuery {
    #[serde(default = "default_days")]
    days: i64,
    #[serde(default = "default_format")]
    format: String,
}

fn default_format() -> String {
    "json".to_string()
}

/// Export analytics data as JSON or CSV
pub async fn analytics_export(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<ExportQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    if let Some(ref analytics) = state.analytics {
        let format = match query.format.as_str() {
            "csv" => crate::services::analytics::ExportFormat::Csv,
            _ => crate::services::analytics::ExportFormat::Json,
        };

        let data = analytics.export(query.days, format)?;

        let (content_type, filename) = match query.format.as_str() {
            "csv" => ("text/csv", "analytics.csv"),
            _ => ("application/json", "analytics.json"),
        };

        Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, content_type),
                (
                    header::CONTENT_DISPOSITION,
                    &format!("attachment; filename=\"{}\"", filename),
                ),
            ],
            data,
        )
            .into_response())
    } else {
        Ok((StatusCode::NOT_FOUND, "Analytics not available").into_response())
    }
}

/// Get stats for a specific content item
pub async fn analytics_content_stats(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(content_id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = match content::get_content_by_id(&state.db, content_id)? {
        Some(content) => content,
        None => return Ok((StatusCode::NOT_FOUND, "Content not found").into_response()),
    };

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    if let Some(ref analytics) = state.analytics {
        let stats = analytics.get_content_stats(content_id)?;
        Ok(axum::Json(stats).into_response())
    } else {
        Ok((StatusCode::NOT_FOUND, "Analytics not available").into_response())
    }
}

// ============================================================================
// Content Versioning Handlers
// ============================================================================

#[derive(Deserialize)]
pub struct VersionQuery {
    #[serde(default = "default_limit")]
    limit: usize,
    #[serde(default)]
    offset: usize,
}

fn default_limit() -> usize {
    50
}

#[derive(Deserialize)]
pub struct DiffQuery {
    old: i64,
    new: i64,
}

/// List version history for a post
pub async fn post_versions(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    if content.content.content_type != ContentType::Post {
        return Ok((StatusCode::NOT_FOUND, "Not a post").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    let versions =
        crate::services::versions::list_versions(&state.db, id, query.limit, query.offset)?;
    let total = crate::services::versions::count_versions(&state.db, id)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("versions", &versions);
    ctx.insert("total_versions", &total);
    ctx.insert("content_type", "post");

    let html = state
        .templates
        .render("admin/versions/history.html", &ctx)?;
    Ok(Html(html).into_response())
}

/// View a specific version of a post
pub async fn post_version_view(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path((id, vid)): Path<(i64, i64)>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    if content.content.content_type != ContentType::Post {
        return Ok((StatusCode::NOT_FOUND, "Not a post").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    let version = crate::services::versions::get_version(&state.db, vid)?;

    if version.content_id != id {
        return Ok((StatusCode::NOT_FOUND, "Version not found").into_response());
    }

    // Render the markdown for preview
    let renderer = crate::services::markdown::MarkdownRenderer::new();
    let body_html = renderer.render(&version.body_markdown);

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("version", &version);
    ctx.insert("body_html", &body_html);
    ctx.insert("content_type", "post");

    let html = state.templates.render("admin/versions/view.html", &ctx)?;
    Ok(Html(html).into_response())
}

/// Restore a post to a previous version
pub async fn post_version_restore(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path((id, vid)): Path<(i64, i64)>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    if content.content.content_type != ContentType::Post {
        return Ok((StatusCode::NOT_FOUND, "Not a post").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    crate::services::versions::restore_version(&state.db, id, vid, Some(user.id))?;

    Ok(Redirect::to(&format!("/admin/posts/{}/edit", id)).into_response())
}

/// Compare two versions of a post
pub async fn post_version_diff(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Query(query): Query<DiffQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    if content.content.content_type != ContentType::Post {
        return Ok((StatusCode::NOT_FOUND, "Not a post").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    let old_version = match crate::services::versions::get_version(&state.db, query.old) {
        Ok(version) => version,
        Err(_) => return Ok((StatusCode::NOT_FOUND, "Version not found").into_response()),
    };
    let new_version = match crate::services::versions::get_version(&state.db, query.new) {
        Ok(version) => version,
        Err(_) => return Ok((StatusCode::NOT_FOUND, "Version not found").into_response()),
    };
    if old_version.content_id != id || new_version.content_id != id {
        return Ok((StatusCode::NOT_FOUND, "Version not found").into_response());
    }
    let diff = crate::services::versions::diff_versions(&state.db, query.old, query.new)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("diff", &diff);
    ctx.insert("content_type", "post");

    let html = state.templates.render("admin/versions/diff.html", &ctx)?;
    Ok(Html(html).into_response())
}

// ============================================================================
// Audit Log Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct AuditFilterParams {
    pub user_id: Option<i64>,
    pub username: Option<String>,
    pub action: Option<String>,
    pub category: Option<String>,
    pub entity_type: Option<String>,
    pub status: Option<String>,
    pub search: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub page: Option<usize>,
}

impl From<AuditFilterParams> for audit::AuditFilter {
    fn from(params: AuditFilterParams) -> Self {
        Self {
            user_id: params.user_id,
            username: params.username,
            action: params.action,
            category: params.category,
            entity_type: params.entity_type,
            status: params.status,
            search: params.search,
            from_date: params.from_date,
            to_date: params.to_date,
        }
    }
}

/// List audit logs with filtering and pagination
pub async fn audit_logs(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<AuditFilterParams>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let per_page = 50usize;
    let (page, offset) = admin_page_offset(params.page.unwrap_or(1), per_page);

    let filter = audit::AuditFilter::from(params);
    let logs = audit::list_logs(&state.db, &filter, per_page, offset)?;
    let total = audit::count_logs(&state.db, &filter)?;
    let total_pages = (total as usize).div_ceil(per_page).max(1);
    let summary = audit::get_summary(&state.db, 7)?;

    // Get filter options
    let audit_users = audit::get_audit_users(&state.db)?;
    let actions = audit::get_all_actions();
    let categories = audit::get_all_categories();

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("logs", &logs);
    ctx.insert("summary", &summary);
    ctx.insert("filter", &filter);
    ctx.insert("page", &page);
    ctx.insert("total_pages", &total_pages);
    ctx.insert("total", &total);
    ctx.insert("audit_users", &audit_users);
    ctx.insert("actions", &actions);
    ctx.insert("categories", &categories);

    let html = state.templates.render("admin/audit/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

/// View single audit log entry
pub async fn audit_log_detail(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let log =
        audit::get_log(&state.db, id)?.ok_or_else(|| anyhow::anyhow!("Audit log not found"))?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("log", &log);

    let html = state.templates.render("admin/audit/view.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Debug, Deserialize)]
pub struct AuditExportParams {
    pub format: Option<String>,
    pub user_id: Option<i64>,
    pub action: Option<String>,
    pub category: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
}

/// Export audit logs as JSON or CSV
pub async fn audit_export(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Query(params): Query<AuditExportParams>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let filter = audit::AuditFilter {
        user_id: params.user_id,
        action: params.action,
        category: params.category,
        from_date: params.from_date,
        to_date: params.to_date,
        ..Default::default()
    };

    let format = params.format.as_deref().unwrap_or("json");
    let data = audit::export_logs(&state.db, &filter, format)?;

    let (content_type, filename) = match format {
        "csv" => ("text/csv", "audit_logs.csv"),
        _ => ("application/json", "audit_logs.json"),
    };

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename=\"{}\"", filename),
            ),
        ],
        data,
    )
        .into_response())
}

/// List version history for a page
pub async fn page_versions(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Query(query): Query<VersionQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

    if content.content.content_type != ContentType::Page {
        return Ok((StatusCode::NOT_FOUND, "Not a page").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    let versions =
        crate::services::versions::list_versions(&state.db, id, query.limit, query.offset)?;
    let total = crate::services::versions::count_versions(&state.db, id)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("versions", &versions);
    ctx.insert("total_versions", &total);
    ctx.insert("content_type", "page");

    let html = state
        .templates
        .render("admin/versions/history.html", &ctx)?;
    Ok(Html(html).into_response())
}

/// View a specific version of a page
pub async fn page_version_view(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path((id, vid)): Path<(i64, i64)>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

    if content.content.content_type != ContentType::Page {
        return Ok((StatusCode::NOT_FOUND, "Not a page").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    let version = crate::services::versions::get_version(&state.db, vid)?;

    if version.content_id != id {
        return Ok((StatusCode::NOT_FOUND, "Version not found").into_response());
    }

    // Render the markdown for preview
    let renderer = crate::services::markdown::MarkdownRenderer::new();
    let body_html = renderer.render(&version.body_markdown);

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("version", &version);
    ctx.insert("body_html", &body_html);
    ctx.insert("content_type", "page");

    let html = state.templates.render("admin/versions/view.html", &ctx)?;
    Ok(Html(html).into_response())
}

/// Restore a page to a previous version
pub async fn page_version_restore(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path((id, vid)): Path<(i64, i64)>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

    if content.content.content_type != ContentType::Page {
        return Ok((StatusCode::NOT_FOUND, "Not a page").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    crate::services::versions::restore_version(&state.db, id, vid, Some(user.id))?;

    Ok(Redirect::to(&format!("/admin/pages/{}/edit", id)).into_response())
}

/// Compare two versions of a page
pub async fn page_version_diff(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Query(query): Query<DiffQuery>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let content = content::get_content_by_id(&state.db, id)?
        .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

    if content.content.content_type != ContentType::Page {
        return Ok((StatusCode::NOT_FOUND, "Not a page").into_response());
    }

    if let Err(e) = require_content_owner_or_admin(&user, &content) {
        return Ok(e);
    }

    let old_version = match crate::services::versions::get_version(&state.db, query.old) {
        Ok(version) => version,
        Err(_) => return Ok((StatusCode::NOT_FOUND, "Version not found").into_response()),
    };
    let new_version = match crate::services::versions::get_version(&state.db, query.new) {
        Ok(version) => version,
        Err(_) => return Ok((StatusCode::NOT_FOUND, "Version not found").into_response()),
    };
    if old_version.content_id != id || new_version.content_id != id {
        return Ok((StatusCode::NOT_FOUND, "Version not found").into_response());
    }
    let diff = crate::services::versions::diff_versions(&state.db, query.old, query.new)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("diff", &diff);
    ctx.insert("content_type", "page");

    let html = state.templates.render("admin/versions/diff.html", &ctx)?;
    Ok(Html(html).into_response())
}

// ============================================================================
// Draft Preview Token Generation
// ============================================================================

/// Generate a time-limited preview token for sharing draft content.
pub async fn generate_preview_token(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let item = content::get_content_by_id(&state.db, id)?;
    let item = match item {
        Some(item) => item,
        None => return Ok((StatusCode::NOT_FOUND, "Content not found").into_response()),
    };

    if let Err(e) = require_content_owner_or_admin(&user, &item) {
        return Ok(e);
    }

    let token = preview::generate_preview_token(&state.db, id)?;
    let config = state.config();
    let preview_url = format!("{}/preview/{}", config.site.url, token);

    Ok(axum::Json(serde_json::json!({
        "preview_url": preview_url,
        "expires_in_seconds": 3600,
    }))
    .into_response())
}

// ============================================================================
// Content Series Handlers
// ============================================================================

pub async fn series_list(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let all_series = series::list_series(&state.db, 100, 0)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("series_list", &all_series);

    let html = state.templates.render("admin/series/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn new_series(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let available_posts = content::list_content(&state.db, Some(ContentType::Post), None, 200, 0)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("series", &Option::<crate::models::SeriesWithItems>::None);
    ctx.insert("is_new", &true);
    ctx.insert("available_posts", &available_posts);

    let html = state.templates.render("admin/series/form.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct SeriesForm {
    title: String,
    slug: Option<String>,
    description: Option<String>,
    status: Option<String>,
    #[serde(default)]
    items: String, // comma-separated content IDs in order
}

pub async fn create_series_handler(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<SeriesForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let status = form.status.as_deref().unwrap_or("draft");
    let description = form.description.as_deref().unwrap_or("");
    let slug = form.slug.as_deref().filter(|s| !s.is_empty());

    let series_id = series::create_series(&state.db, &form.title, slug, description, status)?;

    // Add items in order
    let item_ids: Vec<i64> = form
        .items
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect();
    for content_id in &item_ids {
        let _ = series::add_item_to_series(&state.db, series_id, *content_id);
    }

    Ok(Redirect::to("/admin/series").into_response())
}

pub async fn edit_series(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let s = series::get_series_by_id(&state.db, id)?;
    match s {
        Some(s) => {
            let items = series::list_series_items(&state.db, id)?;
            let available_posts =
                content::list_content(&state.db, Some(ContentType::Post), None, 200, 0)?;
            let series_with = crate::models::SeriesWithItems { series: s, items };

            let mut ctx = make_admin_context(&state, &user);
            ctx.insert("series", &series_with);
            ctx.insert("is_new", &false);
            ctx.insert("available_posts", &available_posts);

            let html = state.templates.render("admin/series/form.html", &ctx)?;
            Ok(Html(html).into_response())
        }
        None => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

pub async fn update_series_handler(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<SeriesForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    series::update_series(
        &state.db,
        id,
        Some(&form.title),
        form.slug.as_deref(),
        form.description.as_deref(),
        form.status.as_deref(),
    )?;

    // Reorder items — replace all items with the submitted order
    let item_ids: Vec<i64> = form
        .items
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect();

    // Remove items not in the new list, add new ones
    let current_items = series::list_series_items(&state.db, id)?;
    let current_ids: Vec<i64> = current_items.iter().map(|i| i.content_id).collect();

    // Remove items no longer in the list
    for cid in &current_ids {
        if !item_ids.contains(cid) {
            let _ = series::remove_item_from_series(&state.db, id, *cid);
        }
    }
    // Add new items
    for cid in &item_ids {
        if !current_ids.contains(cid) {
            let _ = series::add_item_to_series(&state.db, id, *cid);
        }
    }
    // Reorder
    if !item_ids.is_empty() {
        series::reorder_series_items(&state.db, id, &item_ids)?;
    }

    Ok(Redirect::to("/admin/series").into_response())
}

pub async fn delete_series_handler(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    series::delete_series(&state.db, id)?;

    if is_htmx {
        Ok((
            [(
                header::HeaderName::from_static("hx-redirect"),
                "/admin/series".to_string(),
            )],
            "",
        )
            .into_response())
    } else {
        Ok(Redirect::to("/admin/series").into_response())
    }
}

// ============================================================================
// Snippets Handlers
// ============================================================================

pub async fn snippets(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let snippets = content::list_content_for_author(
        &state.db,
        Some(ContentType::Snippet),
        None,
        content_author_filter(&user),
        100,
        0,
    )?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("snippets", &snippets);

    let html = state.templates.render("admin/snippets/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn new_snippet(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &Option::<crate::models::ContentWithTags>::None);
    ctx.insert("is_new", &true);
    ctx.insert("content_type", "snippet");

    let html = state.templates.render("admin/snippets/form.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct SnippetForm {
    title: String,
    slug: Option<String>,
    body_markdown: String,
}

pub async fn create_snippet(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Form(form): Form<SnippetForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let input = CreateContent {
        title: form.title.clone(),
        slug: form.slug.filter(|s| !s.is_empty()),
        content_type: ContentType::Snippet,
        body_markdown: form.body_markdown,
        excerpt: None,
        featured_image: None,
        status: ContentStatus::Published,
        scheduled_at: None,
        tags: vec![],
        metadata: None,
    };

    content::create_content(
        &state.db,
        input,
        Some(user.id),
        state.config().content.excerpt_length,
    )?;

    Ok(Redirect::to("/admin/snippets").into_response())
}

pub async fn edit_snippet(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let snippet = content::get_content_by_id(&state.db, id)?;

    match snippet {
        Some(s) if s.content.content_type == ContentType::Snippet => {
            if let Err(e) = require_content_owner_or_admin(&user, &s) {
                return Ok(e);
            }

            let mut ctx = make_admin_context(&state, &user);
            ctx.insert("content", &s);
            ctx.insert("is_new", &false);
            ctx.insert("content_type", "snippet");

            let html = state.templates.render("admin/snippets/form.html", &ctx)?;
            Ok(Html(html).into_response())
        }
        _ => Ok(StatusCode::NOT_FOUND.into_response()),
    }
}

pub async fn update_snippet(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<SnippetForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let snippet = content::get_content_by_id(&state.db, id)?;
    match snippet {
        Some(ref s) if s.content.content_type == ContentType::Snippet => {
            if let Err(e) = require_content_owner_or_admin(&user, s) {
                return Ok(e);
            }
        }
        _ => return Ok(StatusCode::NOT_FOUND.into_response()),
    }

    let input = UpdateContent {
        title: Some(form.title),
        slug: form.slug,
        body_markdown: Some(form.body_markdown),
        excerpt: None,
        featured_image: None,
        status: Some(ContentStatus::Published),
        scheduled_at: None,
        tags: None,
        metadata: None,
    };

    let config = state.config();
    content::update_content(
        &state.db,
        id,
        input,
        config.content.excerpt_length,
        Some(user.id),
        config.content.version_retention,
    )?;

    Ok(Redirect::to("/admin/snippets").into_response())
}

pub async fn delete_snippet(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let snippet = content::get_content_by_id(&state.db, id)?;
    match snippet {
        Some(ref s) if s.content.content_type == ContentType::Snippet => {
            if let Err(e) = require_content_owner_or_admin(&user, s) {
                return Ok(e);
            }
        }
        _ => return Ok(StatusCode::NOT_FOUND.into_response()),
    }

    content::delete_content(&state.db, id)?;

    if is_htmx {
        Ok((
            [(
                header::HeaderName::from_static("hx-redirect"),
                "/admin/snippets".to_string(),
            )],
            "",
        )
            .into_response())
    } else {
        Ok(Redirect::to("/admin/snippets").into_response())
    }
}

// ============================================================================
// Bulk Operations
// ============================================================================

#[derive(Deserialize)]
pub struct BulkActionForm {
    action: String,
    #[serde(default)]
    ids: String, // comma-separated content IDs
}

pub async fn bulk_action(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Form(form): Form<BulkActionForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let ids: Vec<i64> = form
        .ids
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect();

    if ids.is_empty() {
        return Ok(Redirect::to("/admin/posts").into_response());
    }

    if user.role != UserRole::Admin {
        for id in &ids {
            if let Some(item) = content::get_content_by_id(&state.db, *id)? {
                if let Err(e) = require_content_owner_or_admin(&user, &item) {
                    return Ok(e);
                }
            }
        }
    }

    let action_label = form.action.clone();

    match form.action.as_str() {
        "publish" => {
            for id in &ids {
                let _ = content::update_content(
                    &state.db,
                    *id,
                    UpdateContent {
                        status: Some(ContentStatus::Published),
                        ..Default::default()
                    },
                    state.config().content.excerpt_length,
                    Some(user.id),
                    state.config().content.version_retention,
                );
            }
        }
        "draft" => {
            for id in &ids {
                let _ = content::update_content(
                    &state.db,
                    *id,
                    UpdateContent {
                        status: Some(ContentStatus::Draft),
                        ..Default::default()
                    },
                    state.config().content.excerpt_length,
                    Some(user.id),
                    state.config().content.version_retention,
                );
            }
        }
        "archive" => {
            for id in &ids {
                let _ = content::update_content(
                    &state.db,
                    *id,
                    UpdateContent {
                        status: Some(ContentStatus::Archived),
                        ..Default::default()
                    },
                    state.config().content.excerpt_length,
                    Some(user.id),
                    state.config().content.version_retention,
                );
            }
        }
        "delete" => {
            for id in &ids {
                let _ = content::delete_content(&state.db, *id);
            }
        }
        _ => {}
    }

    // Audit log for bulk action
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Update, AuditCategory::Content)
            .metadata_value("bulk_action", serde_json::json!(action_label))
            .metadata_value("affected_ids", serde_json::json!(ids)),
    );

    Ok(Redirect::to("/admin/posts").into_response())
}

// ===== API Token Management =====

pub async fn tokens(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let tokens = api_token::list_tokens(&state.db).unwrap_or_default();
    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("tokens", &tokens);
    ctx.insert("new_token", &Option::<String>::None);

    let html = state.templates.render("admin/tokens/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct CreateTokenForm {
    pub name: String,
    pub permissions: Option<String>,
    pub expires_days: Option<i64>,
}

pub async fn create_token(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Form(form): Form<CreateTokenForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    // Normalize to a canonical scope so only known values are persisted.
    let permissions = if api_token::can_write(form.permissions.as_deref().unwrap_or("read")) {
        "write"
    } else {
        "read"
    };
    let expires_at = form.expires_days.and_then(|days| {
        if days > 0 {
            Some(
                (chrono::Utc::now() + chrono::Duration::days(days))
                    .format("%Y-%m-%dT%H:%M:%S")
                    .to_string(),
            )
        } else {
            None
        }
    });

    let (raw_token, _token) = api_token::create_token(
        &state.db,
        &form.name,
        permissions,
        Some(user.id),
        expires_at.as_deref(),
    )?;

    // Audit log
    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Create, AuditCategory::Settings).metadata_value(
            "detail",
            serde_json::json!(format!("Created API token: {}", form.name)),
        ),
    );

    let tokens = api_token::list_tokens(&state.db).unwrap_or_default();
    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("tokens", &tokens);
    ctx.insert("new_token", &Some(&raw_token));

    let html = state.templates.render("admin/tokens/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn revoke_token(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    api_token::revoke_token(&state.db, id)?;

    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Delete, AuditCategory::Settings).metadata_value(
            "detail",
            serde_json::json!(format!("Revoked API token ID: {}", id)),
        ),
    );

    Ok(Redirect::to("/admin/tokens").into_response())
}

// ===== Webhook Management =====

pub async fn webhooks(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let hooks = webhook::list_webhooks(&state.db).unwrap_or_default();
    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("webhooks", &hooks);

    let html = state.templates.render("admin/webhooks/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct WebhookForm {
    pub name: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: Option<String>,
    pub event_content_published: Option<String>,
    pub event_content_updated: Option<String>,
    pub event_content_deleted: Option<String>,
    pub event_media_uploaded: Option<String>,
    pub event_media_deleted: Option<String>,
    pub active: Option<String>,
}

impl WebhookForm {
    fn events_string(&self) -> String {
        if let Some(ref events) = self.events {
            return events.clone();
        }
        let mut events = Vec::new();
        if self.event_content_published.is_some() {
            events.push("content.published");
        }
        if self.event_content_updated.is_some() {
            events.push("content.updated");
        }
        if self.event_content_deleted.is_some() {
            events.push("content.deleted");
        }
        if self.event_media_uploaded.is_some() {
            events.push("media.uploaded");
        }
        if self.event_media_deleted.is_some() {
            events.push("media.deleted");
        }
        events.join(",")
    }
}

pub async fn create_webhook_handler(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Form(form): Form<WebhookForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let secret = form.secret.as_deref().filter(|s| !s.is_empty());
    let events = form.events_string();

    webhook::create_webhook(&state.db, &form.name, &form.url, secret, &events)?;

    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Create, AuditCategory::Settings).metadata_value(
            "detail",
            serde_json::json!(format!("Created webhook: {}", form.name)),
        ),
    );

    Ok(Redirect::to("/admin/webhooks").into_response())
}

pub async fn edit_webhook(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let hook = webhook::get_webhook(&state.db, id)?;
    let hooks = webhook::list_webhooks(&state.db).unwrap_or_default();
    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("webhook", &hook);
    ctx.insert("webhooks", &hooks);
    ctx.insert("is_edit", &true);

    let html = state.templates.render("admin/webhooks/index.html", &ctx)?;
    Ok(Html(html).into_response())
}

pub async fn update_webhook_handler(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Path(id): Path<i64>,
    Form(form): Form<WebhookForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let secret = form.secret.as_deref().filter(|s| !s.is_empty());
    let events = form.events_string();
    let active = form.active.is_some();

    webhook::update_webhook(
        &state.db, id, &form.name, &form.url, secret, &events, active,
    )?;

    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Update, AuditCategory::Settings).metadata_value(
            "detail",
            serde_json::json!(format!("Updated webhook: {}", form.name)),
        ),
    );

    Ok(Redirect::to("/admin/webhooks").into_response())
}

pub async fn delete_webhook_handler(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    AuditInfo(mut audit_ctx): AuditInfo,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    webhook::delete_webhook(&state.db, id)?;

    audit_ctx.user_id = Some(user.id);
    audit_ctx.username = Some(user.username.clone());
    audit_ctx.user_role = Some(format!("{:?}", user.role));
    let _ = audit::log(
        &state.db,
        &audit_ctx,
        AuditLogBuilder::new(AuditAction::Delete, AuditCategory::Settings).metadata_value(
            "detail",
            serde_json::json!(format!("Deleted webhook ID: {}", id)),
        ),
    );

    Ok(Redirect::to("/admin/webhooks").into_response())
}

pub async fn webhook_deliveries(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let hook = webhook::get_webhook(&state.db, id)?;
    let deliveries = webhook::list_deliveries(&state.db, id, 50).unwrap_or_default();
    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("webhook", &hook);
    ctx.insert("deliveries", &deliveries);

    let html = state
        .templates
        .render("admin/webhooks/deliveries.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[cfg(test)]
mod tests {
    use super::{
        admin_page_offset, content_author_filter, require_allowed_role_change,
        require_content_owner_or_admin, ContentWithTags, StatusCode, User, UserRole,
    };
    use crate::models::{Content, ContentStatus, ContentType};

    fn test_user(id: i64, role: UserRole) -> User {
        User {
            id,
            username: format!("user{}", id),
            email: format!("user{}@example.com", id),
            password_hash: "hash".to_string(),
            role,
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn test_content(author_id: Option<i64>) -> ContentWithTags {
        ContentWithTags {
            content: Content {
                id: 1,
                slug: "test-post".to_string(),
                title: "Test Post".to_string(),
                content_type: ContentType::Post,
                body_markdown: String::new(),
                body_html: String::new(),
                excerpt: None,
                featured_image: None,
                status: ContentStatus::Draft,
                scheduled_at: None,
                published_at: None,
                author_id,
                metadata: serde_json::json!({}),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                updated_at: "2024-01-01T00:00:00Z".to_string(),
            },
            tags: vec![],
            author: None,
        }
    }

    #[test]
    fn authors_can_only_manage_their_own_content() {
        let author = test_user(1, UserRole::Author);
        let own_content = test_content(Some(author.id));
        let other_content = test_content(Some(99));

        assert!(require_content_owner_or_admin(&author, &own_content).is_ok());
        assert_eq!(
            require_content_owner_or_admin(&author, &other_content)
                .unwrap_err()
                .status(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn admins_can_manage_any_content() {
        let admin = test_user(1, UserRole::Admin);
        let other_content = test_content(Some(99));

        assert!(require_content_owner_or_admin(&admin, &other_content).is_ok());
    }

    #[test]
    fn last_admin_cannot_be_demoted() {
        let admin = test_user(1, UserRole::Admin);

        assert_eq!(
            require_allowed_role_change(&admin, &admin, UserRole::Author, 1)
                .unwrap_err()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn admin_can_demote_another_admin_when_one_remains() {
        let current_admin = test_user(1, UserRole::Admin);
        let target_admin = test_user(2, UserRole::Admin);

        assert!(
            require_allowed_role_change(&current_admin, &target_admin, UserRole::Author, 2).is_ok()
        );
    }

    #[test]
    fn author_filter_applies_only_to_authors() {
        let author = test_user(7, UserRole::Author);
        let admin = test_user(1, UserRole::Admin);

        assert_eq!(content_author_filter(&author), Some(7));
        assert_eq!(content_author_filter(&admin), None);
    }

    #[test]
    fn admin_page_offset_clamps_huge_pages() {
        let (page, offset) = admin_page_offset(usize::MAX, 25);

        assert_eq!(page, 10_000);
        assert_eq!(offset, (10_000 - 1) * 25);
    }
}
