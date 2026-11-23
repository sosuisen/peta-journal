//! A small journal server for the tests that run the shell over HTTP.
// Each test program uses its own part of this
#![allow(dead_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// The message the server sends on its event stream.
pub const CONFIRM: &str = r#"{"type":"confirm","requestId":"confirm-7","message":"Insert?"}"#;

/// The one page the server knows, `notes` with the id `p`, and its one item `a`.
pub const PAGE: &str = r#"{"page":{"id":"p","parentId":"","dirPath":"","name":"notes","type":"doc","createdDate":0,"modifiedDate":0,"properties":{}},"items":[{"id":"a","pageId":"p","parentItemId":null,"dirPath":"","name":"notes","type":"doc","createdDate":0,"modifiedDate":0,"idx":"a","level":1,"body":"the body of a","ast":"","collapsed":false,"types":[],"properties":{},"transcludeItemId":null}],"created":false,"transcludedTrees":{}}"#;

/// A journal server that knows one page, `notes` (`PAGE`), and nothing else: it answers 404
/// to the rest, and sends one message on its event stream. It keeps the first line of each request, such as `GET /api/config HTTP/1.1`.
pub fn start_server() -> (String, Arc<Mutex<Vec<String>>>) {
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
    let mut length = 0;
    let mut line = String::new();
    while reader.read_line(&mut line).unwrap() > 2 {
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse().unwrap_or(0);
            }
        }
        line.clear();
    }
    // No test looks at the body, but it is read all the same: Windows resets a connection
    // closed with bytes left unread, and the client then loses the answer (#906)
    std::io::copy(&mut (&mut reader).take(length), &mut std::io::sink()).unwrap();
    // A save is slow, and is seen only once it is done
    if first.starts_with("PUT /api/items/") {
        thread::sleep(Duration::from_millis(100));
    }
    seen.lock().unwrap().push(first.trim().to_string());
    let mut connection = reader.into_inner();
    let is_page =
        first.starts_with("POST /api/pages/doc ") || first.starts_with("GET /api/pages/p ");
    if is_page {
        let _ = write!(
            connection,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{PAGE}",
            PAGE.len()
        );
    } else if first.starts_with("GET /api/events ") {
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

/// Waits for what the threads of the shell do. A test that waits in vain fails here.
pub fn wait_until(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "never happened: {what}"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
