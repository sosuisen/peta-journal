//! The web frontend: the files Trunk writes to `dist/`, served from a folder (ADR 010).

use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use axum::body::Body;
use axum::extract::Request;
use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::any;
use axum::Router;
use regex::Regex;
use tower::ServiceExt;
use tower_http::services::{ServeDir, ServeFile};

/// The hash Trunk puts into a name: `styles-9508fe9cb1262d3b.css`, `app-…_bg.wasm`, and the
/// folder `snippets/app-…/`.
static CONTENT_HASH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-[0-9a-f]{16}[._/]").unwrap());

/// Serves the files of `dir` at every path that no route of `router` takes.
///
/// A path without a file gets `index.html`, as the app shows a page at a path like
/// `/pages/<name>`. A path under `/api/` does not: a caller of the API must not take the
/// HTML of the app for an answer.
pub fn serve(router: Router, dir: &Path) -> Router {
    let dir = dir.to_path_buf();
    router
        .route("/api/{*path}", any(|| async { StatusCode::NOT_FOUND }))
        .fallback(move |request: Request| file(dir.clone(), request))
}

async fn file(dir: PathBuf, request: Request) -> Response {
    let cache_control = if CONTENT_HASH.is_match(request.uri().path()) {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let index = ServeFile::new(dir.join("index.html"));
    let Ok(mut response) = ServeDir::new(dir).fallback(index).oneshot(request).await;
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static(cache_control));
    response.map(Body::new)
}
