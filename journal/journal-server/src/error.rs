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
        if let AppError::Internal(e) = &self.0 {
            tracing::error!("Internal error: {e:#}");
        }
        let status = StatusCode::from_u16(self.0.http_status())
            .expect("the status of an error is a status of HTTP");
        (status, Json(json!({ "error": self.0.to_string() }))).into_response()
    }
}
