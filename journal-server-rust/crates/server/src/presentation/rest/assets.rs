//! Image uploads into `{markdownDir}/assets` and serving them back.

use std::path::Path as FsPath;

use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use hmtid::Hmtid;

use crate::state::{blocking, SharedState};

/// The largest upload. The default of 2MB refused large screenshots. A file is held in memory
/// while it is saved, so the limit stays. See docs/plans.md for what is still open about it.
const UPLOAD_LIMIT_BYTES: usize = 50 * 1024 * 1024;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/api/assets/upload",
            post(upload).layer(DefaultBodyLimit::max(UPLOAD_LIMIT_BYTES)),
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
        let markdown_dir = state.config.markdown_dir();
        if markdown_dir.trim().is_empty() {
            return Ok(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Markdown directory not configured",
            ));
        }
        let assets_dir = FsPath::new(&markdown_dir).join("assets");
        if let Err(e) = std::fs::create_dir_all(&assets_dir) {
            tracing::error!("Failed to create assets directory: {e}");
            return Ok(json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to create assets directory",
            ));
        }
        let saved_name = saved_name(&state.ids.generate_asset_id(), &original_name);
        let target = assets_dir.join(&saved_name);
        match std::fs::write(&target, &bytes) {
            Ok(_) => {
                tracing::info!("Saved asset: {}", target.display());
                let answer = serde_json::json!({
                    "fileName": saved_name,
                    "originalName": original_name,
                });
                Ok(axum::Json(answer).into_response())
            }
            Err(e) => {
                tracing::error!("Failed to save asset: {e}");
                Ok(json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to save file",
                ))
            }
        }
    })
    .await;
    result.unwrap_or_else(|e| e.into_response())
}

/// The name an upload is saved under: the ID and the extension of the name it came with.
/// Why not that name itself: names collide, and the characters of many names cannot stay in
/// a file name that is also part of a URL. The link in the body keeps the name. See ADR-020.
fn saved_name(id: &Hmtid, original_name: &str) -> String {
    match plain_extension(original_name) {
        Some(extension) => format!("{id}.{extension}"),
        None => id.to_string(),
    }
}

/// The extension of `name` in lower case, when it is made of ASCII letters and digits.
fn plain_extension(name: &str) -> Option<String> {
    let (_, extension) = name.rsplit_once('.')?;
    let plain = !extension.is_empty() && extension.chars().all(|c| c.is_ascii_alphanumeric());
    plain.then(|| extension.to_ascii_lowercase())
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
