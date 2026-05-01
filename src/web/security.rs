use crate::web::state::AppState;
use axum::body::Body;
use axum::extract::{ConnectInfo, State};
use axum::http::header::HeaderValue;
use axum::http::{header, HeaderMap, Method, Request, Response, StatusCode};
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
static HEADER_HSTS: Lazy<HeaderValue> =
    Lazy::new(|| HeaderValue::from_static("max-age=63072000; includeSubDomains"));
// Public pages: strict CSP — no inline scripts, all JS self-hosted
static HEADER_CSP_PUBLIC: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'";
// Admin pages: relaxed script-src to allow inline scripts in admin templates
static HEADER_CSP_ADMIN: &str = "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'; object-src 'none'";

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
        let mut attempts = self.attempts.write().unwrap_or_else(|e| e.into_inner());

        let entry = attempts.entry(key.to_string()).or_default();
        if entry.len() >= self.max_attempts {
            if let Some(last_attempt) = entry.last().copied() {
                if now.duration_since(last_attempt) < self.lockout {
                    return false;
                }
                entry.clear();
            }
        }

        entry.retain(|t| now.duration_since(*t) < self.window);
        true
    }

    pub fn record_attempt(&self, key: &str) {
        let mut attempts = self.attempts.write().unwrap_or_else(|e| e.into_inner());
        let entry = attempts.entry(key.to_string()).or_default();
        entry.push(Instant::now());
    }

    pub fn clear(&self, key: &str) {
        let mut attempts = self.attempts.write().unwrap_or_else(|e| e.into_inner());
        attempts.remove(key);
    }

    pub fn cleanup(&self) {
        let now = Instant::now();
        let mut attempts = self.attempts.write().unwrap_or_else(|e| e.into_inner());
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
        use rand::RngCore;

        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
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
    let is_admin = request.uri().path().starts_with("/admin");
    let mut response = next.run(request).await;

    let headers = response.headers_mut();
    headers.insert(header::X_CONTENT_TYPE_OPTIONS, HEADER_NOSNIFF.clone());
    headers.insert(header::X_FRAME_OPTIONS, HEADER_DENY.clone());
    headers.insert(header::X_XSS_PROTECTION, HEADER_XSS_PROTECTION.clone());
    headers.insert(header::REFERRER_POLICY, HEADER_REFERRER_POLICY.clone());
    headers.insert(header::STRICT_TRANSPORT_SECURITY, HEADER_HSTS.clone());

    if !headers.contains_key(header::CONTENT_SECURITY_POLICY) {
        let csp = if is_admin {
            HEADER_CSP_ADMIN
        } else {
            HEADER_CSP_PUBLIC
        };
        if let Ok(val) = HeaderValue::from_str(csp) {
            headers.insert(header::CONTENT_SECURITY_POLICY, val);
        }
    }

    response
}

fn same_origin(headers: &HeaderMap) -> bool {
    let Some(host) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return false;
    };

    if let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        return origin
            .strip_prefix("http://")
            .or_else(|| origin.strip_prefix("https://"))
            .is_some_and(|origin_host| origin_host == host);
    }

    if let Some(referer) = headers.get(header::REFERER).and_then(|v| v.to_str().ok()) {
        return referer.starts_with(&format!("http://{host}/"))
            || referer.starts_with(&format!("https://{host}/"));
    }

    false
}

/// Middleware that rejects cross-site admin writes. Browser form posts are
/// checked with Origin/Referer; script-driven HTMX/fetch requests can also send
/// an X-CSRF-Token matching the _csrf cookie.
pub async fn csrf_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let is_write = method == Method::POST || method == Method::DELETE;
    let is_protected_path = path.starts_with("/admin") || path.starts_with("/htmx");
    let has_own_csrf = path == "/admin/login" || path == "/admin/setup";

    if !is_write || !is_protected_path || has_own_csrf {
        return next.run(request).await;
    }

    let cookies = CookieJar::from_headers(request.headers());
    let cookie_token = cookies.get("_csrf").map(|c| c.value().to_string());
    let header_token = request
        .headers()
        .get("x-csrf-token")
        .and_then(|v| v.to_str().ok());

    let token_valid = match (header_token, cookie_token.as_deref()) {
        (Some(form_token), Some(cookie_token)) => state.csrf.validate(form_token, cookie_token),
        _ => false,
    };

    if token_valid || same_origin(request.headers()) {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "Invalid form submission").into_response()
    }
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

#[cfg(test)]
mod tests {
    use super::RateLimiter;
    use std::time::Duration;

    #[test]
    fn lockout_remains_active_after_attempt_window_expires() {
        let limiter = RateLimiter::new(2, Duration::from_millis(30), Duration::from_millis(120));

        limiter.record_attempt("login:1");
        limiter.record_attempt("login:1");

        assert!(!limiter.check("login:1"));
        std::thread::sleep(Duration::from_millis(45));
        assert!(!limiter.check("login:1"));
    }
}
