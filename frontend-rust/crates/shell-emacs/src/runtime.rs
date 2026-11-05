//! The real [`Host`]: threads for HTTP, timers and the event stream, and JSON lines on stdin and
//! stdout. The E2E test in `lisp/test/` runs this file, as a unit test would need the OS.

use crate::api::{call, Http, HttpResponse};
use crate::protocol::{Envelope, Outbound};
use crate::shell::{Host, Input, Now, Shell};
use chrono::{Datelike, Local, Timelike};
use peta_core::effects::{ApiOp, LocalDateTimeDto, SseEvent};
use serde_json::Value;
use std::io::{BufRead, Write};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};
use ureq::Agent;

/// How long typed text may take to reach the server once Emacs has gone away.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(5);

enum Message {
    Input(Input),
    EmacsClosed,
}

#[derive(Clone)]
struct UreqHttp {
    agent: Agent,
    server: String,
}

impl UreqHttp {
    fn new(server: String) -> UreqHttp {
        // A status above 299 is an answer here: `api` reads the server's message from it
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent();
        UreqHttp { agent, server }
    }
}

impl Http for UreqHttp {
    fn send(&self, method: &str, path: &str, body: Option<&Value>) -> Result<HttpResponse, String> {
        let url = format!("{}{path}", self.server);
        let result = match (method, body) {
            ("POST", Some(body)) => self.agent.post(&url).send_json(body),
            ("POST", None) => self.agent.post(&url).send_empty(),
            ("PUT", Some(body)) => self.agent.put(&url).send_json(body),
            ("PUT", None) => self.agent.put(&url).send_empty(),
            ("DELETE", _) => self.agent.delete(&url).call(),
            _ => self.agent.get(&url).call(),
        };
        let mut response = result.map_err(|e| e.to_string())?;
        let status = response.status().as_u16();
        let body = response
            .body_mut()
            .with_config()
            .limit(u64::MAX)
            .read_to_string()
            .map_err(|e| e.to_string())?;
        Ok(HttpResponse { status, body })
    }
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

    fn start_timer(&mut self, id: u64, ms: u32) {
        let to_main = self.to_main.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(ms.into()));
            let _ = to_main.send(Message::Input(Input::TimerFired { id }));
        });
    }

    fn connect_sse(&mut self, stream: u64) {
        let (http, to_main) = (self.http.clone(), self.to_main.clone());
        thread::spawn(move || read_events(&http, stream, &to_main));
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

/// Reads the server's event stream until it ends. The core decides when to connect again.
fn read_events(http: &UreqHttp, stream: u64, to_main: &Sender<Message>) {
    let send = |event| {
        let _ = to_main.send(Message::Input(Input::Sse { stream, event }));
    };
    let url = format!("{}/api/events", http.server);
    let response = http.agent.get(&url).call();
    let Ok(mut response) = response else {
        send(SseEvent::Error);
        return;
    };
    if !response.status().is_success() {
        send(SseEvent::Error);
        return;
    }
    send(SseEvent::Open);
    let reader =
        std::io::BufReader::new(response.body_mut().with_config().limit(u64::MAX).reader());
    for line in reader.lines() {
        let Ok(line) = line else { break };
        if let Some(data) = sse_data(&line) {
            send(SseEvent::Message(data.to_string()));
        }
    }
    send(SseEvent::Error);
}

/// The data of a line of an event stream. The server sends each message as one `data:` line.
pub fn sse_data(line: &str) -> Option<&str> {
    let data = line.strip_prefix("data:")?;
    Some(data.strip_prefix(' ').unwrap_or(data))
}

/// Reads the lines of Emacs until it closes stdin.
fn read_emacs(to_main: &Sender<Message>) {
    for line in std::io::stdin().lock().lines() {
        let Ok(line) = line else { break };
        match serde_json::from_str::<Envelope>(&line) {
            Ok(envelope) => {
                let _ = to_main.send(Message::Input(Input::Emacs(envelope)));
            }
            Err(e) => eprintln!("[shell-emacs] bad line from Emacs: {e}: {line}"),
        }
    }
    let _ = to_main.send(Message::EmacsClosed);
}

/// Runs the shell until Emacs has gone away and what it typed is saved.
pub fn run(server: String) {
    let (to_main, from_threads) = channel();
    let host = ThreadHost {
        http: UreqHttp::new(server),
        to_main: to_main.clone(),
        stdout: std::io::stdout(),
    };
    let mut shell = Shell::new(host);
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
            Ok(Message::EmacsClosed) => {}
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
        }
    }
    // Not by returning: dropping the core with commands still waiting panics in their destructors
    std::process::exit(0);
}
