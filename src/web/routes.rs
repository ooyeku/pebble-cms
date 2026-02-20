use super::handlers;
use super::state::AppState;
use axum::extract::DefaultBodyLimit;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

pub fn public_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(handlers::public::index))
        .route("/posts", get(handlers::public::posts))
        .route("/posts/:slug", get(handlers::public::post))
        .route("/pages/:slug", get(handlers::public::page))
        .route("/tags", get(handlers::public::tags))
        .route("/tags/:slug", get(handlers::public::tag))
        .route("/search", get(handlers::public::search))
        .route("/series/:slug", get(handlers::public::series))
        .route("/feed.xml", get(handlers::public::rss_feed))
        .route("/feed.json", get(handlers::public::json_feed))
        .route(
            "/tags/:slug/feed.xml",
            get(handlers::public::tag_rss_feed),
        )
        .route("/sitemap.xml", get(handlers::public::sitemap))
        .route("/media/:filename", get(handlers::public::serve_media))
        .route("/js/:filename", get(handlers::public::serve_js))
        .route("/health", get(handlers::public::health))
        .route(
            "/preview/:token",
            get(handlers::public::draft_preview),
        )
}

pub fn admin_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin/login", get(handlers::auth::login_form))
        .route("/admin/login", post(handlers::auth::login))
        .route("/admin/logout", post(handlers::auth::logout))
        .route("/admin/setup", get(handlers::auth::setup_form))
        .route("/admin/setup", post(handlers::auth::setup))
        .route("/admin", get(handlers::admin::dashboard))
        .route("/admin/posts", get(handlers::admin::posts))
        .route("/admin/posts/new", get(handlers::admin::new_post))
        .route("/admin/posts", post(handlers::admin::create_post))
        .route("/admin/posts/:id/edit", get(handlers::admin::edit_post))
        .route("/admin/posts/:id", post(handlers::admin::update_post))
        .route(
            "/admin/posts/:id/delete",
            post(handlers::admin::delete_post),
        )
        // Post version routes
        .route(
            "/admin/posts/:id/versions",
            get(handlers::admin::post_versions),
        )
        .route(
            "/admin/posts/:id/versions/:vid",
            get(handlers::admin::post_version_view),
        )
        .route(
            "/admin/posts/:id/versions/:vid/restore",
            post(handlers::admin::post_version_restore),
        )
        .route(
            "/admin/posts/:id/diff",
            get(handlers::admin::post_version_diff),
        )
        .route("/admin/pages", get(handlers::admin::pages))
        .route("/admin/pages/new", get(handlers::admin::new_page))
        .route("/admin/pages", post(handlers::admin::create_page))
        .route("/admin/pages/:id/edit", get(handlers::admin::edit_page))
        .route("/admin/pages/:id", post(handlers::admin::update_page))
        .route(
            "/admin/pages/:id/delete",
            post(handlers::admin::delete_page),
        )
        // Page version routes
        .route(
            "/admin/pages/:id/versions",
            get(handlers::admin::page_versions),
        )
        .route(
            "/admin/pages/:id/versions/:vid",
            get(handlers::admin::page_version_view),
        )
        .route(
            "/admin/pages/:id/versions/:vid/restore",
            post(handlers::admin::page_version_restore),
        )
        .route(
            "/admin/pages/:id/diff",
            get(handlers::admin::page_version_diff),
        )
        .route("/admin/media", get(handlers::admin::media))
        .route(
            "/admin/media",
            post(handlers::admin::upload_media).layer(DefaultBodyLimit::max(100 * 1024 * 1024)),
        )
        .route("/admin/media/:id", delete(handlers::admin::delete_media))
        .route("/admin/tags", get(handlers::admin::tags))
        .route("/admin/tags", post(handlers::admin::create_tag))
        .route("/admin/tags/:id", post(handlers::admin::update_tag))
        .route("/admin/tags/:id/delete", post(handlers::admin::delete_tag))
        .route("/admin/settings", get(handlers::admin::settings))
        .route("/admin/settings", post(handlers::admin::save_settings))
        .route(
            "/admin/settings/homepage",
            post(handlers::admin::save_homepage_settings),
        )
        .route("/admin/database", get(handlers::admin::database_dashboard))
        .route("/admin/database", post(handlers::admin::database_action))
        .route("/admin/analytics", get(handlers::admin::analytics))
        // Audit log routes
        .route("/admin/audit", get(handlers::admin::audit_logs))
        .route("/admin/audit/export", get(handlers::admin::audit_export))
        .route("/admin/audit/:id", get(handlers::admin::audit_log_detail))
        .route("/admin/users", get(handlers::admin::users))
        .route("/admin/users", post(handlers::admin::create_user))
        .route("/admin/users/:id", post(handlers::admin::update_user))
        .route(
            "/admin/users/:id/delete",
            post(handlers::admin::delete_user),
        )
        // Draft preview token generation
        .route(
            "/admin/preview/:id",
            post(handlers::admin::generate_preview_token),
        )
        // Series routes
        .route("/admin/series", get(handlers::admin::series_list))
        .route("/admin/series/new", get(handlers::admin::new_series))
        .route(
            "/admin/series",
            post(handlers::admin::create_series_handler),
        )
        .route(
            "/admin/series/:id/edit",
            get(handlers::admin::edit_series),
        )
        .route(
            "/admin/series/:id",
            post(handlers::admin::update_series_handler),
        )
        .route(
            "/admin/series/:id/delete",
            post(handlers::admin::delete_series_handler),
        )
        // Snippet routes
        .route("/admin/snippets", get(handlers::admin::snippets))
        .route("/admin/snippets/new", get(handlers::admin::new_snippet))
        .route(
            "/admin/snippets",
            post(handlers::admin::create_snippet),
        )
        .route(
            "/admin/snippets/:id/edit",
            get(handlers::admin::edit_snippet),
        )
        .route(
            "/admin/snippets/:id",
            post(handlers::admin::update_snippet),
        )
        .route(
            "/admin/snippets/:id/delete",
            post(handlers::admin::delete_snippet),
        )
        // Bulk operations
        .route("/admin/bulk", post(handlers::admin::bulk_action))
}

pub fn htmx_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/htmx/preview", post(handlers::htmx::preview))
        .route("/htmx/search", get(handlers::htmx::search))
        .route(
            "/htmx/tags/autocomplete",
            get(handlers::htmx::tag_autocomplete),
        )
        .route(
            "/htmx/analytics/realtime",
            get(handlers::admin::analytics_realtime),
        )
        .route(
            "/htmx/analytics/content",
            get(handlers::admin::analytics_content),
        )
}

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/analytics/export",
            get(handlers::admin::analytics_export),
        )
        .route(
            "/api/analytics/content/:id",
            get(handlers::admin::analytics_content_stats),
        )
}

async fn admin_not_available() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}

pub fn production_fallback_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/admin", get(admin_not_available))
        .route("/admin/*path", get(admin_not_available))
        .route("/admin/*path", post(admin_not_available))
        .route("/admin/*path", delete(admin_not_available))
        .route("/htmx/*path", get(admin_not_available))
        .route("/htmx/*path", post(admin_not_available))
}
