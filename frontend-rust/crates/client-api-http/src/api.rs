//! Performs [`ApiOp`]s against the journal server. The core knows the requests and the answers.
//! The transport is behind [`Http`], so this can be tested without a server.

use client_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult};
use client_core::model::Uploaded;
use client_core::rest::{rest_call, HttpReply, HttpRequest};
use std::path::Path;

/// One HTTP exchange with the journal server.
pub trait Http {
    fn send(&self, request: &HttpRequest) -> Result<HttpReply, String>;
    /// Sends a file as the multipart form of the server's asset upload.
    fn upload(&self, file_name: &str, bytes: &[u8]) -> Result<HttpReply, String>;
}

/// Performs one operation and answers it the way the core expects.
pub fn call<H: Http>(http: &H, op: ApiOp) -> ApiResult<ApiOutput> {
    // Why not a request: the only such operation is an upload, and only the shell knows its file.
    // The shell sends those through `upload`
    let rest = rest_call(op).ok_or_else(|| ApiError("not a REST call".into()))?;
    let reply = http.send(&rest.request).map_err(ApiError)?;
    rest.decode(&reply)
}

/// Sends the file at `path`, and answers with the name the server saved it under.
pub fn upload<H: Http>(http: &H, path: &str) -> ApiResult<ApiOutput> {
    let path = Path::new(path);
    let bytes = std::fs::read(path).map_err(|e| ApiError(format!("{}: {e}", path.display())))?;
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy())
        .unwrap_or_default();
    let reply = http.upload(&file_name, &bytes).map_err(ApiError)?;
    if !(200..300).contains(&reply.status) {
        return Err(ApiError(format!("Upload failed: {}", reply.status)));
    }
    let uploaded: Uploaded =
        serde_json::from_str(&reply.body).map_err(|e| ApiError(e.to_string()))?;
    Ok(ApiOutput::Uploaded(uploaded))
}
