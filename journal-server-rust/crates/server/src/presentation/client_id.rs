//! The ID that a client names itself with on every change request (ADR-015).

use std::str::FromStr;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;

use crate::error::{AppError, AppResult};

/// The header that carries the ID.
pub const HEADER: &str = "x-client-id";

/// A UUID that one running client made for itself, kept as the client wrote it. The server
/// sends it back in that form, so the client finds its own ID by comparing text.
///
/// It only tells the clients apart: anyone can send any ID, so it proves nothing about who
/// the client is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientId(String);

impl ClientId {
    /// The ID that the headers of a request name.
    pub fn of(headers: &HeaderMap) -> AppResult<ClientId> {
        let value = headers.get(HEADER).ok_or_else(|| {
            AppError::bad_request(
                "The X-Client-Id header is required. \
                 Send a UUID that this client made for itself when it started",
            )
        })?;
        // A value that is not text is not a UUID either
        value.to_str().unwrap_or_default().parse()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ClientId {
    type Err = AppError;

    fn from_str(s: &str) -> AppResult<ClientId> {
        match uuid::Uuid::try_parse(s) {
            Ok(_) => Ok(ClientId(s.to_string())),
            Err(_) => Err(AppError::bad_request(
                "The X-Client-Id header must be a UUID, \
                 like 123e4567-e89b-42d3-a456-426614174000",
            )),
        }
    }
}

/// A handler that takes this tells the other clients who made its change.
impl<S: Send + Sync> FromRequestParts<S> for ClientId {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> AppResult<ClientId> {
        ClientId::of(&parts.headers)
    }
}
