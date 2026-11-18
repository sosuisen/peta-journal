//! Opens an app through a custom URL scheme (for example `peta-journal://page/123`).

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use journal_api_types::routes;
use journal_api_types::LaunchUrlInput;
use serde::Serialize;

use crate::infrastructure::launcher;
use crate::presentation::rest::At;
use crate::state::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new().at(routes::LAUNCH_URL, launch)
}

#[derive(Serialize)]
struct LaunchResponse {
    success: bool,
    message: String,
}

fn reply(status: StatusCode, success: bool, message: &str) -> Response {
    (
        status,
        Json(LaunchResponse {
            success,
            message: message.to_string(),
        }),
    )
        .into_response()
}

async fn launch(Json(req): Json<LaunchUrlInput>) -> Response {
    let Some(url) = req.url.filter(|u| !u.is_empty()) else {
        return reply(StatusCode::BAD_REQUEST, false, "URL is required");
    };
    // Security: reject dangerous protocols
    let lower = url.to_lowercase();
    if lower.starts_with("javascript:") || lower.starts_with("data:") || lower.starts_with("file:")
    {
        tracing::warn!("Rejected dangerous URL: {url}");
        return reply(
            StatusCode::FORBIDDEN,
            false,
            "Dangerous protocols are not allowed",
        );
    }
    // Security: only allow protocol URLs (must contain ://)
    if !url.contains("://") {
        tracing::warn!("Rejected invalid URL: {url}");
        return reply(
            StatusCode::BAD_REQUEST,
            false,
            "URL must be a valid protocol URL (e.g., app://)",
        );
    }
    let ok = tokio::task::spawn_blocking(move || launcher::launch(&url))
        .await
        .unwrap_or(false);
    if ok {
        reply(StatusCode::OK, true, "App launched successfully")
    } else {
        reply(
            StatusCode::INTERNAL_SERVER_ERROR,
            false,
            "Failed to launch app",
        )
    }
}
