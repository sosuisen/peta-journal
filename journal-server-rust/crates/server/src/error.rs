//! The error type of the services, and how it maps to HTTP responses.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Bad input from the caller (the Java `IllegalArgumentException`): HTTP 400.
    #[error("{0}")]
    BadRequest(String),
    /// Nothing at that address: HTTP 404.
    #[error("{0}")]
    NotFound(String),
    /// The request clashes with what is there, like a name another page has: HTTP 409.
    #[error("{0}")]
    Conflict(String),
    /// Anything else: HTTP 500.
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn bad_request(msg: impl Into<String>) -> AppError {
        AppError::BadRequest(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> AppError {
        AppError::NotFound(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> AppError {
        AppError::Conflict(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> AppError {
        AppError::Internal(anyhow::anyhow!(msg.into()))
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Internal(anyhow::Error::new(e))
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::Internal(anyhow::Error::new(e))
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::Internal(anyhow::Error::new(e))
    }
}

impl From<hmtid::HmtidError> for AppError {
    fn from(e: hmtid::HmtidError) -> Self {
        AppError::BadRequest(e.to_string())
    }
}

impl From<crate::domain::bounds::NotBounds> for AppError {
    fn from(e: crate::domain::bounds::NotBounds) -> Self {
        AppError::BadRequest(e.to_string())
    }
}

impl From<crate::domain::journal_name::NotAJournalName> for AppError {
    fn from(e: crate::domain::journal_name::NotAJournalName) -> Self {
        AppError::BadRequest(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
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

/// Parses an HMTID from request input; a bad ID is a 400.
pub fn parse_id(s: &str) -> AppResult<hmtid::Hmtid> {
    hmtid::Hmtid::parse(s).map_err(|e| AppError::BadRequest(format!("Invalid HMTID '{s}': {e}")))
}
