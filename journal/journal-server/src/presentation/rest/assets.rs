//! Image uploads into `{markdownDir}/assets` and serving them back.

use std::path::Path as FsPath;

use crate::error::AppError;
use crate::presentation::rest::handling;
use crate::state::{blocking, SharedState};
use anyhow::Context;
use axum::extract::{DefaultBodyLimit, Multipart, Path, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use journal_api_types::routes;
use journal_core::domain::asset_name::saved_name;
use serde::Serialize;

/// The largest upload. The default of 2MB refused large screenshots. A file is held in memory
/// while it is saved, so the limit stays. See docs/plans.md for what is still open about it.
const UPLOAD_LIMIT_BYTES: usize = 50 * 1024 * 1024;

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
    let result =
        blocking(move || Ok(axum::Json(save(&state, &original_name, &bytes)?).into_response()))
            .await;
    result.unwrap_or_else(|e| e.into_response())
}

/// A file saved among the assets: the name it has there, and the name it came with.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavedAsset {
    file_name: String,
    original_name: String,
}

/// Saves a file into `{markdownDir}/assets` under a new name. The MCP tool of the same
/// operation saves through here too (ADR 060).
pub(crate) fn save(
    state: &SharedState,
    original_name: &str,
    bytes: &[u8],
) -> Result<SavedAsset, AppError> {
    let markdown_dir = state.config.markdown_dir();
    if markdown_dir.trim().is_empty() {
        return Err(AppError::internal("Markdown directory not configured"));
    }
    let assets_dir = FsPath::new(&markdown_dir).join("assets");
    std::fs::create_dir_all(&assets_dir).context("Failed to create assets directory")?;
    let file_name = saved_name(&state.ids.generate_asset_id(), original_name);
    let target = assets_dir.join(&file_name);
    std::fs::write(&target, bytes).context("Failed to save file")?;
    tracing::info!("Saved asset: {}", target.display());
    Ok(SavedAsset {
        file_name,
        original_name: original_name.to_string(),
    })
}

/// Saves a file of this PC among the assets, for a caller that names the file and does not
/// send it. The path is absolute: the folder the server runs in means nothing to a caller.
pub(crate) fn save_file_at(state: &SharedState, path: &str) -> Result<SavedAsset, AppError> {
    let path = FsPath::new(path);
    let name = path.file_name().and_then(|name| name.to_str());
    let (true, Some(original_name)) = (path.is_absolute(), name) else {
        return Err(AppError::bad_request(
            "filePath must be the absolute path of a file",
        ));
    };
    let size = std::fs::metadata(path)
        .ok()
        .filter(|file| file.is_file())
        .map(|file| file.len())
        .ok_or_else(|| AppError::not_found(format!("No file at {}", path.display())))?;
    if size > UPLOAD_LIMIT_BYTES as u64 {
        return Err(AppError::bad_request("The file is larger than 50MB"));
    }
    let bytes = std::fs::read(path).context("Failed to read the file")?;
    save(state, original_name, &bytes)
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
