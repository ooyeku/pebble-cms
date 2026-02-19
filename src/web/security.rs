use crate::web::state::AppState;
use axum::body::Body;
use axum::extract::{ConnectInfo, State};
use axum::http::header::HeaderValue;
use axum::http::{header, Method, Request, Response, StatusCode};
use axum::middleware::Next;
use axum::response::IntoResponse;
use axum_extra::extract::CookieJar;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

// Pre-computed header values to avoid runtime parsing and unwrap
static HEADER_NOSNIFF: Lazy<HeaderValue> = Lazy::new(|| HeaderValue::from_static("nosniff"));
static HEADER_DENY: Lazy<HeaderValue> = Lazy::new(|| HeaderValue::from_static("DENY"));
static HEADER_XSS_PROTECTION: Lazy<HeaderValue> =
    Lazy::new(|| HeaderValue::from_static("1; mode=block"));
static HEADER_REFERRER_POLICY: Lazy<HeaderValue> =
    Lazy::new(|| HeaderValue::from_static("strict-origin-when-cross-origin"));
// CSP with nonce placeholder — nonce is injected per-request
static HEADER_CSP_TEMPLATE: &str = "default-src 'self'; script-src 'self' https://unpkg.com; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'";

pub fn security_headers<B>(mut response: Response<B>) -> Response<B> {
    let headers = response.headers_mut();

    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HEADER_NOSNIFF.clone());
    headers.insert(header::X_FRAME_OPTIONS, HEADER_DENY.clone());
    headers.insert(header::X_XSS_PROTECTION, HEADER_XSS_PROTECTION.clone());
    headers.insert(header::REFERRER_POLICY, HEADER_REFERRER_POLICY.clone());

    // Only set CSP if not already set (nonce middleware may have already set it)
    if !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
        if let Ok(val) = HeaderValue::from_str(HEADER_CSP_TEMPLATE) {
            headers.insert(header::CONTENT_SECURITY_POLICY, val);
        }
    }

    response
}

pub struct RateLimiter {
    attempts: RwLock<HashMap<String, Vec<Instant>>>,
    max_attempts: usize,
    window: Duration,
    lockout: Duration,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(5, Duration::from_secs(300), Duration::from_secs(900))
    }
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window: Duration, lockout: Duration) -> Self {
        Self {
            attempts: RwLock::new(HashMap::new()),
            max_attempts,
            window,
            lockout,
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut attempts = self.attempts.write().unwrap();

        let entry = attempts.entry(key.to_string()).or_default();
        entry.retain(|t| now.duration_since(*t) < self.window);

        if entry.len() >= self.max_attempts {
            let oldest = entry.first().copied();
            if let Some(oldest_time) = oldest {
                if now.duration_since(oldest_time) < self.lockout {
                    return false;
                }
                entry.clear();
            }
        }

        true
    }

    pub fn record_attempt(&self, key: &str) {
        let mut attempts = self.attempts.write().unwrap();
        let entry = attempts.entry(key.to_string()).or_default();
        entry.push(Instant::now());
    }

    pub fn clear(&self, key: &str) {
        let mut attempts = self.attempts.write().unwrap();
        attempts.remove(key);
    }

    pub fn cleanup(&self) {
        let now = Instant::now();
        let mut attempts = self.attempts.write().unwrap();
        attempts.retain(|_, v| {
            v.retain(|t| now.duration_since(*t) < self.window);
            !v.is_empty()
        });
    }
}

pub struct CsrfManager;

impl Default for CsrfManager {
    fn default() -> Self {
        Self
    }
}

impl CsrfManager {
    pub fn generate(&self) -> String {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use rand::Rng;

        let mut bytes = [0u8; 32];
        rand::thread_rng().fill(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }

    pub fn validate(&self, form_token: &str, cookie_token: &str) -> bool {
        if form_token.is_empty() || cookie_token.is_empty() {
            return false;
        }
        if form_token.len() != cookie_token.len() {
            return false;
        }
        let result = form_token
            .bytes()
            .zip(cookie_token.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b));
        result == 0
    }
}

pub async fn apply_security_headers(request: Request<Body>, next: Next) -> Response<Body> {
    let response = next.run(request).await;
    security_headers(response)
}

/// Middleware that rate-limits write operations (POST/DELETE) on admin endpoints.
/// Keyed by session cookie so legitimate multi-user setups aren't penalized.
pub async fn write_rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // Only rate-limit write operations on admin routes (not login — that has its own limiter)
    let is_write = (method == Method::POST || method == Method::DELETE)
        && path.starts_with("/admin")
        && path != "/admin/login";

    if !is_write {
        return next.run(request).await;
    }

    // Key by session cookie or IP for unauthenticated requests
    let cookies = CookieJar::from_headers(request.headers());
    let key = cookies
        .get("session")
        .map(|c| format!("write:{}", c.value()))
        .unwrap_or_else(|| {
            let ip = connect_info
                .map(|ci| ci.0.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            format!("write:{}", ip)
        });

    if !state.write_rate_limiter.check(&key) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            "Too many write operations. Please slow down.",
        )
            .into_response();
    }

    state.write_rate_limiter.record_attempt(&key);
    next.run(request).await
}
