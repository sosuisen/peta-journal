//! The web frontend: the files Trunk writes to `dist/`, served from a folder (ADR 010).

use std::path::Path;

use axum::http::header::CACHE_CONTROL;
use axum::http::{HeaderValue, StatusCode};
use axum::routing::any;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_header::SetResponseHeader;

/// Serves the files of `dir` at every path that no route of `router` takes.
///
/// A path without a file gets `index.html`, as the app shows a page at a path like
/// `/pages/<name>`. A path under `/api/` does not: a caller of the API must not take the
/// HTML of the app for an answer.
pub fn serve(router: Router, dir: &Path) -> Router {
    let index = ServeFile::new(dir.join("index.html"));
    let files = SetResponseHeader::overriding(
        ServeDir::new(dir).fallback(index),
        CACHE_CONTROL,
        HeaderValue::from_static("no-cache"),
    );
    router
        .route("/api/{*path}", any(|| async { StatusCode::NOT_FOUND }))
        .fallback_service(files)
}
