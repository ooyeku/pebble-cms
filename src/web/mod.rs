mod error;
mod extractors;
mod handlers;
mod routes;
pub mod security;
mod state;

pub use state::AppState;

use crate::{Config, Database};
use anyhow::Result;
use axum::middleware;
use axum::Router;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;

pub async fn serve(config: Config, db: Database, addr: &str) -> Result<()> {
    let state = AppState::new(config, db, false)?;
    let state = Arc::new(state);

    let app = Router::new()
        .merge(routes::public_routes())
        .merge(routes::admin_routes())
        .merge(routes::htmx_routes())
        .layer(middleware::from_fn(security::apply_security_headers))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

pub async fn serve_production(config: &Config, host: &str, port: u16) -> Result<()> {
    let db = Database::open(&config.database.path)?;
    let state = AppState::new(config.clone(), db, true)?;
    let state = Arc::new(state);

    let app = Router::new()
        .merge(routes::public_routes())
        .merge(routes::production_fallback_routes())
        .layer(middleware::from_fn(security::apply_security_headers))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
