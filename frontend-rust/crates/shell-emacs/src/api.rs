//! Performs [`ApiOp`]s against the journal server. The core knows the requests and the answers.
//! The transport is behind [`Http`], so this can be tested without a server.

use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult};
use peta_core::rest::{rest_call, HttpReply, HttpRequest};

/// One HTTP exchange with the journal server.
pub trait Http {
    fn send(&self, request: &HttpRequest) -> Result<HttpReply, String>;
}

/// Performs one operation and answers it the way the core expects.
pub fn call<H: Http>(http: &H, op: ApiOp) -> ApiResult<ApiOutput> {
    // Why not a request: the only such operation is an upload, whose handle names a browser
    // `File`. Nothing in Emacs makes one
    let rest = rest_call(op).ok_or_else(|| ApiError("Emacs cannot upload files".into()))?;
    let reply = http.send(&rest.request).map_err(ApiError)?;
    rest.decode(&reply)
}
