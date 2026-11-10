//! The real [`Host`]: the journal server over HTTP, and the clock of this machine.

use crate::shell::{Host, NativeShell};
use chrono::{Datelike, Local, Timelike};
use peta_core::effects::{ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent};
use peta_core::rest::ClientId;
use peta_server_client::{call, read_events, UreqHttp};
use std::thread;
use std::time::Duration;

struct JournalServer {
    http: UreqHttp,
}

impl Host for JournalServer {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        call(&self.http, op)
    }

    fn now(&self) -> (LocalDateTimeDto, u64) {
        let now = Local::now();
        let local = LocalDateTimeDto {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            hour: now.hour(),
            minute: now.minute(),
        };
        (local, now.timestamp_millis().unsigned_abs())
    }

    fn sleep(&self, ms: u32) {
        thread::sleep(Duration::from_millis(ms.into()));
    }

    fn events(&self, send: &mut dyn FnMut(SseEvent)) {
        read_events(&self.http, send);
    }
}

/// Starts the core for the journal server at `server`, such as `http://localhost:8080`.
/// `to_shell` gets the requests for the shell, and may be called from any thread.
pub fn connect(server: String, to_shell: impl Fn(Vec<u8>) + Send + Sync + 'static) -> NativeShell {
    // One ID for this shell: the server hears it in the requests, and the core at the start
    let client_id = ClientId::new();
    let host = JournalServer {
        http: UreqHttp::new(server, client_id.clone()),
    };
    NativeShell::new(host, client_id, to_shell)
}
