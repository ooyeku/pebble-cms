use crate::models::User;
use crate::services::audit::AuditContext;
use crate::services::auth;
use crate::web::state::AppState;
use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::header;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Redirect, Response};
use axum_extra::extract::CookieJar;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

/// Rejection type for CurrentUser extractor — redirects to login instead of bare 401.
pub struct AuthRedirect;

impl IntoResponse for AuthRedirect {
    fn into_response(self) -> Response {
        Redirect::to("/admin/login").into_response()
    }
}

pub struct CurrentUser(pub User);

impl FromRequestParts<Arc<AppState>> for CurrentUser {
    type Rejection = AuthRedirect;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 Arc<AppState>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let state = state.clone();
        let headers = parts.headers.clone();
        Box::pin(async move {
            let cookies = CookieJar::from_headers(&headers);
            let token = cookies
                .get("session")
                .map(|c| c.value().to_string())
                .ok_or(AuthRedirect)?;

            let user = auth::validate_session(&state.db, &token)
                .map_err(|_| AuthRedirect)?
                .ok_or(AuthRedirect)?;

            Ok(CurrentUser(user))
        })
    }
}

pub struct OptionalUser(pub Option<User>);

impl FromRequestParts<Arc<AppState>> for OptionalUser {
    type Rejection = StatusCode;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        state: &'life1 Arc<AppState>,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let state = state.clone();
        let headers = parts.headers.clone();
        Box::pin(async move {
            let cookies = CookieJar::from_headers(&headers);
            let token = cookies.get("session").map(|c| c.value().to_string());

            let user = match token {
                Some(t) => auth::validate_session(&state.db, &t).ok().flatten(),
                None => None,
            };

            Ok(OptionalUser(user))
        })
    }
}

pub struct HxRequest(pub bool);

impl<S> FromRequestParts<S> for HxRequest
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let is_htmx = parts.headers.get("HX-Request").is_some();
        Box::pin(async move { Ok(HxRequest(is_htmx)) })
    }
}

pub struct AuditInfo(pub AuditContext);

impl<S> FromRequestParts<S> for AuditInfo
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    fn from_request_parts<'life0, 'life1, 'async_trait>(
        parts: &'life0 mut Parts,
        _state: &'life1 S,
    ) -> Pin<Box<dyn Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        'life1: 'async_trait,
        Self: 'async_trait,
    {
        let ip = parts
            .headers
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .or_else(|| {
                parts
                    .headers
                    .get("x-real-ip")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                parts
                    .extensions
                    .get::<ConnectInfo<SocketAddr>>()
                    .map(|ci| ci.0.ip().to_string())
            });

        // Extract User Agent
        let user_agent = parts
            .headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            Ok(AuditInfo(AuditContext {
                user_id: None,
                username: None,
                user_role: None,
                ip_address: ip,
                user_agent,
            }))
        })
    }
}
