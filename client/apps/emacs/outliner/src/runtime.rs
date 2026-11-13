//! The real [`Host`]: threads for HTTP, timers and the event stream, and JSON lines on stdin and
//! stdout. The E2E test in `lisp/test/` runs this file, as a unit test would need the OS.

use crate::protocol::{Envelope, Outbound};
use crate::shell::{Host, Input, Now, Shell};
use chrono::{Datelike, Local, Timelike};
use client_api_http::{call, read_events, upload, UreqHttp};
use client_core::effects::{ApiOp, LocalDateTimeDto};
use client_core::rest::ClientId;
use std::io::{BufRead, Write};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::{Duration, Instant};

/// How long typed text may take to reach the server once Emacs has gone away.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(5);

enum Message {
    Input(Input),
    EmacsClosed,
}

struct ThreadHost {
    http: UreqHttp,
    to_main: Sender<Message>,
    stdout: std::io::Stdout,
}

impl Host for ThreadHost {
    fn emit(&mut self, message: Outbound) {
        let line = serde_json::to_string(&message).expect("a message is always valid JSON");
        let mut out = self.stdout.lock();
        // Emacs has gone away when this fails, and stdin tells the main loop about that
        let _ = writeln!(out, "{line}").and_then(|()| out.flush());
    }

    fn api(&mut self, id: u64, op: ApiOp) {
        let (http, to_main) = (self.http.clone(), self.to_main.clone());
        thread::spawn(move || {
            let result = call(&http, op);
            let _ = to_main.send(Message::Input(Input::ApiDone {
                id,
                result: Box::new(result),
            }));
        });
    }

    fn upload(&mut self, id: u64, path: String) {
        let (http, to_main) = (self.http.clone(), self.to_main.clone());
        thread::spawn(move || {
            let result = upload(&http, &path);
            let _ = to_main.send(Message::Input(Input::ApiDone {
                id,
                result: Box::new(result),
            }));
        });
    }

    fn start_timer(&mut self, id: u64, ms: u32) {
        let to_main = self.to_main.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(ms.into()));
            let _ = to_main.send(Message::Input(Input::TimerFired { id }));
        });
    }

    fn connect_sse(&mut self, stream: u64) {
        let (http, to_main) = (self.http.clone(), self.to_main.clone());
        thread::spawn(move || {
            read_events(&http, |event| {
                let _ = to_main.send(Message::Input(Input::Sse { stream, event }));
            });
        });
    }

    fn now(&self) -> Now {
        let now = Local::now();
        Now {
            local: LocalDateTimeDto {
                year: now.year(),
                month: now.month(),
                day: now.day(),
                hour: now.hour(),
                minute: now.minute(),
            },
            now_ms: now.timestamp_millis().unsigned_abs(),
        }
    }
}

/// Reads the lines of Emacs until it closes stdin.
fn read_emacs(to_main: &Sender<Message>) {
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        match serde_json::from_str::<Envelope>(&line) {
            Ok(envelope) => {
                let _ = to_main.send(Message::Input(Input::Emacs(envelope)));
            }
            Err(e) => eprintln!("[client-shell-emacs] bad line from Emacs: {e}: {line}"),
        }
    }
    let _ = to_main.send(Message::EmacsClosed);
}

/// Runs the shell until Emacs has gone away and what it typed is saved.
pub fn run(server: String) {
    let (to_main, from_threads) = channel();
    // One ID for this process: the server hears it in the requests, and the core from the shell
    let client_id = ClientId::new();
    let host = ThreadHost {
        http: UreqHttp::new(server, client_id.clone()),
        to_main: to_main.clone(),
        stdout: std::io::stdout(),
    };
    let mut shell = Shell::new(host, client_id);
    thread::spawn(move || read_emacs(&to_main));

    while let Ok(message) = from_threads.recv() {
        match message {
            Message::Input(input) => shell.handle(input),
            Message::EmacsClosed => {
                shell.shutdown();
                break;
            }
        }
    }
    let deadline = Instant::now() + SHUTDOWN_LIMIT;
    while !shell.is_idle() {
        let left = deadline.saturating_duration_since(Instant::now());
        match from_threads.recv_timeout(left) {
            Ok(Message::Input(input)) => shell.handle(input),
            // Emacs has closed already, so this is the end of the time or of the threads
            Ok(Message::EmacsClosed) | Err(_) => break,
        }
    }
    // Never dropped: dropping the core with commands still waiting panics in their destructors.
    // Why not `process::exit`: on Windows it skips what writes the coverage of this run
    std::mem::forget(shell);
}
