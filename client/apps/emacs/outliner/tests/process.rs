//! Runs the real program, as Emacs does: JSON lines on its stdin and stdout, and HTTP to a
//! server. The server here is a small one of the test, so neither Emacs nor the journal server
//! is needed. This is the only Rust test of `src/runtime.rs` and `src/main.rs`.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, ChildStdin, Command, ExitStatus, Stdio};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// How long a test waits for the program or the server before it fails.
const PATIENCE: Duration = Duration::from_secs(20);

/// One request the server got.
#[derive(Clone, Debug)]
struct Seen {
    method: String,
    path: String,
    /// The names are in lower case.
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Seen {
    fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.as_str())
    }

    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

struct Answer {
    method: &'static str,
    path: String,
    /// None for a request that the server never answers.
    status: Option<u16>,
    body: String,
}

/// A line of the event stream, or its end.
enum Stream {
    Data(String),
    Cut,
}

/// A journal server that answers what a test tells it to, and 404 to the rest.
struct Server {
    address: String,
    seen: Arc<Mutex<Vec<Seen>>>,
    answers: Arc<Mutex<Vec<Answer>>>,
    stream: Sender<Stream>,
}

impl Server {
    fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let seen = Arc::new(Mutex::new(Vec::new()));
        let answers = Arc::new(Mutex::new(Vec::new()));
        let (stream, from_test) = channel();
        let from_test = Arc::new(Mutex::new(from_test));
        let (seen_here, answers_here) = (seen.clone(), answers.clone());
        thread::spawn(move || {
            for connection in listener.incoming().flatten() {
                let (seen, answers) = (seen_here.clone(), answers_here.clone());
                let from_test = from_test.clone();
                // One thread each: the event stream stays open while the other requests come
                thread::spawn(move || serve(connection, &seen, &answers, &from_test));
            }
        });
        Server {
            address,
            seen,
            answers,
            stream,
        }
    }

    /// Answers this request from now on. A later answer to the same request wins.
    fn answers(&self, method: &'static str, path: &str, status: u16, body: &Value) {
        self.answers.lock().unwrap().insert(
            0,
            Answer {
                method,
                path: path.to_string(),
                status: Some(status),
                body: body.to_string(),
            },
        );
    }

    /// Takes this request from now on and never answers it.
    fn never_answers(&self, method: &'static str, path: &str) {
        self.answers.lock().unwrap().insert(
            0,
            Answer {
                method,
                path: path.to_string(),
                status: None,
                body: String::new(),
            },
        );
    }

    /// Serves the doc page `notes`, with the ID `p` and these items.
    fn has_page(&self, items: &[Value]) {
        let doc = json!({ "page": page_info(), "items": items, "created": false });
        self.answers("POST", "/api/pages/doc", 200, &doc);
        let page = json!({ "page": page_info(), "items": items });
        self.answers("GET", "/api/pages/p", 200, &page);
    }

    /// Sends one message on the event stream.
    fn tells(&self, data: &Value) {
        self.stream.send(Stream::Data(data.to_string())).unwrap();
    }

    fn cuts_the_event_stream(&self) {
        self.stream.send(Stream::Cut).unwrap();
    }

    fn seen(&self, method: &str, path: &str) -> Vec<Seen> {
        let seen = self.seen.lock().unwrap();
        seen.iter()
            .filter(|request| request.method == method && request.path == path)
            .cloned()
            .collect()
    }

    /// Waits until the server has got this request `count` times, and gives the last of them.
    fn waits_for(&self, method: &str, path: &str, count: usize) -> Seen {
        let deadline = Instant::now() + PATIENCE;
        loop {
            let mut seen = self.seen(method, path);
            if seen.len() >= count {
                return seen.remove(count - 1);
            }
            assert!(
                Instant::now() < deadline,
                "the server did not get {method} {path} {count} time(s)"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn serve(
    connection: TcpStream,
    seen: &Mutex<Vec<Seen>>,
    answers: &Mutex<Vec<Answer>>,
    from_test: &Mutex<Receiver<Stream>>,
) {
    let mut reader = BufReader::new(connection);
    let Some(request) = read_request(&mut reader) else {
        return;
    };
    seen.lock().unwrap().push(request.clone());
    let mut connection = reader.into_inner();
    let answer = answers
        .lock()
        .unwrap()
        .iter()
        .find(|answer| answer.method == request.method && answer.path == request.path)
        .map(|answer| (answer.status, answer.body.clone()));
    if request.path == "/api/events" && answer.is_none() {
        let head =
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
        let _ = connection.write_all(head.as_bytes());
        let _ = connection.flush();
        // The next stream waits here until this one is cut
        let from_test = from_test.lock().unwrap();
        while let Ok(Stream::Data(data)) = from_test.recv() {
            let _ = connection.write_all(format!("data: {data}\n\n").as_bytes());
            let _ = connection.flush();
        }
        return;
    }
    let (status, body) = match answer {
        Some((Some(status), body)) => (status, body),
        Some((None, _)) => {
            // Longer than any test waits, with the connection open
            thread::sleep(PATIENCE * 2);
            return;
        }
        None => (404, json!({ "error": "not in this test" }).to_string()),
    };
    let reply = format!(
        "HTTP/1.1 {status} Answer\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = connection.write_all(reply.as_bytes());
}

fn read_request(reader: &mut BufReader<TcpStream>) -> Option<Seen> {
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let mut words = line.split_whitespace();
    let (method, path) = (words.next()?.to_string(), words.next()?.to_string());
    let mut headers = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let Some((name, value)) = line.trim_end().split_once(':') else {
            break;
        };
        headers.push((name.to_ascii_lowercase(), value.trim().to_string()));
    }
    let length = headers
        .iter()
        .find(|(name, _)| name == "content-length")
        .map_or(0, |(_, value)| value.parse().unwrap());
    let mut body = vec![0; length];
    reader.read_exact(&mut body).ok()?;
    Some(Seen {
        method,
        path,
        headers,
        body,
    })
}

fn page_info() -> Value {
    json!({
        "id": "p", "parentId": "", "dirPath": "", "name": "notes", "type": "doc",
        "createdDate": 0, "modifiedDate": 0, "properties": {},
    })
}

fn item(id: &str, body: &str) -> Value {
    json!({
        "id": id, "pageId": "p", "parentItemId": null, "dirPath": "", "name": "notes",
        "type": "doc", "createdDate": 0, "modifiedDate": 0, "idx": id, "level": 1,
        "body": body, "ast": "", "collapsed": false, "types": [], "properties": {},
        "transcludeItemId": null,
    })
}

/// Applies the changes of a line of rows in order, as `peta-journal.el` does. A line of
/// rows gets the rows Emacs has after it as `shown`.
fn apply(rows: &mut Vec<Value>, changes: &Value) {
    let at = |change: &Value, key: &str| change[key].as_u64().unwrap() as usize;
    for change in changes.as_array().unwrap() {
        let (kind, change) = change.as_object().unwrap().iter().next().unwrap();
        match kind.as_str() {
            "Insert" => rows.insert(at(change, "at"), change["item"].clone()),
            "Remove" => {
                rows.remove(at(change, "at"));
            }
            "Move" => {
                let row = rows.remove(at(change, "from"));
                rows.insert(at(change, "to"), row);
            }
            "Update" => rows[at(change, "at")] = change["item"].clone(),
            "Reset" => rows.clone_from(change["items"].as_array().unwrap()),
            other => panic!("no change of rows is called {other}"),
        }
    }
}

/// The side of Emacs: it starts the program, writes lines to it and reads its lines.
struct Emacs {
    program: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<Value>,
    /// Every line of the shell so far. A wait looks here first, as the lines come in no fixed order.
    heard: Vec<Value>,
    seq: u64,
}

impl Emacs {
    fn starts_the_shell(arguments: &[&str]) -> Emacs {
        let mut program = Command::new(env!("CARGO_BIN_EXE_outliner-emacs"))
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = program.stdout.take().unwrap();
        let (to_test, lines) = channel();
        thread::spawn(move || {
            let mut rows = Vec::new();
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let mut message: Value =
                    serde_json::from_str(&line).expect("a line of the shell is JSON");
                if message["t"] == "rows" {
                    apply(&mut rows, &message["changes"]);
                    message["shown"] = json!(rows);
                }
                let _ = to_test.send(message);
            }
        });
        Emacs {
            stdin: program.stdin.take(),
            program,
            lines,
            heard: Vec::new(),
            seq: 0,
        }
    }

    /// Starts the shell against the server and opens the page `notes`.
    fn opens_notes(server: &Server) -> Emacs {
        let mut emacs = Emacs::starts_the_shell(&[&server.address]);
        emacs.sends(&json!({ "type": "start", "url": "/pages/notes" }));
        emacs
    }

    fn sends(&mut self, message: &Value) {
        self.seq += 1;
        let mut line = message.clone();
        line["seq"] = json!(self.seq);
        self.writes(&line.to_string());
    }

    fn writes(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().unwrap();
        writeln!(stdin, "{line}").unwrap();
        stdin.flush().unwrap();
    }

    /// Waits until the shell has sent a line that `wanted` picks.
    fn waits_for(&mut self, what: &str, wanted: impl Fn(&Value) -> bool) {
        let deadline = Instant::now() + PATIENCE;
        while !self.heard.iter().any(&wanted) {
            let left = deadline.saturating_duration_since(Instant::now());
            let message = self
                .lines
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("the shell did not send {what}"));
            self.heard.push(message);
        }
    }

    /// Waits for the rows with these bodies.
    fn waits_for_rows(&mut self, bodies: &[&str]) {
        self.waits_for(&format!("the rows {bodies:?}"), |message| {
            message["t"] == "rows"
                && message["shown"]
                    .as_array()
                    .is_some_and(|rows| rows.iter().map(|row| &row["body"]).eq(bodies))
        });
    }

    fn waits_for_connection(&mut self, connection: &str) {
        self.waits_for(&format!("the connection {connection}"), |message| {
            message["t"] == "status" && message["connection"] == connection
        });
    }

    /// Closes stdin, as Emacs does when it kills the process, and waits for the end. It gives
    /// the exit code and what the shell wrote to stderr.
    fn goes_away(mut self) -> (Option<i32>, String) {
        let status = self.closes_stdin().expect("the shell did not end");
        let mut errors = String::new();
        let mut stderr = self.program.stderr.take().unwrap();
        stderr.read_to_string(&mut errors).unwrap();
        (status.code(), errors)
    }
}

impl Emacs {
    /// Closes stdin and waits for the end of the shell. None when it does not end.
    fn closes_stdin(&mut self) -> Option<ExitStatus> {
        drop(self.stdin.take());
        let deadline = Instant::now() + PATIENCE;
        while Instant::now() < deadline {
            if let Some(status) = self.program.try_wait().unwrap() {
                return Some(status);
            }
            thread::sleep(Duration::from_millis(10));
        }
        None
    }
}

impl Drop for Emacs {
    fn drop(&mut self) {
        // Why not just kill it: a killed shell does not write its coverage (`cargo cov`)
        if self.closes_stdin().is_none() {
            let _ = self.program.kill();
            let _ = self.program.wait();
        }
    }
}

// サーバーのページは、行としてEmacsに届く。
#[test]
fn a_page_of_the_server_reaches_emacs_as_rows() {
    let server = Server::start();
    server.has_page(&[item("a", "first"), item("b", "second")]);

    let mut emacs = Emacs::opens_notes(&server);

    emacs.waits_for_rows(&["first", "second"]);
    let asked = server.waits_for("POST", "/api/pages/doc", 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&asked.body).unwrap(),
        json!({ "title": "notes", "dirPath": "" })
    );
}

// 変更の要求には、どれも、このクライアントの同じIDが付く（ADR-015）。
#[test]
fn every_change_carries_the_one_id_of_this_client() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    let mut emacs = Emacs::opens_notes(&server);
    emacs.waits_for_rows(&["first"]);

    emacs.sends(&json!({ "type": "task_cycle", "id": "a" }));

    let opened = server.waits_for("POST", "/api/pages/doc", 1);
    let cycled = server.waits_for("POST", "/api/items/a/cycle-task", 1);
    let id = opened
        .header("X-Client-Id")
        .expect("the request has the ID");
    assert!(!id.is_empty());
    assert_eq!(cycled.header("X-Client-Id"), Some(id));
}

// サーバーがエラーの状態で返したメッセージは、Emacsに届く。
#[test]
fn the_message_of_the_server_about_an_error_reaches_emacs() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    let refused = json!({ "error": "The disk is full." });
    server.answers("PUT", "/api/pages/p", 500, &refused);
    let mut emacs = Emacs::opens_notes(&server);
    emacs.waits_for_rows(&["first"]);

    emacs.sends(&json!({ "type": "rename_page", "path": "other" }));

    // Only the body of the answer has this text: its status alone would not bring it here
    emacs.waits_for("the message of the server", |message| {
        message["t"] == "message" && message["text"] == "The disk is full."
    });
}

// 画像のファイルは、フォームとしてサーバーに送られる。
#[test]
fn an_image_file_goes_to_the_server_as_a_form() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    let saved = json!({ "fileName": "saved.png", "originalName": "pic.png" });
    server.answers("POST", "/api/assets/upload", 200, &saved);
    let folder = std::env::temp_dir().join(format!("outliner-emacs-{}", std::process::id()));
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("pic.png");
    std::fs::write(&file, b"\x89PNG bytes").unwrap();
    let mut emacs = Emacs::opens_notes(&server);
    emacs.waits_for_rows(&["first"]);

    emacs.sends(&json!({
        "type": "insert_image", "id": "a", "path": file.to_str().unwrap(), "cursor": 0,
    }));

    let upload = server.waits_for("POST", "/api/assets/upload", 1);
    let _ = std::fs::remove_dir_all(&folder);
    let form = upload.text();
    assert!(upload
        .header("Content-Type")
        .is_some_and(|kind| kind.starts_with("multipart/form-data; boundary=")));
    assert!(
        form.contains(r#"name="file"; filename="pic.png""#),
        "{form}"
    );
    assert!(form.contains("PNG bytes"), "{form}");
    assert!(upload.header("X-Client-Id").is_some());
}

// 別のクライアントの変更の知らせで、ページが読み直される。
#[test]
fn a_change_by_another_client_reloads_the_page() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    let mut emacs = Emacs::opens_notes(&server);
    emacs.waits_for_rows(&["first"]);
    emacs.waits_for_connection("connected");

    server.has_page(&[item("a", "changed elsewhere")]);
    server.tells(&json!({
        "type": "pagesUpdated", "pageId": "p", "source": "app", "clientId": "another",
    }));

    emacs.waits_for_rows(&["changed elsewhere"]);
}

// イベントのストリームが切れると、シェルはつなぎ直す。
#[test]
fn the_shell_connects_again_when_the_event_stream_ends() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    let mut emacs = Emacs::opens_notes(&server);
    emacs.waits_for_connection("connected");

    server.cuts_the_event_stream();

    server.waits_for("GET", "/api/events", 2);
}

// 入力した文は、シェルが終わる前に保存される。
#[test]
fn typed_text_is_saved_before_the_shell_ends() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    server.answers(
        "PUT",
        "/api/items/a",
        200,
        &json!({ "item": item("a", "first typed") }),
    );
    let mut emacs = Emacs::opens_notes(&server);
    emacs.waits_for_rows(&["first"]);
    emacs.sends(&json!({ "type": "edit_requested", "id": "a", "cursor": 5 }));
    emacs.sends(&json!({ "type": "body_changed", "id": "a", "body": "first typed" }));

    let (code, _) = emacs.goes_away();

    assert_eq!(code, Some(0));
    let saved = server.seen("PUT", "/api/items/a");
    assert_eq!(saved.len(), 1, "{saved:?}");
    assert_eq!(
        serde_json::from_slice::<Value>(&saved[0].body).unwrap()["body"],
        "first typed"
    );
}

// メッセージでない行は、エラーとして書き出され、読み飛ばされる。
#[test]
fn a_line_that_is_not_a_message_is_reported_and_passed_over() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    let mut emacs = Emacs::starts_the_shell(&[&server.address]);

    emacs.writes("not a message");
    emacs.sends(&json!({ "type": "start", "url": "/pages/notes" }));

    emacs.waits_for_rows(&["first"]);
    let (_, errors) = emacs.goes_away();
    assert!(errors.contains("bad line from Emacs"), "{errors}");
    assert!(errors.contains("not a message"), "{errors}");
}

// 保存するものがなければ、Emacsがいなくなると、シェルはすぐに終わる。
#[test]
fn the_shell_ends_at_once_when_emacs_goes_away_with_nothing_to_save() {
    // Without an address: the shell contacts no server before Emacs tells it to start
    let emacs = Emacs::starts_the_shell(&[]);

    let (code, errors) = emacs.goes_away();

    assert_eq!(code, Some(0));
    assert_eq!(errors, "");
}

// サーバーがいないと、つながっていないことがEmacsに伝わる。
#[test]
fn emacs_is_told_when_there_is_no_server() {
    // The port of a listener that is gone: nothing answers there
    let nowhere = {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", listener.local_addr().unwrap())
    };
    let mut emacs = Emacs::starts_the_shell(&[&nowhere]);

    emacs.sends(&json!({ "type": "start", "url": "/pages/notes" }));

    emacs.waits_for_connection("error");
}

// イベントのストリームを持たないサーバーには、つながったことにならない。
#[test]
fn a_server_without_an_event_stream_is_not_connected() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    let missing = json!({ "error": "Not found" });
    server.answers("GET", "/api/events", 404, &missing);

    let mut emacs = Emacs::opens_notes(&server);

    emacs.waits_for_connection("error");
    emacs.waits_for_rows(&["first"]);
    let connected = |message: &Value| message["connection"] == "connected";
    assert!(!emacs.heard.iter().any(connected), "{:?}", emacs.heard);
}

// サーバーが保存に答えなくても、シェルは5秒で終わる。
#[test]
fn the_shell_ends_after_a_limit_when_the_server_does_not_answer_the_save() {
    let server = Server::start();
    server.has_page(&[item("a", "first")]);
    server.never_answers("PUT", "/api/items/a");
    let mut emacs = Emacs::opens_notes(&server);
    emacs.waits_for_rows(&["first"]);
    emacs.sends(&json!({ "type": "edit_requested", "id": "a", "cursor": 5 }));
    emacs.sends(&json!({ "type": "body_changed", "id": "a", "body": "first typed" }));

    let closed = Instant::now();
    let (code, _) = emacs.goes_away();

    assert_eq!(code, Some(0));
    assert_eq!(server.seen("PUT", "/api/items/a").len(), 1);
    // The limit is 5 seconds: the shell waited for the answer, and not for ever
    assert!(
        closed.elapsed() >= Duration::from_secs(4),
        "{:?}",
        closed.elapsed()
    );
}
