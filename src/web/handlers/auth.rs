use crate::models::UserRole;
use crate::services::auth;
use crate::web::error::AppResult;
use crate::web::state::AppState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use axum_extra::extract::cookie::{Cookie, CookieJar};
use serde::Deserialize;
use std::sync::Arc;
use tera::Context;
use time::Duration;

pub async fn login_form(State(state): State<Arc<AppState>>) -> AppResult<Response> {
    if !auth::has_users(&state.db)? {
        return Ok(Redirect::to("/admin/setup").into_response());
    }

    let ctx = Context::new();
    let html = state.templates.render("admin/login.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> AppResult<Response> {
    match auth::authenticate(&state.db, &form.username, &form.password)? {
        Some(user) => {
            let token = auth::create_session(&state.db, user.id, 7)?;
            let cookie = Cookie::build(("session", token))
                .path("/")
                .http_only(true)
                .max_age(Duration::days(7))
                .build();

            Ok((jar.add(cookie), Redirect::to("/admin")).into_response())
        }
        None => {
            let mut ctx = Context::new();
            ctx.insert("error", "Invalid username or password");
            let html = state.templates.render("admin/login.html", &ctx)?;
            Ok((StatusCode::UNAUTHORIZED, Html(html)).into_response())
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

pub async fn setup_form(State(state): State<Arc<AppState>>) -> AppResult<Response> {
    if auth::has_users(&state.db)? {
        return Ok(Redirect::to("/admin/login").into_response());
    }

    let ctx = Context::new();
    let html = state.templates.render("admin/setup.html", &ctx)?;
    Ok(Html(html).into_response())
}

#[derive(Deserialize)]
pub struct SetupForm {
    username: String,
    email: String,
    password: String,
    password_confirm: String,
}

pub async fn setup(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    Form(form): Form<SetupForm>,
) -> AppResult<Response> {
    if auth::has_users(&state.db)? {
        return Ok(Redirect::to("/admin/login").into_response());
    }

    if form.password != form.password_confirm {
        let mut ctx = Context::new();
        ctx.insert("error", "Passwords do not match");
        let html = state.templates.render("admin/setup.html", &ctx)?;
        return Ok((StatusCode::BAD_REQUEST, Html(html)).into_response());
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
        .max_age(Duration::days(7))
        .build();

    Ok((jar.add(cookie), Redirect::to("/admin")).into_response())
}
