use crate::models::UserRole;
use crate::services::auth;
use crate::web::error::AppResult;
use crate::web::state::AppState;
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tera::Context;
use time::Duration;

fn get_client_ip(headers: &HeaderMap, addr: Option<SocketAddr>) -> String {
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(s) = forwarded.to_str() {
            if let Some(ip) = s.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(s) = real_ip.to_str() {
            return s.to_string();
        }
    }

    addr.map(|a| a.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_csrf_cookie(jar: &CookieJar) -> Option<String> {
    jar.get("_csrf").map(|c| c.value().to_string())
}

pub async fn login_form(State(state): State<Arc<AppState>>, jar: CookieJar) -> AppResult<Response> {
    if !auth::has_users(&state.db)? {
        return Ok(Redirect::to("/admin/setup").into_response());
    }

    let csrf_token = state.csrf.generate();
    let mut ctx = Context::new();
    ctx.insert("csrf_token", &csrf_token);

    let html = state.templates.render("admin/login.html", &ctx)?;

    let csrf_cookie = Cookie::build(("_csrf", csrf_token.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(Duration::hours(1))
        .build();

    Ok((jar.add(csrf_cookie), Html(html)).into_response())
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
    csrf_token: Option<String>,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    headers: HeaderMap,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> AppResult<Response> {
    let client_key = get_client_ip(&headers, connect_info.map(|c| c.0));
    let csrf_cookie = get_csrf_cookie(&jar);

    let new_csrf = state.csrf.generate();
    let new_csrf_cookie = Cookie::build(("_csrf", new_csrf.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(Duration::hours(1))
        .build();

    if !state.rate_limiter.check(&client_key) {
        let mut ctx = Context::new();
        ctx.insert(
            "error",
            "Too many login attempts. Please try again in 15 minutes.",
        );
        ctx.insert("csrf_token", &new_csrf);
        let html = state.templates.render("admin/login.html", &ctx)?;
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            jar.add(new_csrf_cookie),
            Html(html),
        )
            .into_response());
    }

    let csrf_valid = match (&form.csrf_token, &csrf_cookie) {
        (Some(form_token), Some(cookie_token)) => state.csrf.validate(form_token, cookie_token),
        _ => {
            tracing::debug!("CSRF token missing from form or cookie");
            false
        }
    };

    if !csrf_valid {
        let mut ctx = Context::new();
        ctx.insert("error", "Invalid form submission. Please try again.");
        ctx.insert("csrf_token", &new_csrf);
        let html = state.templates.render("admin/login.html", &ctx)?;
        return Ok((StatusCode::FORBIDDEN, jar.add(new_csrf_cookie), Html(html)).into_response());
    }

    match auth::authenticate(&state.db, &form.username, &form.password)? {
        Some(user) => {
            state.rate_limiter.clear(&client_key);

            if let Some(old_session) = jar.get("session") {
                let _ = auth::delete_session(&state.db, old_session.value());
            }

            let token = auth::create_session(&state.db, user.id, 7)?;
            let session_cookie = Cookie::build(("session", token))
                .path("/")
                .http_only(true)
                .secure(!cfg!(debug_assertions))
                .same_site(SameSite::Lax)
                .max_age(Duration::days(7))
                .build();

            Ok((jar.add(session_cookie), Redirect::to("/admin")).into_response())
        }
        None => {
            state.rate_limiter.record_attempt(&client_key);

            let mut ctx = Context::new();
            ctx.insert("error", "Invalid username or password");
            ctx.insert("csrf_token", &new_csrf);
            let html = state.templates.render("admin/login.html", &ctx)?;
            Ok((
                StatusCode::UNAUTHORIZED,
                jar.add(new_csrf_cookie),
                Html(html),
            )
                .into_response())
        }
    }
}

pub async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> AppResult<Response> {
    if let Some(cookie) = jar.get("session") {
        let _ = auth::delete_session(&state.db, cookie.value());
    }

    let cookie = Cookie::build(("session", ""))
        .path("/")
        .max_age(Duration::ZERO)
        .build();

    Ok((jar.remove(cookie), Redirect::to("/admin/login")).into_response())
}

pub async fn setup_form(State(state): State<Arc<AppState>>, jar: CookieJar) -> AppResult<Response> {
    if auth::has_users(&state.db)? {
        return Ok(Redirect::to("/admin/login").into_response());
    }

    let csrf_token = state.csrf.generate();
    let mut ctx = Context::new();
    ctx.insert("csrf_token", &csrf_token);

    let html = state.templates.render("admin/setup.html", &ctx)?;

    let csrf_cookie = Cookie::build(("_csrf", csrf_token))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(Duration::hours(1))
        .build();

    Ok((jar.add(csrf_cookie), Html(html)).into_response())
}

#[derive(Deserialize)]
pub struct SetupForm {
    username: String,
    email: String,
    password: String,
    password_confirm: String,
    csrf_token: Option<String>,
}

pub async fn setup(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<SetupForm>,
) -> AppResult<Response> {
    if auth::has_users(&state.db)? {
        return Ok(Redirect::to("/admin/login").into_response());
    }

    let csrf_cookie = get_csrf_cookie(&jar);
    let new_csrf = state.csrf.generate();
    let new_csrf_cookie = Cookie::build(("_csrf", new_csrf.clone()))
        .path("/")
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .max_age(Duration::hours(1))
        .build();

    let csrf_valid = match (&form.csrf_token, &csrf_cookie) {
        (Some(form_token), Some(cookie_token)) => state.csrf.validate(form_token, cookie_token),
        _ => false,
    };

    if !csrf_valid {
        let mut ctx = Context::new();
        ctx.insert("error", "Invalid form submission. Please try again.");
        ctx.insert("csrf_token", &new_csrf);
        let html = state.templates.render("admin/setup.html", &ctx)?;
        return Ok((StatusCode::FORBIDDEN, jar.add(new_csrf_cookie), Html(html)).into_response());
    }

    if form.password != form.password_confirm {
        let mut ctx = Context::new();
        ctx.insert("error", "Passwords do not match");
        ctx.insert("csrf_token", &new_csrf);
        let html = state.templates.render("admin/setup.html", &ctx)?;
        return Ok((
            StatusCode::BAD_REQUEST,
            jar.add(new_csrf_cookie),
            Html(html),
        )
            .into_response());
    }

    let user_id = auth::create_user(
        &state.db,
        &form.username,
        &form.email,
        &form.password,
        UserRole::Admin,
    )?;
    let token = auth::create_session(&state.db, user_id, 7)?;

    let cookie = Cookie::build(("session", token))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax)
        .max_age(Duration::days(7))
        .build();

    Ok((jar.add(cookie), Redirect::to("/admin")).into_response())
}
