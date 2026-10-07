use crate::models::{ContentStatus, ContentType, CreateContent, UpdateContent};
use crate::services::{content, media, series, tags};
use crate::web::extractors::{ApiTokenAuth, ApiTokenWrite};
use crate::web::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use std::sync::Arc;

const MAX_API_PAGE: usize = 10_000;

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub tag: Option<String>,
}

fn paginate(
    page: Option<usize>,
    per_page: Option<usize>,
    default_size: usize,
    max_size: usize,
) -> (usize, usize, usize) {
    let page = page.unwrap_or(1).max(1).min(MAX_API_PAGE);
    let per_page = per_page.unwrap_or(default_size).min(max_size).max(1);
    let offset = page.saturating_sub(1).saturating_mul(per_page);
    (page, per_page, offset)
}

fn json_envelope(
    data: serde_json::Value,
    total: i64,
    page: usize,
    per_page: usize,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "data": data,
        "meta": {
            "total": total,
            "page": page,
            "per_page": per_page,
        }
    }))
}

fn json_single(data: serde_json::Value) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "data": data,
    }))
}

fn not_found(msg: &str) -> Response {
    let body = serde_json::json!({
        "error": "Not Found",
        "message": msg,
    });
    (StatusCode::NOT_FOUND, Json(body)).into_response()
}

/// GET /api/v1/posts
pub async fn list_posts(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Query(params): Query<PaginationParams>,
) -> Response {
    let config = state.config();
    let default_size = config.api.default_page_size;
    let max_size = config.api.max_page_size;
    drop(config);

    let (page, per_page, offset) = paginate(params.page, params.per_page, default_size, max_size);

    // If filtering by tag, use the tag service
    if let Some(ref tag_slug) = params.tag {
        match tags::get_posts_by_tag(&state.db, tag_slug) {
            Ok(posts) => {
                let total = posts.len() as i64;
                let paginated: Vec<_> = posts.into_iter().skip(offset).take(per_page).collect();
                json_envelope(
                    serde_json::to_value(&paginated).unwrap_or_default(),
                    total,
                    page,
                    per_page,
                )
                .into_response()
            }
            Err(e) => {
                tracing::error!("API list_posts by tag error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Internal server error"})),
                )
                    .into_response()
            }
        }
    } else {
        let total = content::count_content(
            &state.db,
            Some(ContentType::Post),
            Some(ContentStatus::Published),
        )
        .unwrap_or(0);
        match content::list_published_content(&state.db, ContentType::Post, per_page, offset) {
            Ok(posts) => json_envelope(
                serde_json::to_value(&posts).unwrap_or_default(),
                total,
                page,
                per_page,
            )
            .into_response(),
            Err(e) => {
                tracing::error!("API list_posts error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error": "Internal server error"})),
                )
                    .into_response()
            }
        }
    }
}

/// GET /api/v1/posts/:slug
pub async fn get_post(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Path(slug): Path<String>,
) -> Response {
    match content::get_content_by_slug(&state.db, &slug) {
        Ok(Some(post))
            if post.content.content_type == ContentType::Post
                && post.content.status == ContentStatus::Published =>
        {
            json_single(serde_json::to_value(&post).unwrap_or_default()).into_response()
        }
        Ok(_) => not_found("Post not found"),
        Err(e) => {
            tracing::error!("API get_post error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/pages
pub async fn list_pages(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Query(params): Query<PaginationParams>,
) -> Response {
    let config = state.config();
    let default_size = config.api.default_page_size;
    let max_size = config.api.max_page_size;
    drop(config);

    let (page, per_page, offset) = paginate(params.page, params.per_page, default_size, max_size);
    let total = content::count_content(
        &state.db,
        Some(ContentType::Page),
        Some(ContentStatus::Published),
    )
    .unwrap_or(0);

    match content::list_published_content(&state.db, ContentType::Page, per_page, offset) {
        Ok(pages) => json_envelope(
            serde_json::to_value(&pages).unwrap_or_default(),
            total,
            page,
            per_page,
        )
        .into_response(),
        Err(e) => {
            tracing::error!("API list_pages error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/pages/:slug
pub async fn get_page(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Path(slug): Path<String>,
) -> Response {
    match content::get_content_by_slug(&state.db, &slug) {
        Ok(Some(page))
            if page.content.content_type == ContentType::Page
                && page.content.status == ContentStatus::Published =>
        {
            json_single(serde_json::to_value(&page).unwrap_or_default()).into_response()
        }
        Ok(_) => not_found("Page not found"),
        Err(e) => {
            tracing::error!("API get_page error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/tags
pub async fn list_tags(State(state): State<Arc<AppState>>, _auth: ApiTokenAuth) -> Response {
    match tags::list_published_tags_with_counts(&state.db) {
        Ok(tags) => {
            let total = tags.len() as i64;
            json_envelope(
                serde_json::to_value(&tags).unwrap_or_default(),
                total,
                1,
                total as usize,
            )
            .into_response()
        }
        Err(e) => {
            tracing::error!("API list_tags error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/tags/:slug
pub async fn get_tag(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Path(slug): Path<String>,
) -> Response {
    match tags::get_tag_by_slug(&state.db, &slug) {
        Ok(Some(tag)) => {
            let posts = tags::get_posts_by_tag(&state.db, &slug).unwrap_or_default();
            let data = serde_json::json!({
                "tag": tag,
                "posts": posts,
            });
            json_single(data).into_response()
        }
        Ok(None) => not_found("Tag not found"),
        Err(e) => {
            tracing::error!("API get_tag error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/series
pub async fn list_series_api(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Query(params): Query<PaginationParams>,
) -> Response {
    let config = state.config();
    let default_size = config.api.default_page_size;
    let max_size = config.api.max_page_size;
    drop(config);

    let (page, per_page, offset) = paginate(params.page, params.per_page, default_size, max_size);
    let total = series::count_series(&state.db).unwrap_or(0);

    match series::list_series(&state.db, per_page, offset) {
        Ok(all_series) => json_envelope(
            serde_json::to_value(&all_series).unwrap_or_default(),
            total,
            page,
            per_page,
        )
        .into_response(),
        Err(e) => {
            tracing::error!("API list_series error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/series/:slug
pub async fn get_series_api(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Path(slug): Path<String>,
) -> Response {
    match series::get_series_with_items(&state.db, &slug) {
        Ok(Some(si)) => json_single(serde_json::to_value(&si).unwrap_or_default()).into_response(),
        Ok(None) => not_found("Series not found"),
        Err(e) => {
            tracing::error!("API get_series error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/media
pub async fn list_media_api(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Query(params): Query<PaginationParams>,
) -> Response {
    let config = state.config();
    let default_size = config.api.default_page_size;
    let max_size = config.api.max_page_size;
    drop(config);

    let (page, per_page, offset) = paginate(params.page, params.per_page, default_size, max_size);
    let total = media::count_media(&state.db).unwrap_or(0);

    match media::list_media(&state.db, per_page, offset) {
        Ok(media_list) => json_envelope(
            serde_json::to_value(&media_list).unwrap_or_default(),
            total,
            page,
            per_page,
        )
        .into_response(),
        Err(e) => {
            tracing::error!("API list_media error: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": "Internal server error"})),
            )
                .into_response()
        }
    }
}

/// GET /api/v1/site
pub async fn site_info(State(state): State<Arc<AppState>>, _auth: ApiTokenAuth) -> Response {
    let config = state.config();
    let data = serde_json::json!({
        "title": config.site.title,
        "description": config.site.description,
        "url": config.site.url,
        "language": config.site.language,
        "theme": config.theme.name,
        "version": env!("CARGO_PKG_VERSION"),
    });
    drop(config);
    json_single(data).into_response()
}

// ---------------------------------------------------------------------------
// Write API (requires a token with write permission via `ApiTokenWrite`)
// ---------------------------------------------------------------------------

fn error_response(status: StatusCode, message: &str) -> Response {
    let error = status.canonical_reason().unwrap_or("Error");
    (
        status,
        Json(serde_json::json!({ "error": error, "message": message })),
    )
        .into_response()
}

/// Map a content-service error to an HTTP response. Known client-side validation
/// messages (raised via `bail!` in the service) are surfaced as 4xx; anything
/// else is treated as an unexpected server error and the detail is not leaked.
fn classify_write_error(context: &str, e: anyhow::Error) -> Response {
    let msg = e.to_string();
    let lower = msg.to_ascii_lowercase();

    if lower.contains("already exists") {
        return error_response(StatusCode::CONFLICT, &msg);
    }

    let is_client_error = lower.starts_with("invalid")
        || lower.contains("must be")
        || lower.contains("cannot be")
        || lower.contains("required")
        || lower.contains("scheduled")
        || lower.contains("timestamp");

    if is_client_error {
        return error_response(StatusCode::BAD_REQUEST, &msg);
    }

    tracing::error!("API {} error: {}", context, e);
    error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
}

fn created_response(state: &Arc<AppState>, id: i64) -> Response {
    match content::get_content_by_id(&state.db, id) {
        Ok(Some(item)) => (
            StatusCode::CREATED,
            json_single(serde_json::to_value(&item).unwrap_or_default()),
        )
            .into_response(),
        _ => (
            StatusCode::CREATED,
            json_single(serde_json::json!({ "id": id })),
        )
            .into_response(),
    }
}

fn create_content_api(
    state: &Arc<AppState>,
    content_type: ContentType,
    mut input: CreateContent,
    author_id: Option<i64>,
) -> Response {
    input.content_type = content_type;
    let excerpt_length = state.config().content.excerpt_length;

    match content::create_content(&state.db, input, author_id, excerpt_length) {
        Ok(id) => created_response(state, id),
        Err(e) => classify_write_error("create_content", e),
    }
}

/// Resolve a slug to an existing content item of the expected type. Returns the
/// item id, or a boxed error response (404 if missing/wrong type, 500 on DB error).
fn resolve_content_id(
    state: &Arc<AppState>,
    content_type: ContentType,
    slug: &str,
) -> Result<i64, Box<Response>> {
    match content::get_content_by_slug(&state.db, slug) {
        Ok(Some(item)) if item.content.content_type == content_type => Ok(item.content.id),
        Ok(_) => Err(Box::new(not_found(match content_type {
            ContentType::Page => "Page not found",
            _ => "Post not found",
        }))),
        Err(e) => {
            tracing::error!("API resolve_content error: {}", e);
            Err(Box::new(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
            )))
        }
    }
}

fn update_content_api(
    state: &Arc<AppState>,
    content_type: ContentType,
    slug: &str,
    input: UpdateContent,
    actor_id: Option<i64>,
) -> Response {
    let id = match resolve_content_id(state, content_type, slug) {
        Ok(id) => id,
        Err(resp) => return *resp,
    };

    let (excerpt_length, version_retention) = {
        let config = state.config();
        (
            config.content.excerpt_length,
            config.content.version_retention,
        )
    };

    match content::update_content(
        &state.db,
        id,
        input,
        excerpt_length,
        actor_id,
        version_retention,
    ) {
        Ok(()) => match content::get_content_by_id(&state.db, id) {
            Ok(Some(item)) => {
                json_single(serde_json::to_value(&item).unwrap_or_default()).into_response()
            }
            _ => error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        },
        Err(e) => classify_write_error("update_content", e),
    }
}

fn delete_content_api(state: &Arc<AppState>, content_type: ContentType, slug: &str) -> Response {
    let id = match resolve_content_id(state, content_type, slug) {
        Ok(id) => id,
        Err(resp) => return *resp,
    };

    match content::delete_content(&state.db, id) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("API delete_content error: {}", e);
            error_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
        }
    }
}

/// POST /api/v1/posts
pub async fn create_post(
    State(state): State<Arc<AppState>>,
    auth: ApiTokenWrite,
    Json(input): Json<CreateContent>,
) -> Response {
    create_content_api(&state, ContentType::Post, input, auth.0.created_by)
}

/// PUT /api/v1/posts/:slug
pub async fn update_post(
    State(state): State<Arc<AppState>>,
    auth: ApiTokenWrite,
    Path(slug): Path<String>,
    Json(input): Json<UpdateContent>,
) -> Response {
    update_content_api(&state, ContentType::Post, &slug, input, auth.0.created_by)
}

/// DELETE /api/v1/posts/:slug
pub async fn delete_post(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenWrite,
    Path(slug): Path<String>,
) -> Response {
    delete_content_api(&state, ContentType::Post, &slug)
}

/// POST /api/v1/pages
pub async fn create_page(
    State(state): State<Arc<AppState>>,
    auth: ApiTokenWrite,
    Json(input): Json<CreateContent>,
) -> Response {
    create_content_api(&state, ContentType::Page, input, auth.0.created_by)
}

/// PUT /api/v1/pages/:slug
pub async fn update_page(
    State(state): State<Arc<AppState>>,
    auth: ApiTokenWrite,
    Path(slug): Path<String>,
    Json(input): Json<UpdateContent>,
) -> Response {
    update_content_api(&state, ContentType::Page, &slug, input, auth.0.created_by)
}

/// DELETE /api/v1/pages/:slug
pub async fn delete_page(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenWrite,
    Path(slug): Path<String>,
) -> Response {
    delete_content_api(&state, ContentType::Page, &slug)
}

#[cfg(test)]
mod tests {
    use super::{paginate, MAX_API_PAGE};

    #[test]
    fn paginate_clamps_huge_pages_before_offset_math() {
        let (page, per_page, offset) = paginate(Some(usize::MAX), Some(100), 20, 50);

        assert_eq!(page, MAX_API_PAGE);
        assert_eq!(per_page, 50);
        assert_eq!(offset, (MAX_API_PAGE - 1) * 50);
    }
}

#[cfg(test)]
mod write_api_tests {
    use crate::services::api_token;
    use crate::web::routes;
    use crate::web::state::AppState;
    use crate::{Config, Database};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tower::ServiceExt;

    // A process-global counter guarantees each in-memory database gets a unique
    // shared-cache name even when parallel tests sample the clock in the same tick
    // (a colliding name would otherwise share one DB and stomp on each other's data).
    static UNIQUE: AtomicU64 = AtomicU64::new(0);

    fn unique_id() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let seq = UNIQUE.fetch_add(1, Ordering::Relaxed);
        format!("{}_{}", nanos, seq)
    }

    fn test_config() -> Config {
        let upload_dir = std::env::temp_dir().join(format!("pebble_api_uploads_{}", unique_id()));
        std::fs::create_dir_all(&upload_dir).unwrap();
        let toml = format!(
            r#"
[site]
title = "Test"
description = "Test"
url = "http://localhost:3000"

[server]
host = "127.0.0.1"
port = 3000

[database]
path = "data/pebble.db"

[content]
posts_per_page = 10
excerpt_length = 200

[media]
upload_dir = "{}"

[theme]
name = "default"

[auth]
session_lifetime = "7d"

[api]
enabled = true
"#,
            upload_dir.display()
        );
        toml::from_str(&toml).unwrap()
    }

    struct Harness {
        app: axum::Router,
        write_token: String,
        read_token: String,
    }

    fn setup() -> Harness {
        let db = Database::open_memory(&format!("api_write_{}", unique_id())).unwrap();
        db.migrate().unwrap();
        let (write_token, _) = api_token::create_token(&db, "writer", "write", None, None).unwrap();
        let (read_token, _) = api_token::create_token(&db, "reader", "read", None, None).unwrap();
        let state = AppState::new(test_config(), PathBuf::from("pebble.toml"), db, false).unwrap();
        let app = routes::api_routes().with_state(Arc::new(state));
        Harness {
            app,
            write_token,
            read_token,
        }
    }

    async fn send(
        app: &axum::Router,
        method: &str,
        uri: &str,
        token: Option<&str>,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(t) = token {
            builder = builder.header("authorization", format!("Bearer {}", t));
        }
        let req = match body {
            Some(b) => builder
                .header("content-type", "application/json")
                .body(Body::from(b.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let json = if bytes.is_empty() {
            serde_json::Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
        };
        (status, json)
    }

    #[tokio::test]
    async fn create_requires_a_write_scoped_token() {
        let h = setup();
        let body =
            serde_json::json!({"title": "Hello", "body_markdown": "# Hi", "status": "published"});

        let (status, _) = send(&h.app, "POST", "/api/v1/posts", None, Some(body.clone())).await;
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "no token must be rejected"
        );

        let (status, _) = send(
            &h.app,
            "POST",
            "/api/v1/posts",
            Some(&h.read_token),
            Some(body.clone()),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "read token cannot write");

        let (status, json) = send(
            &h.app,
            "POST",
            "/api/v1/posts",
            Some(&h.write_token),
            Some(body),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["data"]["title"], "Hello");
        assert_eq!(json["data"]["content_type"], "post");
    }

    #[tokio::test]
    async fn create_update_delete_lifecycle() {
        let h = setup();

        let (status, json) = send(
            &h.app,
            "POST",
            "/api/v1/posts",
            Some(&h.write_token),
            Some(serde_json::json!({"title": "First", "slug": "first", "body_markdown": "body", "status": "published"})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(json["data"]["slug"], "first");

        let (status, json) = send(
            &h.app,
            "PUT",
            "/api/v1/posts/first",
            Some(&h.write_token),
            Some(serde_json::json!({"title": "First (edited)"})),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["title"], "First (edited)");

        let (status, json) = send(
            &h.app,
            "GET",
            "/api/v1/posts/first",
            Some(&h.read_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["data"]["title"], "First (edited)");

        let (status, _) = send(
            &h.app,
            "DELETE",
            "/api/v1/posts/first",
            Some(&h.write_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NO_CONTENT);

        let (status, _) = send(
            &h.app,
            "GET",
            "/api/v1/posts/first",
            Some(&h.read_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND, "deleted post must be gone");
    }

    #[tokio::test]
    async fn invalid_slug_is_a_bad_request() {
        let h = setup();
        let (status, json) = send(
            &h.app,
            "POST",
            "/api/v1/posts",
            Some(&h.write_token),
            Some(
                serde_json::json!({"title": "Bad", "slug": "Invalid Slug!", "body_markdown": "x"}),
            ),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(json["message"]
            .as_str()
            .unwrap_or("")
            .to_lowercase()
            .contains("slug"));
    }

    #[tokio::test]
    async fn wrong_type_endpoint_does_not_touch_other_content() {
        let h = setup();
        let (status, _) = send(
            &h.app,
            "POST",
            "/api/v1/pages",
            Some(&h.write_token),
            Some(serde_json::json!({"title": "About", "slug": "about", "body_markdown": "x", "status": "published"})),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // Deleting a page through the posts endpoint must 404, not delete it.
        let (status, _) = send(
            &h.app,
            "DELETE",
            "/api/v1/posts/about",
            Some(&h.write_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, _) = send(
            &h.app,
            "GET",
            "/api/v1/pages/about",
            Some(&h.read_token),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "page must still exist");
    }
}
