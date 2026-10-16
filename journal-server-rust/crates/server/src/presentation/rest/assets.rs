//! Image uploads into `{markdownDir}/assets` and serving them back.

use std::path::Path as FsPath;

use axum::extract::{Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;

use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/assets/upload", post(upload))
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
            return Ok(json_error(StatusCode::INTERNAL_SERVER_ERROR, "Markdown directory not configured"));
        }
        let assets_dir = FsPath::new(&markdown_dir).join("assets");
        if let Err(e) = std::fs::create_dir_all(&assets_dir) {
            tracing::error!("Failed to create assets directory: {e}");
            return Ok(json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to create assets directory"));
        }
        let mut name = if original_name.trim().is_empty() { "image.png".to_string() } else { original_name };
        // Sanitize filename
        name = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '_' })
            .collect();
        let saved_name = resolve_unique_name(&assets_dir, &name);
        let target = assets_dir.join(&saved_name);
        match std::fs::write(&target, &bytes) {
            Ok(_) => {
                tracing::info!("Saved asset: {}", target.display());
                Ok((
                    [(header::CONTENT_TYPE, "application/json")],
                    format!("{{\"fileName\":\"{saved_name}\"}}"),
                )
                    .into_response())
            }
            Err(e) => {
                tracing::error!("Failed to save asset: {e}");
                Ok(json_error(StatusCode::INTERNAL_SERVER_ERROR, "Failed to save file"))
            }
        }
    })
    .await;
    result.unwrap_or_else(|e| e.into_response())
}

/// If a file with the given name already exists, appends -1, -2, etc. before the extension.
fn resolve_unique_name(dir: &FsPath, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }
    let (base, ext) = match name.rfind('.') {
        Some(pos) if pos > 0 => (&name[..pos], &name[pos..]),
        _ => (name, ""),
    };
    let mut counter = 1;
    loop {
        let candidate = format!("{base}-{counter}{ext}");
        if !dir.join(&candidate).exists() {
            return candidate;
        }
        counter += 1;
    }
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
