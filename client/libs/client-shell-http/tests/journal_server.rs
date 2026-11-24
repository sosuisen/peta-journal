//! Runs the shell against a server over HTTP, with the clock of this machine. The server here is
//! a small one of the test. This is the only test of `src/server.rs`.

use client_core::effects::{UiOp, UiOutput};
use client_core::keys::Held;
use client_core::rich_text::RichLink;
use client_core::Event;
use client_native_bridge::{NativeShell, ShellEffectFfi};
use client_shell_http::connect;
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

mod common;
use common::{start_server, wait_until};

/// The shell on the other side of the border, with what it was sent.
struct Screen {
    shell: NativeShell,
    /// The address of the server of the test, such as `http://127.0.0.1:50000`.
    address: String,
    requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>>,
    seen: Arc<Mutex<Vec<String>>>,
}

impl Screen {
    fn new() -> Screen {
        let (address, seen) = start_server();
        let requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>> = Arc::default();
        let kept = requests.clone();
        let shell = connect(address.clone(), move |bytes| {
            let sent: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(&bytes).unwrap();
            kept.lock().unwrap().extend(sent);
        });
        Screen {
            shell,
            address,
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
    // The request may come after `send` returns, from another thread that is at work in the core
    wait_until("the shell is asked for a title", || {
        screen.asked(&prompt).is_some()
    });
    let id = screen.asked(&prompt).unwrap();
    let mut answer = Vec::new();
    BincodeFfiFormat::serialize(&mut answer, &UiOutput::Text(Some(String::new()))).unwrap();

    screen.shell.resolve(id, &answer).unwrap();

    wait_until("the shell is asked to go to the new page", || {
        !screen.navigations().is_empty()
    });
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

// 貼った画像のファイルは、HTTPでサーバーのアップロードに届く。
#[test]
fn a_pasted_image_reaches_the_upload_of_the_server() {
    let screen = Screen::new();
    let folder = tempfile::tempdir().unwrap();
    let image = folder.path().join("sample.png");
    std::fs::write(&image, b"drawing").unwrap();

    screen.shell.paste_image("a", &image, 0);

    wait_until("the server is sent the image", || {
        screen.saw("POST /api/assets/upload ")
    });
}

// 押したassetsのファイルは、シェルがつながったjournalサーバーのアドレスで、ブラウザで開く（ADR 083）。
#[test]
fn a_clicked_file_of_the_assets_opens_on_the_server_the_shell_is_connected_to() {
    let screen = Screen::new();

    screen.send(&Event::LinkClicked {
        link: RichLink::File {
            path: "assets/report.pdf".into(),
        },
        held: Held::default(),
    });

    let opened = UiOp::OpenInBrowser {
        address: format!("{}/assets/report.pdf", screen.address),
    };
    wait_until("the file opens in the browser", || {
        screen.asked(&opened).is_some()
    });
}
