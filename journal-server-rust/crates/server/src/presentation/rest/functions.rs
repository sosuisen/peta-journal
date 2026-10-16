//! Extension functions: lists them and serves their files.

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
