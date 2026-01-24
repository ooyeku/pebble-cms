mod error;
mod extractors;
mod handlers;
mod routes;
pub mod security;
mod state;

pub use state::AppState;

use crate::services::analytics::{
    extract_browser_family, extract_device_type, extract_referrer_domain, generate_session_hash,
    get_daily_salt, run_aggregation_job, Analytics, AnalyticsEvent,
};
use crate::{Config, Database};
use anyhow::Result;
use axum::body::Body;
use axum::extract::{ConnectInfo, State};
use axum::http::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

pub async fn serve(config: Config, db: Database, addr: &str) -> Result<()> {
    let analytics = Arc::new(Analytics::new(db.clone()));

    let state = AppState::new(config, db.clone(), false)?.with_analytics(analytics.clone());
    let state = Arc::new(state);

    let analytics_aggregator = analytics.clone();
    tokio::spawn(async move {
        run_aggregation_job(analytics_aggregator).await;
    });

    let app = Router::new()
        .merge(routes::public_routes())
        .merge(routes::admin_routes())
        .merge(routes::htmx_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            analytics_middleware,
        ))
        .layer(middleware::from_fn(security::apply_security_headers))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn serve_production(config: &Config, host: &str, port: u16) -> Result<()> {
    let db = Database::open(&config.database.path)?;

    let analytics = Arc::new(Analytics::new(db.clone()));

    let state = AppState::new(config.clone(), db.clone(), true)?.with_analytics(analytics.clone());
    let state = Arc::new(state);

    let analytics_aggregator = analytics.clone();
    tokio::spawn(async move {
        run_aggregation_job(analytics_aggregator).await;
    });

    let app = Router::new()
        .merge(routes::public_routes())
        .merge(routes::production_fallback_routes())
        .layer(middleware::from_fn_with_state(
            state.clone(),
            analytics_middleware,
        ))
        .layer(middleware::from_fn(security::apply_security_headers))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    axum::serve(listener, app).await?;

    Ok(())
}

async fn analytics_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = request.uri().path().to_string();

    if should_skip_tracking(&path) {
        return next.run(request).await;
    }

    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let referrer = request
        .headers()
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let dnt = request
        .headers()
        .get("dnt")
        .and_then(|v| v.to_str().ok())
        .map(|v| v == "1")
        .unwrap_or(false);

    let ip = addr.ip().to_string();

    let response = next.run(request).await;

    if dnt {
        return response;
    }

    if let Some(analytics) = &state.analytics {
        let daily_salt = get_daily_salt(&state.db).unwrap_or_else(|_| "default".to_string());
        let session_hash = generate_session_hash(&ip, &user_agent, &daily_salt);
        let response_time_ms = start.elapsed().as_millis() as i64;

        let (content_id, content_type) = extract_content_info(&path, &state.db);

        let event = AnalyticsEvent {
            path: path.clone(),
            referrer_domain: extract_referrer_domain(&referrer),
            country_code: None,
            device_type: extract_device_type(&user_agent),
            browser_family: extract_browser_family(&user_agent),
            session_hash,
            response_time_ms: Some(response_time_ms),
            status_code: response.status().as_u16(),
            content_id,
            content_type,
        };

        // Record event immediately for real-time analytics
        if let Err(e) = analytics.record_event(&event) {
            tracing::error!("Failed to record analytics event: {}", e);
        }
    }

    response
}

fn should_skip_tracking(path: &str) -> bool {
    let skip_prefixes = ["/static", "/media", "/admin", "/api", "/htmx", "/_"];
    let skip_exact = ["/robots.txt", "/favicon.ico", "/health", "/sitemap.xml"];

    skip_prefixes.iter().any(|p| path.starts_with(p))
        || skip_exact.contains(&path)
        || path.ends_with(".css")
        || path.ends_with(".js")
        || path.ends_with(".png")
        || path.ends_with(".jpg")
        || path.ends_with(".ico")
        || path.ends_with(".woff")
        || path.ends_with(".woff2")
}

fn extract_content_info(path: &str, db: &Database) -> (Option<i64>, Option<String>) {
    if path.starts_with("/posts/") {
        let slug = path.trim_start_matches("/posts/");
        if let Ok(conn) = db.get() {
            if let Ok((id, content_type)) = conn.query_row(
                "SELECT id, content_type FROM content WHERE slug = ?1",
                [slug],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            ) {
                return (Some(id), Some(content_type));
            }
        }
    } else if path.starts_with("/pages/") {
        let slug = path.trim_start_matches("/pages/");
        if let Ok(conn) = db.get() {
            if let Ok((id, content_type)) = conn.query_row(
                "SELECT id, content_type FROM content WHERE slug = ?1",
                [slug],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            ) {
                return (Some(id), Some(content_type));
            }
        }
    }
    (None, None)
}
