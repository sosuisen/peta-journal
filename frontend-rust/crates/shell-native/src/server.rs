//! The real [`Host`]: the journal server over HTTP, and the clock of this machine.

use crate::shell::{Host, NativeShell};
use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent};
use peta_core::rest::ClientId;

struct JournalServer;

impl Host for JournalServer {
    fn call(&self, _op: ApiOp) -> ApiResult<ApiOutput> {
        Err(ApiError("not written yet".into()))
    }

    fn now(&self) -> (LocalDateTimeDto, u64) {
        let local = LocalDateTimeDto {
            year: 0,
            month: 0,
            day: 0,
            hour: 0,
            minute: 0,
        };
        (local, 0)
    }

    fn sleep(&self, _ms: u32) {}

    fn events(&self, _send: &mut dyn FnMut(SseEvent)) {}
}

/// Starts the core for the journal server at `server`, such as `http://localhost:8080`.
/// `to_shell` gets the requests for the shell, and may be called from any thread.
pub fn connect(server: String, to_shell: impl Fn(Vec<u8>) + Send + Sync + 'static) -> NativeShell {
    let _ = server;
    NativeShell::new(JournalServer, ClientId::new(), to_shell)
}
