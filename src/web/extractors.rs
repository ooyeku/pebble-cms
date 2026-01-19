use crate::models::User;
use crate::services::auth;
use crate::web::state::AppState;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum_extra::extract::CookieJar;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

pub struct CurrentUser(pub User);

impl FromRequestParts<Arc<AppState>> for CurrentUser {
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
            let token = cookies
                .get("session")
                .map(|c| c.value().to_string())
                .ok_or(StatusCode::UNAUTHORIZED)?;

            let user = auth::validate_session(&state.db, &token)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
                .ok_or(StatusCode::UNAUTHORIZED)?;

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
