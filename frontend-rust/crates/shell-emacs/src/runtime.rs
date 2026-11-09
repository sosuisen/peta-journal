//! The real [`Host`]: threads for HTTP, timers and the event stream, and JSON lines on stdin and
//! stdout. The E2E test in `lisp/test/` runs this file, as a unit test would need the OS.

use crate::api::{call, upload, Http};
use crate::protocol::{Envelope, Outbound};
use crate::shell::{Host, Input, Now, Shell};
use chrono::{Datelike, Local, Timelike};
use peta_core::effects::{ApiOp, LocalDateTimeDto, SseEvent};
use peta_core::rest::{ClientId, HttpReply, HttpRequest};
use std::io::{BufRead, Write};
use std::sync::mpsc::{channel, RecvTimeoutError, Sender};
use std::thread;
use std::time::{Duration, Instant};
use ureq::Agent;

/// How long typed text may take to reach the server once Emacs has gone away.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(5);

/// Marks the ends of the form in an upload. It is long enough not to occur in an image.
const UPLOAD_BOUNDARY: &str = "peta-journal-emacs-upload-7f3a9c1e5b";

enum Message {
    Input(Input),
    EmacsClosed,
}

#[derive(Clone)]
struct UreqHttp {
    agent: Agent,
    server: String,
    /// Sent with every request. The server refuses a change without it (ADR-015).
    client_id: ClientId,
}

impl UreqHttp {
    fn new(server: String, client_id: ClientId) -> UreqHttp {
        // A status above 299 is an answer here: `api` reads the server's message from it
        let agent = Agent::config_builder()
            .http_status_as_error(false)
            .build()
            .new_agent();
        UreqHttp {
            agent,
            server,
            client_id,
        }
    }
}

impl Http for UreqHttp {
    fn send(&self, request: &HttpRequest) -> Result<HttpReply, String> {
        let url = format!("{}{}", self.server, request.path);
        let me = (ClientId::HEADER, self.client_id.as_str());
        let result = match (request.method, &request.body) {
            ("POST", Some(body)) => self.agent.post(&url).header(me.0, me.1).send_json(body),
            ("POST", None) => self.agent.post(&url).header(me.0, me.1).send_empty(),
            ("PUT", Some(body)) => self.agent.put(&url).header(me.0, me.1).send_json(body),
            ("PUT", None) => self.agent.put(&url).header(me.0, me.1).send_empty(),
            ("DELETE", _) => self.agent.delete(&url).header(me.0, me.1).call(),
            _ => self.agent.get(&url).call(),
        };
        read_reply(result)
    }

    fn upload(&self, file_name: &str, bytes: &[u8]) -> Result<HttpReply, String> {
        let url = format!("{}/api/assets/upload", self.server);
        let body = multipart_body(file_name, bytes);
        let result = self
            .agent
            .post(&url)
            .header(ClientId::HEADER, self.client_id.as_str())
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={UPLOAD_BOUNDARY}"),
            )
            .send(&body[..]);
        read_reply(result)
    }
}

fn read_reply(
    result: Result<ureq::http::Response<ureq::Body>, ureq::Error>,
) -> Result<HttpReply, String> {
    let mut response = result.map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    let body = response
        .body_mut()
        .with_config()
        .limit(u64::MAX)
        .read_to_string()
        .map_err(|e| e.to_string())?;
    Ok(HttpReply { status, body })
}

/// The form with one field, `file`, as the server's asset upload reads it.
fn multipart_body(file_name: &str, bytes: &[u8]) -> Vec<u8> {
    // These would end the header early. The server replaces them in the saved name anyway
    let file_name = file_name.replace(['"', '\r', '\n'], "_");
    let mut body = format!(
        "--{UPLOAD_BOUNDARY}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"{file_name}\"\r\n\
         Content-Type: application/octet-stream\r\n\r\n"
    )
    .into_bytes();
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{UPLOAD_BOUNDARY}--\r\n").as_bytes());
    body
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
            Ok(Message::EmacsClosed) => {}
            Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => break,
        }
    }
    // Not by returning: dropping the core with commands still waiting panics in their destructors
    std::process::exit(0);
}
