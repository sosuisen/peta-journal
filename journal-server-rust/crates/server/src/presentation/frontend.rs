//! The web frontend: the files Trunk writes to `dist/`, served from a folder (ADR 010).

use std::path::Path;

use axum::http::StatusCode;
use axum::routing::any;
use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

/// Serves the files of `dir` at every path that no route of `router` takes.
///
/// A path without a file gets `index.html`, as the app shows a page at a path like
/// `/pages/<name>`. A path under `/api/` does not: a caller of the API must not take the
/// HTML of the app for an answer.
pub fn serve(router: Router, dir: &Path) -> Router {
    let index = ServeFile::new(dir.join("index.html"));
    router
        .route("/api/{*path}", any(|| async { StatusCode::NOT_FOUND }))
        .fallback_service(ServeDir::new(dir).fallback(index))
}
