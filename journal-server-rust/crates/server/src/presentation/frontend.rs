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
use tower_http::set_header::SetResponseHeader;

/// The hash Trunk puts into a name: `styles-9508fe9cb1262d3b.css`, `app-…_bg.wasm`, and the
/// folder `snippets/app-…/`.
static CONTENT_HASH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"-[0-9a-f]{16}[._/]").unwrap());

/// The browser asks the server on every load whether the file changed.
const NO_CACHE: &str = "no-cache";

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
    let of_the_file = if CONTENT_HASH.is_match(request.uri().path()) {
        "public, max-age=31536000, immutable"
    } else {
        NO_CACHE
    };
    // A page can have a name that looks like a hashed file, so `index.html` says for itself
    // that it is not to be cached
    let index = SetResponseHeader::overriding(
        ServeFile::new(dir.join("index.html")),
        CACHE_CONTROL,
        HeaderValue::from_static(NO_CACHE),
    );
    let Ok(mut response) = ServeDir::new(dir).fallback(index).oneshot(request).await;
    response
        .headers_mut()
        .entry(CACHE_CONTROL)
        .or_insert(HeaderValue::from_static(of_the_file));
    response.map(Body::new)
}
