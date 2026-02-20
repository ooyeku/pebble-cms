use crate::models::{ContentStatus, ContentType};
use crate::services::{content, media, series, tags};
use crate::web::extractors::ApiTokenAuth;
use crate::web::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use std::sync::Arc;

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
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(default_size).min(max_size).max(1);
    let offset = (page - 1) * per_page;
    (page, per_page, offset)
}

fn json_envelope(data: serde_json::Value, total: i64, page: usize, per_page: usize) -> Json<serde_json::Value> {
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
                json_envelope(serde_json::to_value(&paginated).unwrap_or_default(), total, page, per_page).into_response()
            }
            Err(e) => {
                tracing::error!("API list_posts by tag error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
            }
        }
    } else {
        let total = content::count_content(&state.db, Some(ContentType::Post), Some(ContentStatus::Published)).unwrap_or(0);
        match content::list_published_content(&state.db, ContentType::Post, per_page, offset) {
            Ok(posts) => json_envelope(serde_json::to_value(&posts).unwrap_or_default(), total, page, per_page).into_response(),
            Err(e) => {
                tracing::error!("API list_posts error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
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
        Ok(Some(post)) if post.content.content_type == ContentType::Post && post.content.status == ContentStatus::Published => {
            json_single(serde_json::to_value(&post).unwrap_or_default()).into_response()
        }
        Ok(_) => not_found("Post not found"),
        Err(e) => {
            tracing::error!("API get_post error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
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
    let total = content::count_content(&state.db, Some(ContentType::Page), Some(ContentStatus::Published)).unwrap_or(0);

    match content::list_published_content(&state.db, ContentType::Page, per_page, offset) {
        Ok(pages) => json_envelope(serde_json::to_value(&pages).unwrap_or_default(), total, page, per_page).into_response(),
        Err(e) => {
            tracing::error!("API list_pages error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
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
        Ok(Some(page)) if page.content.content_type == ContentType::Page && page.content.status == ContentStatus::Published => {
            json_single(serde_json::to_value(&page).unwrap_or_default()).into_response()
        }
        Ok(_) => not_found("Page not found"),
        Err(e) => {
            tracing::error!("API get_page error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
        }
    }
}

/// GET /api/v1/tags
pub async fn list_tags(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
) -> Response {
    match tags::list_tags_with_counts(&state.db) {
        Ok(tags) => {
            let total = tags.len() as i64;
            json_envelope(serde_json::to_value(&tags).unwrap_or_default(), total, 1, total as usize).into_response()
        }
        Err(e) => {
            tracing::error!("API list_tags error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
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
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
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

    match series::list_series(&state.db, per_page, offset) {
        Ok(all_series) => {
            let total = all_series.len() as i64;
            json_envelope(serde_json::to_value(&all_series).unwrap_or_default(), total, page, per_page).into_response()
        }
        Err(e) => {
            tracing::error!("API list_series error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
        }
    }
}

/// GET /api/v1/series/:slug
pub async fn get_series_api(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
    Path(slug): Path<String>,
) -> Response {
    match series::get_series_by_slug(&state.db, &slug) {
        Ok(Some(s)) => {
            // Get the full series with items
            match series::list_series(&state.db, 1000, 0) {
                Ok(all) => {
                    let with_items = all.into_iter().find(|si| si.series.slug == slug);
                    match with_items {
                        Some(si) => json_single(serde_json::to_value(&si).unwrap_or_default()).into_response(),
                        None => json_single(serde_json::to_value(&s).unwrap_or_default()).into_response(),
                    }
                }
                Err(_) => json_single(serde_json::to_value(&s).unwrap_or_default()).into_response(),
            }
        }
        Ok(None) => not_found("Series not found"),
        Err(e) => {
            tracing::error!("API get_series error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
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

    match media::list_media(&state.db, per_page, offset) {
        Ok(media_list) => {
            let total = media_list.len() as i64;
            json_envelope(serde_json::to_value(&media_list).unwrap_or_default(), total, page, per_page).into_response()
        }
        Err(e) => {
            tracing::error!("API list_media error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": "Internal server error"}))).into_response()
        }
    }
}

/// GET /api/v1/site
pub async fn site_info(
    State(state): State<Arc<AppState>>,
    _auth: ApiTokenAuth,
) -> Response {
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
