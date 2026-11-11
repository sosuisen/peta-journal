//! How an error of the journal core becomes an HTTP response.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

pub use journal_core::error::{parse_id, AppError, AppResult};

/// An error of the journal core, as a handler answers with it. The core knows no HTTP, so the
/// mapping to a status lives here.
#[derive(Debug)]
pub struct HttpError(pub AppError);

/// What a handler returns.
pub type HttpResult<T> = Result<T, HttpError>;

impl From<AppError> for HttpError {
    fn from(error: AppError) -> Self {
        HttpError(error)
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        match self.0 {
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, Json(json!({ "error": msg }))).into_response()
            }
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, Json(json!({ "error": msg }))).into_response()
            }
            AppError::Conflict(msg) => {
                (StatusCode::CONFLICT, Json(json!({ "error": msg }))).into_response()
            }
            AppError::Internal(e) => {
                tracing::error!("Internal error: {e:#}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response()
            }
        }
    }
}
