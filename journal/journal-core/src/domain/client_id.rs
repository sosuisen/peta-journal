//! The ID that a client names itself with on every change (ADR-015).

use std::fmt;
use std::str::FromStr;

/// A UUID that one running client made for itself, kept as the client wrote it. The clients
/// get it back in that form with the news of the change, so a client finds its own ID by
/// comparing text.
///
/// It only tells the clients apart: anyone can send any ID, so it proves nothing about who
/// the client is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientId(String);

impl ClientId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The text is not a UUID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAClientId;

impl fmt::Display for NotAClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "The X-Client-Id header must be a UUID, like 123e4567-e89b-42d3-a456-426614174000"
        )
    }
}

impl std::error::Error for NotAClientId {}

impl FromStr for ClientId {
    type Err = NotAClientId;

    fn from_str(s: &str) -> Result<ClientId, NotAClientId> {
        match uuid::Uuid::try_parse(s) {
            Ok(_) => Ok(ClientId(s.to_string())),
            Err(_) => Err(NotAClientId),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // UUIDは、書かれたとおりの文字でIDになる。
    #[test]
    fn a_uuid_is_the_id_as_it_is_written() {
        let written = "123E4567-e89b-42d3-a456-426614174000";

        let id: ClientId = written.parse().unwrap();

        assert_eq!(id.as_str(), written);
    }

    // UUIDでない文字列は、IDとして読めない。
    #[test]
    fn a_text_that_is_not_a_uuid_is_no_id() {
        assert_eq!("emacs".parse::<ClientId>(), Err(NotAClientId));
    }
}
