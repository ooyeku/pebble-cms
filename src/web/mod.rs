mod error;
mod extractors;
mod handlers;
mod routes;
pub mod security;
mod state;

pub use state::AppState;

use crate::services::analytics::{
    extract_browser_family, extract_device_type, extract_referrer_domain, generate_session_hash,
    get_daily_salt, lookup_country, run_aggregation_job, Analytics, AnalyticsConfig, AnalyticsEvent,
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
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

pub async fn serve(config: Config, config_path: PathBuf, db: Database, addr: &str) -> Result<()> {
    let analytics_config = AnalyticsConfig::default();
    let analytics = Arc::new(Analytics::with_config(db.clone(), analytics_config));

    let state = AppState::new(config, config_path, db.clone(), false)?.with_analytics(analytics.clone());
    let state = Arc::new(state);

    let analytics_aggregator = analytics.clone();
    tokio::spawn(async move {
        run_aggregation_job(analytics_aggregator).await;
    });

    let app = Router::new()
        .merge(routes::public_routes())
        .merge(routes::admin_routes())
        .merge(routes::htmx_routes())
        .merge(routes::api_routes())
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

pub async fn serve_production(config: &Config, config_path: PathBuf, host: &str, port: u16) -> Result<()> {
    let db = Database::open(&config.database.path)?;

    let analytics_config = AnalyticsConfig::default();
    let analytics = Arc::new(Analytics::with_config(db.clone(), analytics_config));

    let state = AppState::new(config.clone(), config_path, db.clone(), true)?.with_analytics(analytics.clone());
    let state = Arc::new(state);

    let analytics_aggregator = analytics.clone();
    tokio::spawn(async move {
        run_aggregation_job(analytics_aggregator).await;
    });

    let app = Router::new()
        .merge(routes::public_routes())
        .merge(routes::api_routes())
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

    // Get DNT header before moving request
    let dnt_header = request
        .headers()
        .get("dnt")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // Check if we should track this request using analytics config
    if let Some(analytics) = &state.analytics {
        if !analytics.should_track(&path, dnt_header.as_deref()) {
            return next.run(request).await;
        }
    } else if should_skip_tracking(&path) {
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

    let ip = addr.ip().to_string();

    let response = next.run(request).await;

    if let Some(analytics) = &state.analytics {
        let daily_salt = get_daily_salt(&state.db).unwrap_or_else(|_| "default".to_string());
        let session_hash = generate_session_hash(&ip, &user_agent, &daily_salt);
        let response_time_ms = start.elapsed().as_millis() as i64;

        let (content_id, content_type) = extract_content_info(&path, &state.db);

        // Lookup country from IP (privacy-preserving: IP is not stored)
        let country_code = if analytics.config().geo_lookup {
            lookup_country(&ip)
        } else {
            None
        };

        let event = AnalyticsEvent {
            path: path.clone(),
            referrer_domain: extract_referrer_domain(&referrer),
            country_code,
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
