use axum::body::Body;
use axum::http::{header, Request, Response};
use axum::middleware::Next;
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

pub fn security_headers<B>(mut response: Response<B>) -> Response<B> {
    let headers = response.headers_mut();

    headers.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());

    headers.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());

    headers.insert(header::X_XSS_PROTECTION, "1; mode=block".parse().unwrap());

    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'; script-src 'self' 'unsafe-inline' https://unpkg.com; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".parse().unwrap(),
    );

    response
}

pub struct RateLimiter {
    attempts: RwLock<HashMap<String, Vec<Instant>>>,
    max_attempts: usize,
    #[allow(dead_code)]
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
        entry.retain(|t| now.duration_since(*t) < self.lockout);

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
            v.retain(|t| now.duration_since(*t) < self.lockout);
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
        !form_token.is_empty() && form_token == cookie_token
    }
}

pub async fn apply_security_headers(request: Request<Body>, next: Next) -> Response<Body> {
    let response = next.run(request).await;
    security_headers(response)
}
