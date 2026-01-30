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
        .route("/feed.xml", get(handlers::public::rss_feed))
        .route("/feed.json", get(handlers::public::json_feed))
        .route("/sitemap.xml", get(handlers::public::sitemap))
        .route("/media/:filename", get(handlers::public::serve_media))
        .route("/js/:filename", get(handlers::public::serve_js))
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
        .route("/admin/pages", get(handlers::admin::pages))
        .route("/admin/pages/new", get(handlers::admin::new_page))
        .route("/admin/pages", post(handlers::admin::create_page))
        .route("/admin/pages/:id/edit", get(handlers::admin::edit_page))
        .route("/admin/pages/:id", post(handlers::admin::update_page))
        .route(
            "/admin/pages/:id/delete",
            post(handlers::admin::delete_page),
        )
        .route("/admin/media", get(handlers::admin::media))
        .route(
            "/admin/media",
            post(handlers::admin::upload_media).layer(DefaultBodyLimit::max(10 * 1024 * 1024)),
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
        .route("/admin/users", get(handlers::admin::users))
        .route("/admin/users", post(handlers::admin::create_user))
        .route("/admin/users/:id", post(handlers::admin::update_user))
        .route(
            "/admin/users/:id/delete",
            post(handlers::admin::delete_user),
        )
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
