use crate::models::{ContentStatus, ContentType, CreateContent, UpdateContent, UserRole};
use crate::services::{auth, content, media, tags};
use crate::web::error::AppResult;
use crate::web::extractors::{CurrentUser, HxRequest};
use crate::web::state::AppState;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;

pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let recent_posts = content::list_content(&state.db, Some(ContentType::Post), None, 5, 0)?;
    let post_count = content::count_content(&state.db, Some(ContentType::Post), None)?;
    let page_count = content::count_content(&state.db, Some(ContentType::Page), None)?;
    let published_count = content::count_content(&state.db, None, Some(ContentStatus::Published))?;

    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
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
) -> AppResult<Html<String>> {
    let posts = content::list_content(&state.db, Some(ContentType::Post), None, 50, 0)?;

    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("posts", &posts);

    let html = state.templates.render("admin/posts/index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn new_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let all_tags = tags::list_tags(&state.db)?;

    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("content", &Option::<crate::models::ContentWithTags>::None);
    ctx.insert("all_tags", &all_tags);
    ctx.insert("is_new", &true);
    ctx.insert("content_type", "post");

    let html = state.templates.render("admin/posts/form.html", &ctx)?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct ContentForm {
    title: String,
    slug: Option<String>,
    body_markdown: String,
    excerpt: Option<String>,
    status: String,
    #[serde(default)]
    tags: String,
    // SEO fields
    meta_title: Option<String>,
    meta_description: Option<String>,
    canonical_url: Option<String>,
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

pub async fn create_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    HxRequest(_is_htmx): HxRequest,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
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
        tags,
        metadata: Some(build_seo_metadata(&form)),
    };

    content::create_content(
        &state.db,
        input,
        Some(user.id),
        state.config.content.excerpt_length,
    )?;

    Ok(Redirect::to("/admin/posts").into_response())
}

pub async fn edit_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<i64>,
) -> AppResult<Response> {
    let post = content::get_content_by_id(&state.db, id)?;

    match post {
        Some(p) if p.content.content_type == ContentType::Post => {
            let all_tags = tags::list_tags(&state.db)?;

            let mut ctx = Context::new();
            ctx.insert("site", &state.config.site);
            ctx.insert("user", &user);
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
    CurrentUser(_user): CurrentUser,
    HxRequest(_is_htmx): HxRequest,
    Path(id): Path<i64>,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
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
        tags: Some(tags),
        metadata: Some(build_seo_metadata(&form)),
    };

    content::update_content(&state.db, id, input, state.config.content.excerpt_length)?;

    Ok(Redirect::to("/admin/posts").into_response())
}

pub async fn delete_post(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
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
) -> AppResult<Html<String>> {
    let pages = content::list_content(&state.db, Some(ContentType::Page), None, 50, 0)?;

    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("pages", &pages);

    let html = state.templates.render("admin/pages/index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn new_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("content", &Option::<crate::models::ContentWithTags>::None);
    ctx.insert("is_new", &true);
    ctx.insert("content_type", "page");

    let html = state.templates.render("admin/pages/form.html", &ctx)?;
    Ok(Html(html))
}

pub async fn create_page(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
    let input = CreateContent {
        title: form.title.clone(),
        slug: form.slug.clone().filter(|s| !s.is_empty()),
        content_type: ContentType::Page,
        body_markdown: form.body_markdown.clone(),
        excerpt: form.excerpt.clone().filter(|s| !s.is_empty()),
        featured_image: None,
        status: form.status.parse().unwrap_or(ContentStatus::Draft),
        tags: vec![],
        metadata: Some(build_seo_metadata(&form)),
    };

    let id = content::create_content(
        &state.db,
        input,
        Some(user.id),
        state.config.content.excerpt_length,
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
    let page = content::get_content_by_id(&state.db, id)?;

    match page {
        Some(p) if p.content.content_type == ContentType::Page => {
            let mut ctx = Context::new();
            ctx.insert("site", &state.config.site);
            ctx.insert("user", &user);
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
    CurrentUser(_user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
    Form(form): Form<ContentForm>,
) -> AppResult<Response> {
    let input = UpdateContent {
        title: Some(form.title.clone()),
        slug: form.slug.clone().filter(|s| !s.is_empty()),
        body_markdown: Some(form.body_markdown.clone()),
        excerpt: form.excerpt.clone(),
        featured_image: None,
        status: Some(form.status.parse().unwrap_or(ContentStatus::Draft)),
        tags: None,
        metadata: Some(build_seo_metadata(&form)),
    };

    content::update_content(&state.db, id, input, state.config.content.excerpt_length)?;

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
    CurrentUser(_user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
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
) -> AppResult<Html<String>> {
    let media_list = media::list_media(&state.db, 100, 0)?;

    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("media", &media_list);

    let html = state.templates.render("admin/media/index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn media(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    media_page(State(state), CurrentUser(user)).await
}

pub async fn upload_media(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
    mut multipart: Multipart,
) -> AppResult<Response> {
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
    }

    Ok(Redirect::to("/admin/media").into_response())
}

pub async fn delete_media(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
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
) -> AppResult<Html<String>> {
    let tags_list = tags::list_tags_with_counts(&state.db)?;

    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("tags", &tags_list);

    let html = state.templates.render("admin/tags/index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn tags(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    tags_page(State(state), CurrentUser(user)).await
}

#[derive(Deserialize)]
pub struct TagForm {
    name: String,
}

pub async fn create_tag(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user): CurrentUser,
    Form(form): Form<TagForm>,
) -> AppResult<Response> {
    tags::create_tag(&state.db, &form.name, None)?;
    Ok(Redirect::to("/admin/tags").into_response())
}

pub async fn delete_tag(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user): CurrentUser,
    HxRequest(is_htmx): HxRequest,
    Path(id): Path<i64>,
) -> AppResult<Response> {
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
) -> AppResult<Html<String>> {
    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("config", &state.config);

    let html = state.templates.render("admin/settings/index.html", &ctx)?;
    Ok(Html(html))
}

pub async fn users(
    State(state): State<Arc<AppState>>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Html<String>> {
    if user.role != UserRole::Admin {
        return Ok(Html("Unauthorized".to_string()));
    }

    let users_list = auth::list_users(&state.db)?;

    let mut ctx = Context::new();
    ctx.insert("site", &state.config.site);
    ctx.insert("user", &user);
    ctx.insert("users", &users_list);

    let html = state.templates.render("admin/users/index.html", &ctx)?;
    Ok(Html(html))
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
    if user.role != UserRole::Admin {
        return Ok(StatusCode::FORBIDDEN.into_response());
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
    if user.role != UserRole::Admin {
        return Ok(StatusCode::FORBIDDEN.into_response());
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
    if user.role != UserRole::Admin {
        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    if user.id == id {
        return Ok((StatusCode::BAD_REQUEST, "Cannot delete yourself").into_response());
    }

    auth::delete_user(&state.db, id)?;

    Ok(Redirect::to("/admin/users").into_response())
}

pub async fn update_tag(
    State(state): State<Arc<AppState>>,
    CurrentUser(_user): CurrentUser,
    Path(id): Path<i64>,
    Form(form): Form<TagForm>,
) -> AppResult<Response> {
    tags::update_tag(&state.db, id, &form.name, None)?;
    Ok(Redirect::to("/admin/tags").into_response())
}
