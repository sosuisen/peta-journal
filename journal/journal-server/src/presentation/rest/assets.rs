//! Image uploads into `{markdownDir}/assets` and serving them back.

use std::path::Path as FsPath;

use crate::presentation::rest::handling;
use crate::state::{blocking, SharedState};
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use journal_api_types::routes;
use journal_core::journal::ASSET_LIMIT_BYTES;

/// The largest upload, the same as the journal saves.
const UPLOAD_LIMIT_BYTES: usize = ASSET_LIMIT_BYTES;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route(
            routes::UPLOAD_ASSET.pattern,
            handling(routes::UPLOAD_ASSET, upload).layer(DefaultBodyLimit::max(UPLOAD_LIMIT_BYTES)),
        )
        .route("/assets/{file_name}", get(get_asset))
}

fn json_error(status: StatusCode, msg: &str) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        format!("{{\"error\":\"{msg}\"}}"),
    )
        .into_response()
}

async fn upload(State(state): State<SharedState>, mut multipart: Multipart) -> Response {
    let mut file: Option<(String, Vec<u8>)> = None;
    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() == Some("file") {
            let name = field.file_name().unwrap_or("").to_string();
            match field.bytes().await {
                Ok(bytes) => file = Some((name, bytes.to_vec())),
                Err(_) => return json_error(StatusCode::BAD_REQUEST, "Failed to read file"),
            }
            break;
        }
    }
    let Some((original_name, bytes)) = file else {
        return json_error(StatusCode::BAD_REQUEST, "file is required");
    };
    let result = blocking(move || {
        let saved = state.journal.save_asset(&original_name, &bytes)?;
        Ok(axum::Json(saved).into_response())
    })
    .await;
    result.unwrap_or_else(|e| e.into_response())
}

async fn get_asset(State(state): State<SharedState>, Path(file_name): Path<String>) -> Response {
    let markdown_dir = state.config.markdown_dir();
    if markdown_dir.trim().is_empty() {
        return StatusCode::NOT_FOUND.into_response();
    }
    // Prevent path traversal
    if file_name.contains("..") || file_name.contains('/') || file_name.contains('\\') {
        return StatusCode::BAD_REQUEST.into_response();
    }
    let file = FsPath::new(&markdown_dir).join("assets").join(&file_name);
    if !file.is_file() {
        return StatusCode::NOT_FOUND.into_response();
    }
    match tokio::fs::read(&file).await {
        Ok(data) => {
            let mime = mime_guess::from_path(&file).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.to_string())], data).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
