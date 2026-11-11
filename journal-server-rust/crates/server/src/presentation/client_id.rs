//! Reads the ID that a client names itself with from a change request (ADR-015).
//!
//! The middleware of the REST API reads it from the header and puts it on the request. A
//! handler that tells the other clients of its change takes it as `Extension<ClientId>`.

use axum::http::HeaderMap;

pub use journal_core::domain::client_id::ClientId;

use crate::error::{AppError, AppResult};

/// The header that carries the ID.
pub const HEADER: &str = "x-client-id";

/// The ID that the headers of a request name.
pub fn of(headers: &HeaderMap) -> AppResult<ClientId> {
    let value = headers.get(HEADER).ok_or_else(|| {
        AppError::bad_request(
            "The X-Client-Id header is required. \
             Send a UUID that this client made for itself when it started",
        )
    })?;
    // A value that is not text is not a UUID either
    Ok(value.to_str().unwrap_or_default().parse::<ClientId>()?)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers_with(id: &'static str) -> HeaderMap {
        HeaderMap::from_iter([(
            axum::http::HeaderName::from_static(HEADER),
            HeaderValue::from_static(id),
        )])
    }

    // ヘッダーのUUIDが、IDになる。
    #[test]
    fn the_uuid_of_the_header_is_the_id() {
        let written = "123e4567-e89b-42d3-a456-426614174000";

        let id = of(&headers_with(written)).unwrap();

        assert_eq!(id, written.parse().unwrap());
    }

    // ヘッダーがなければ、IDは読めない。
    #[test]
    fn no_header_gives_no_id() {
        let refusal = of(&HeaderMap::new()).unwrap_err();

        assert!(matches!(refusal, AppError::BadRequest(_)));
    }

    // UUIDでないヘッダーは、IDとして読めない。
    #[test]
    fn a_header_that_is_not_a_uuid_gives_no_id() {
        let refusal = of(&headers_with("emacs")).unwrap_err();

        assert!(matches!(refusal, AppError::BadRequest(_)));
    }
}
