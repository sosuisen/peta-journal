//! The ID that a client names itself with on every change request (ADR-015).

use std::str::FromStr;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::HeaderMap;

use crate::error::{AppError, AppResult};

/// The header that carries the ID.
pub const HEADER: &str = "x-client-id";

/// Where the hyphens of a UUID are.
const HYPHENS: [usize; 4] = [8, 13, 18, 23];
const UUID_LEN: usize = 36;

/// A UUID that one running client made for itself. It only tells the clients apart: anyone can
/// send any ID, so it proves nothing about who the client is.
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
        let is_uuid = s.len() == UUID_LEN
            && s.bytes().enumerate().all(|(at, byte)| {
                if HYPHENS.contains(&at) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            });
        if is_uuid {
            Ok(ClientId(s.to_string()))
        } else {
            Err(AppError::bad_request(
                "The X-Client-Id header must be a UUID, \
                 like 123e4567-e89b-42d3-a456-426614174000",
            ))
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

#[cfg(test)]
mod tests {
    use super::*;

    // ハイフン付きの36文字のUUIDは、大文字でも小文字でもIDとして読める。
    #[test]
    fn a_uuid_with_hyphens_is_an_id_in_either_case() {
        for text in [
            "123e4567-e89b-42d3-a456-426614174000",
            "123E4567-E89B-42D3-A456-426614174000",
        ] {
            assert_eq!(text.parse::<ClientId>().unwrap().as_str(), text);
        }
    }

    // 長さが違う、ハイフンの位置が違う、16進数でない文字がある文字列は、IDとして読めない。
    #[test]
    fn a_text_that_is_not_shaped_as_a_uuid_is_no_id() {
        for text in [
            "",
            "123e4567-e89b-42d3-a456-42661417400",
            "123e4567-e89b-42d3-a456-4266141740000",
            "123e4567e89b42d3a456426614174000",
            "123e4567+e89b-42d3-a456-426614174000",
            "123e4567-e89b+42d3-a456-426614174000",
            "123e4567-e89b-42d3+a456-426614174000",
            "123e4567-e89b-42d3-a456+426614174000",
            "123e4567-e89b-42d3-a456-42661417400g",
            "123e4567-e89b-42d3--456-426614174000",
        ] {
            assert!(text.parse::<ClientId>().is_err(), "{text}");
        }
    }
}
