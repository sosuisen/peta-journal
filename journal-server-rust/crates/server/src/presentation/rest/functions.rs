//! Extension functions: lists them, lists their files and serves them.

use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::state::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/functions", get(list_functions))
        .route("/api/functions/{dir_name}", get(list_files))
        .route("/api/functions/{dir_name}/{*path}", get(serve_file))
}

#[derive(Serialize)]
struct FunctionInfo {
    name: String,
    url: String,
}

async fn list_functions(State(state): State<SharedState>) -> Json<Vec<FunctionInfo>> {
    Json(
        state
            .functions
            .list_functions()
            .iter()
            .map(|f| FunctionInfo {
                name: f.name.clone(),
                url: format!("/api/functions/{}/index.html", f.dir_name),
            })
            .collect(),
    )
}

/// The names of the files directly in a function's directory, sorted. A browser cannot list
/// a directory by itself, so a function that reads a set of its own files asks here.
async fn list_files(State(state): State<SharedState>, Path(dir_name): Path<String>) -> Response {
    if dir_name.contains("..") {
        return StatusCode::FORBIDDEN.into_response();
    }
    let dir = state.functions.functions_dir().join(&dir_name);
    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(_) => return StatusCode::NOT_FOUND.into_response(),
    };
    let mut names = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry.file_type().await.is_ok_and(|t| t.is_file()) {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    names.sort();
    Json(names).into_response()
}

fn guess_content_type(path: &str) -> &'static str {
    let lower = path.to_lowercase();
    if lower.ends_with(".html") {
        "text/html"
    } else if lower.ends_with(".js") {
        "application/javascript"
    } else if lower.ends_with(".css") {
        "text/css"
    } else if lower.ends_with(".json") {
        "application/json"
    } else if lower.ends_with(".md") {
        "text/markdown"
    } else if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

async fn serve_file(State(state): State<SharedState>, Path((dir_name, path)): Path<(String, String)>) -> Response {
    // Security: reject path traversal
    if dir_name.contains("..") || path.contains("..") {
        return StatusCode::FORBIDDEN.into_response();
    }
    let file = state.functions.functions_dir().join(&dir_name).join(&path);
    if !file.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(&file).await {
        Ok(content) => ([(header::CONTENT_TYPE, guess_content_type(&path))], content).into_response(),
        Err(_) => {
            tracing::error!("Failed to serve file: {dir_name}/{path}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
