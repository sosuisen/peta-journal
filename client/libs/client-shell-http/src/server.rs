//! The real [`Host`]: the journal server over HTTP.

use client_api_http::{call, read_events, upload, UreqHttp};
use client_core::effects::{ApiOp, ApiOutput, ApiResult, SseEvent};
use client_core::rest::ClientId;
use client_native_bridge::{Host, NativeShell, SystemClock};
use std::path::Path;

struct JournalServer {
    http: UreqHttp,
    /// The address of the server, where the web frontend and the assets are served too.
    server: String,
}

impl Host for JournalServer {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        call(&self.http, op)
    }

    fn upload(&self, path: &Path) -> ApiResult<ApiOutput> {
        upload(&self.http, &path.to_string_lossy())
    }

    fn events(&self, send: &mut dyn FnMut(SseEvent)) {
        read_events(&self.http, send);
    }

    fn app_url(&self) -> Option<String> {
        Some(self.server.clone())
    }
}

/// Starts the core for the journal server at `server`, such as `http://localhost:8080`, with
/// the clock of this machine. `to_shell` gets the requests for the shell, and may be called
/// from any thread.
pub fn connect(server: String, to_shell: impl Fn(Vec<u8>) + Send + Sync + 'static) -> NativeShell {
    // One ID for this shell: the server hears it in the requests, and the core at the start
    let client_id = ClientId::new();
    let host = JournalServer {
        http: UreqHttp::new(server.clone(), client_id.clone()),
        server,
    };
    NativeShell::new(host, SystemClock, client_id, to_shell)
}
