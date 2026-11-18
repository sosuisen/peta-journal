//! The error type of the services. The journal server maps it to HTTP responses.

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

impl AppError {
    /// The HTTP status of the error. An app that embeds the core has no HTTP, and still
    /// answers its frontend with this number: the frontend reads the answers of the REST API.
    pub fn http_status(&self) -> u16 {
        match self {
            AppError::BadRequest(_) => 400,
            AppError::NotFound(_) => 404,
            AppError::Conflict(_) => 409,
            AppError::Internal(_) => 500,
        }
    }

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

impl From<crate::domain::client_id::NotAClientId> for AppError {
    fn from(e: crate::domain::client_id::NotAClientId) -> Self {
        AppError::BadRequest(e.to_string())
    }
}

impl From<crate::domain::journal_name::NotAJournalName> for AppError {
    fn from(e: crate::domain::journal_name::NotAJournalName) -> Self {
        AppError::BadRequest(e.to_string())
    }
}

/// Parses an HMTID from request input; a bad ID is a 400.
pub fn parse_id(s: &str) -> Result<hmtid::Hmtid, AppError> {
    hmtid::Hmtid::parse(s).map_err(|e| AppError::BadRequest(format!("Invalid HMTID '{s}': {e}")))
}
