use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        tracing::error!("Application error: {:?}", self.0);
        let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Error - Pebble CMS</title>
<style>
:root { --bg: #f8fafc; --text: #1e293b; }
@media (prefers-color-scheme: dark) { :root { --bg: #0f172a; --text: #f1f5f9; } }
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: system-ui, -apple-system, sans-serif; background: var(--bg); color: var(--text); min-height: 100vh; display: flex; align-items: center; justify-content: center; }
.error-card { text-align: center; max-width: 450px; padding: 2rem; }
h1 { font-size: 4rem; margin-bottom: 0.5rem; opacity: 0.3; }
h2 { margin-bottom: 1rem; }
p { color: #64748b; margin-bottom: 1.5rem; }
a { color: #2563eb; text-decoration: none; }
a:hover { text-decoration: underline; }
</style>
</head>
<body>
<div class="error-card">
<h1>500</h1>
<h2>Something went wrong</h2>
<p>An unexpected error occurred. Please try again or go back to the previous page.</p>
<a href="/admin">Back to Dashboard</a>
</div>
</body>
</html>"#;
        (StatusCode::INTERNAL_SERVER_ERROR, Html(html)).into_response()
    }
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

pub type AppResult<T> = Result<T, AppError>;
