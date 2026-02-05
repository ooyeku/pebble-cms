use crate::models::{ContentStatus, ContentType, CreateContent, UpdateContent, User, UserRole};
use crate::services::{auth, content, database, media, settings, tags};
use crate::web::error::AppResult;
use crate::web::extractors::{CurrentUser, HxRequest};
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
        ctx.insert(
            "theme_custom_css",
            &config.theme.custom.to_css_variables(),
        );
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

pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let recent_posts = content::list_content(&state.db, Some(ContentType::Post), None, 5, 0)?;
    let post_count = content::count_content(&state.db, Some(ContentType::Post), None)?;
    let page_count = content::count_content(&state.db, Some(ContentType::Page), None)?;
    let published_count = content::count_content(&state.db, None, Some(ContentStatus::Published))?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("recent_posts", &recent_posts);
    ctx.insert("post_count", &post_count);
    ctx.insert("page_count", &page_count);
    ctx.insert("published_count", &published_count);

    let html = state.templates.render("admin/dashboard.html", &ctx)?;
    Ok(Html(html))
}

pub async fn posts(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let posts = content::list_content(&state.db, Some(ContentType::Post), None, 50, 0)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("posts", &posts);

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

fn build_seo_metadata(form: &ContentForm) -> serde_json::Value {
    let mut metadata = serde_json::json!({});
    if let Some(ref mt) = form.meta_title {
        if !mt.is_empty() {
            metadata["meta_title"] = serde_json::json!(mt);
        }
    }
    if let Some(ref md) = form.meta_description {
        if !md.is_empty() {
            metadata["meta_description"] = serde_json::json!(md);
        }
    }
    if let Some(ref cu) = form.canonical_url {
        if !cu.is_empty() {
            metadata["canonical_url"] = serde_json::json!(cu);
        }
    }
    metadata
}

fn build_page_metadata(form: &ContentForm) -> serde_json::Value {
    let mut metadata = build_seo_metadata(form);

    // Custom code fields - only save non-empty values
    if let Some(ref html) = form.custom_html {
        if !html.trim().is_empty() {
            metadata["custom_html"] = serde_json::json!(html);
        }
    }
    if let Some(ref css) = form.custom_css {
        if !css.trim().is_empty() {
            metadata["custom_css"] = serde_json::json!(css);
        }
    }
    if let Some(ref js) = form.custom_js {
        if !js.trim().is_empty() {
            metadata["custom_js"] = serde_json::json!(js);
        }
    }
    // use_custom_code: "only" = only custom code, empty/none = markdown only
    if let Some(ref mode) = form.use_custom_code {
        if !mode.is_empty() {
            metadata["use_custom_code"] = serde_json::json!(mode);
        }
    }

    metadata
}

pub async fn create_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
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

    content::create_content(
        &state.db,
        input,
        Some(user.id),
        state.config().content.excerpt_length,
    )?;

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
    HxRequest(_is_htmx): HxRequest,
    Path(id): Path<i64>,
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

    Ok(Redirect::to("/admin/posts").into_response())
}

pub async fn delete_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    content::delete_content(&state.db, id)?;

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
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    let pages = content::list_content(&state.db, Some(ContentType::Page), None, 50, 0)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("pages", &pages);

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
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
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
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_author_or_admin(&user) {
        return Ok(e);
    }

    content::delete_content(&state.db, id)?;

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
    if !state.upload_rate_limiter.check(&rate_key) {
        return Ok((
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            "Too many uploads. Please wait before uploading more files.",
        )
            .into_response());
    }

    while let Some(field) = multipart.next_field().await? {
        let name = field.file_name().unwrap_or("unknown").to_string();
        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let data = field.bytes().await?;

        media::upload_media(
            &state.db,
            &state.media_dir,
            &name,
            &content_type,
            &data,
            Some(user.id),
        )?;
        state.upload_rate_limiter.record_attempt(&rate_key);
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
    ctx.insert("available_themes", &crate::config::ThemeConfig::AVAILABLE_THEMES);

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
    };

    // Drop the read lock before updating
    drop(current);

    // Update config (writes to file and updates in-memory)
    if let Err(e) = state.update_config(new_config) {
        let mut ctx = make_admin_context(&state, &user);
        ctx.insert("error", &e.to_string());
        ctx.insert("config", &*state.config());
        ctx.insert("homepage", &settings::get_homepage_settings(&state.db).unwrap_or_default());
        ctx.insert("available_themes", &crate::config::ThemeConfig::AVAILABLE_THEMES);
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
    Form(form): Form<CreateUserForm>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    let role: UserRole = form.role.parse().unwrap_or(UserRole::Author);
    auth::create_user(&state.db, &form.username, &form.email, &form.password, role)?;

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

    let role = form.role.and_then(|r| r.parse().ok());
    auth::update_user(&state.db, id, form.email.as_deref(), role)?;

    Ok(Redirect::to("/admin/users").into_response())
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    if let Err(e) = require_admin(&user) {
        return Ok(e);
    }

    if user.id == id {
        return Ok((StatusCode::BAD_REQUEST, "Cannot delete yourself").into_response());
    }

    auth::delete_user(&state.db, id)?;

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
            analytics.get_content_performance(query.days, 20).unwrap_or_default()
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

    let versions = crate::services::versions::list_versions(&state.db, id, query.limit, query.offset)?;
    let total = crate::services::versions::count_versions(&state.db, id)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("versions", &versions);
    ctx.insert("total_versions", &total);
    ctx.insert("content_type", "post");

    let html = state.templates.render("admin/versions/history.html", &ctx)?;
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

    let diff = crate::services::versions::diff_versions(&state.db, query.old, query.new)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("diff", &diff);
    ctx.insert("content_type", "post");

    let html = state.templates.render("admin/versions/diff.html", &ctx)?;
    Ok(Html(html).into_response())
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

    let versions = crate::services::versions::list_versions(&state.db, id, query.limit, query.offset)?;
    let total = crate::services::versions::count_versions(&state.db, id)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("versions", &versions);
    ctx.insert("total_versions", &total);
    ctx.insert("content_type", "page");

    let html = state.templates.render("admin/versions/history.html", &ctx)?;
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

    let diff = crate::services::versions::diff_versions(&state.db, query.old, query.new)?;

    let mut ctx = make_admin_context(&state, &user);
    ctx.insert("content", &content);
    ctx.insert("diff", &diff);
    ctx.insert("content_type", "page");

    let html = state.templates.render("admin/versions/diff.html", &ctx)?;
    Ok(Html(html).into_response())
}
