//! Runs the shell against a server over HTTP, with the clock of this machine. The server here is
//! a small one of the test. This is the only test of `src/server.rs`.

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use peta_core::effects::{UiOp, UiOutput};
use peta_core::Event;
use peta_shell_native::{connect, NativeShell, ShellEffectFfi};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// The message the server sends on its event stream.
const CONFIRM: &str = r#"{"type":"confirm","requestId":"confirm-7","message":"Insert?"}"#;

/// A journal server that knows nothing: it answers 404, and sends one message on its event
/// stream. It keeps the first line of each request, such as `GET /api/config HTTP/1.1`.
fn start_server() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let kept = seen.clone();
    thread::spawn(move || {
        for connection in listener.incoming().flatten() {
            let kept = kept.clone();
            // One thread each: the event stream stays open while the other requests come
            thread::spawn(move || serve(connection, &kept));
        }
    });
    (address, seen)
}

fn serve(connection: TcpStream, seen: &Mutex<Vec<String>>) {
    let mut reader = BufReader::new(connection);
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    // No request of these tests has a body, so the head is all there is to read
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap() > 2 {
        line.clear();
    }
    seen.lock().unwrap().push(first.trim().to_string());
    let mut connection = reader.into_inner();
    if first.starts_with("GET /api/events ") {
        let head = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\n\r\n";
        let _ = write!(connection, "{head}data: {CONFIRM}\n\n");
        let _ = connection.flush();
        // The stream stays open for as long as the test runs
        thread::sleep(Duration::from_secs(600));
    } else {
        let _ = write!(
            connection,
            "HTTP/1.1 404 Not Found\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
        );
    }
}

/// The shell on the other side of the border, with what it was sent.
struct Screen {
    shell: NativeShell,
    requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Screen {
    fn new() -> Screen {
        let (address, seen) = start_server();
        let requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>> = Arc::default();
        let kept = requests.clone();
        let shell = connect(address, move |bytes| {
            let sent: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(&bytes).unwrap();
            kept.lock().unwrap().extend(sent);
        });
        Screen {
            shell,
            requests,
            seen,
        }
    }

    fn send(&self, event: &Event) {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, event).unwrap();
        self.shell.update(&bytes).unwrap();
    }

    /// The number of the request for this screen operation, once the shell has it.
    fn asked(&self, op: &UiOp) -> Option<u32> {
        let requests = self.requests.lock().unwrap();
        let asked = requests
            .iter()
            .find(|request| matches!(&request.effect, ShellEffectFfi::Ui(o) if o == op));
        asked.map(|request| request.id.0)
    }

    fn navigations(&self) -> Vec<String> {
        let requests = self.requests.lock().unwrap();
        requests
            .iter()
            .filter_map(|request| match &request.effect {
                ShellEffectFfi::Navigate(op) => Some(op.url.clone()),
                _ => None,
            })
            .collect()
    }

    fn saw(&self, start: &str) -> bool {
        let seen = self.seen.lock().unwrap();
        seen.iter().any(|line| line.starts_with(start))
    }
}

fn wait_until(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "never happened: {what}"
        );
        thread::sleep(Duration::from_millis(5));
    }
}

// 起動すると、設定の読み込みがHTTPでサーバーに届く。
#[test]
fn the_start_asks_the_server_over_http() {
    let screen = Screen::new();

    wait_until("the server is asked for the recent pages", || {
        screen.saw("GET /api/pages/recent?")
    });
}

// サーバーのイベントストリームのメッセージが、コアに届く。確認の依頼は、シェルに届く。
#[test]
fn a_message_of_the_event_stream_reaches_the_core() {
    let screen = Screen::new();

    wait_until("the shell is asked to confirm", || {
        screen.asked(&UiOp::Confirm("Insert?".into())).is_some()
    });
}

// 名前のない新しいページは、この機械の時計から名前を得る。
#[test]
fn a_new_page_without_a_name_is_named_after_the_clock() {
    let screen = Screen::new();
    let prompt = UiOp::Prompt("Page title:".into());
    screen.send(&Event::CreateNewPage {
        search_query: String::new(),
    });
    let id = screen.asked(&prompt).unwrap();
    let mut answer = Vec::new();
    BincodeFfiFormat::serialize(&mut answer, &UiOutput::Text(Some(String::new()))).unwrap();

    screen.shell.resolve(id, &answer).unwrap();

    let url = screen.navigations().pop().unwrap();
    let digits = url.strip_prefix("/pages/Untitled-20").unwrap();
    assert_eq!(digits.len(), 10, "{url}");
    assert!(digits.chars().all(|c| c.is_ascii_digit()), "{url}");
}

// 検索は、入力から0.2秒待ってから、サーバーに届く。
#[test]
fn a_search_reaches_the_server_after_its_delay() {
    let screen = Screen::new();
    let typed = Instant::now();

    screen.send(&Event::SearchQueryChanged("apple".into()));

    wait_until("the server is asked to search", || {
        let seen = screen.seen.lock().unwrap();
        seen.iter().any(|line| line.contains("apple"))
    });
    assert!(typed.elapsed() >= Duration::from_millis(200));
}
