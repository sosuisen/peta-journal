//! How each [`ApiOp`] travels over the journal server's REST API. This knows the requests and the
//! answers, and sends nothing: the shells do that, each with its own HTTP client.

use crate::effects::{ApiOp, ApiOutput, ApiResult};
use serde_json::Value;

/// One request to the journal server. `path` starts at `/api`.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub path: String,
    pub body: Option<Value>,
}

/// The answer of the server, whatever its status.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpReply {
    pub status: u16,
    pub body: String,
}

/// The request of an operation and how to read its answer.
pub struct RestCall {
    pub request: HttpRequest,
    read: fn(&HttpReply) -> ApiResult<ApiOutput>,
}

impl RestCall {
    pub fn decode(&self, reply: &HttpReply) -> ApiResult<ApiOutput> {
        (self.read)(reply)
    }
}

/// The REST call of an operation. None when only the shell can do it: an upload needs a file that
/// the shell holds.
pub fn rest_call(_op: ApiOp) -> Option<RestCall> {
    None
}
