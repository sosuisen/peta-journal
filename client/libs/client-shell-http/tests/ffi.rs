//! The entrance for a shell in another language, called here from Rust. It only exists with the
//! feature `ffi`: `cargo test -p client-shell-http --features ffi`.
#![cfg(feature = "ffi")]

use client_core::effects::UiOp;
use client_core::typing::Typing;
use client_core::{EditPlace, Event};
use client_native_bridge::ShellEffectFfi;
use client_shell_http::ffi::{copied_html, PetaCore, PetaShell};
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use std::sync::{Arc, Mutex};

mod common;
use common::{start_server, wait_until};

/// A shell that keeps what the core asks of it.
#[derive(Default)]
struct Kept {
    requests: Mutex<Vec<Request<ShellEffectFfi>>>,
}

impl PetaShell for Kept {
    fn process_effects(&self, bytes: Vec<u8>) {
        let sent: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(&bytes).unwrap();
        self.requests.lock().unwrap().extend(sent);
    }
}

// 別の言語のシェルの入口からコアを起動すると、シェルへの依頼が届く。
#[test]
fn a_shell_in_another_language_gets_the_requests_of_the_core() {
    let (address, _seen) = start_server();
    let shell = Arc::new(Kept::default());

    let _core = PetaCore::new(address, shell.clone());

    wait_until("the shell is asked to confirm", || {
        let requests = shell.requests.lock().unwrap();
        requests.iter().any(|request| {
            matches!(&request.effect, ShellEffectFfi::Ui(UiOp::Confirm(message)) if message == "Insert?")
        })
    });
}

// 入口から送ったイベントは、コアに届く。検索の箱への入力が、ビューモデルに入る。
#[test]
fn an_event_from_the_entrance_shows_in_the_view_model() {
    let (address, _seen) = start_server();
    let core = PetaCore::new(address, Arc::new(Kept::default()));
    let mut event = Vec::new();
    BincodeFfiFormat::serialize(&mut event, &Event::SearchQueryChanged("apple".into())).unwrap();

    core.update(&event).unwrap();

    let view = core.view();
    assert!(String::from_utf8_lossy(&view).contains("apple"));
}

// 入口から、画面の差分をbytesで読める。開いたページの行が、最初の差分に入る（ADR 076）。
#[test]
fn the_changes_of_the_screen_are_read_from_the_entrance_as_bytes() {
    let (address, _seen) = start_server();
    let core = PetaCore::new(address, Arc::new(Kept::default()));
    core.open("/pages/notes".into()).unwrap();
    wait_until("the page shows", || {
        String::from_utf8_lossy(&core.view()).contains("the body of a")
    });

    let changes = core.changes();

    assert!(String::from_utf8_lossy(&changes).contains("the body of a"));
}

// 入口からコアを閉じると、保存を待っていた本文が、閉じる前にサーバーに保存される（#766）。
#[test]
fn closing_from_the_entrance_saves_the_body_waiting_to_be_saved() {
    let (address, seen) = start_server();
    let core = PetaCore::new(address, Arc::new(Kept::default()));
    core.open("/pages/notes".into()).unwrap();
    wait_until("the page shows", || {
        String::from_utf8_lossy(&core.view()).contains("the body of a")
    });
    let mut edit = Vec::new();
    let editing = Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    };
    BincodeFfiFormat::serialize(&mut edit, &editing).unwrap();
    core.update(&edit).unwrap();
    let typing = Typing::Typed {
        item_id: "a".into(),
        body: "typed into a".into(),
        cursor: 12,
    };
    let mut typed = Vec::new();
    BincodeFfiFormat::serialize(&mut typed, &typing).unwrap();
    core.typed(&typed).unwrap();

    core.close();

    let seen = seen.lock().unwrap();
    assert!(seen
        .iter()
        .any(|line| line.starts_with("PUT /api/items/a ")));
}

// 読めないバイト列は、イベントとしても答えとしても、エラーの文として返る。
#[test]
fn bytes_that_cannot_be_read_come_back_as_an_error() {
    let (address, _seen) = start_server();
    let core = PetaCore::new(address, Arc::new(Kept::default()));

    assert!(core.update(&[0xff; 3]).is_err());
    assert!(core.resolve(9_999, &[]).is_err());
}

// 入口から、URLでページを開ける。サーバーに、そのページを尋ねる。何も指さないURLは、エラーの文になる。
#[test]
fn a_page_is_opened_by_its_url_from_the_entrance() {
    let (address, seen) = start_server();
    let core = PetaCore::new(address, Arc::new(Kept::default()));

    core.open("/pages/notes".into()).unwrap();

    wait_until("the server is asked for the page", || {
        let seen = seen.lock().unwrap();
        seen.iter()
            .any(|line| line.starts_with("POST /api/pages/doc"))
    });
    assert!(core.open(String::new()).is_err());
}

// 入口から、画像をファイルのパスで貼れる。ファイルは、サーバーのアップロードに届く。
#[test]
fn an_image_is_pasted_by_its_path_from_the_entrance() {
    let (address, seen) = start_server();
    let core = PetaCore::new(address, Arc::new(Kept::default()));
    let folder = tempfile::tempdir().unwrap();
    let image = folder.path().join("sample.png");
    std::fs::write(&image, b"drawing").unwrap();

    core.paste_image("a".into(), image.to_string_lossy().into_owned(), 0);

    wait_until("the server is sent the image", || {
        let seen = seen.lock().unwrap();
        seen.iter()
            .any(|line| line.starts_with("POST /api/assets/upload "))
    });
}

// 別の言語のシェルの入口から、コピーのHTMLを作れる。ウィキリンクは、ブラウザ版のページへのリンクになる。
#[test]
fn a_shell_in_another_language_gets_the_html_of_a_copy() {
    let ast = r#"{"type":"document","children":[{"type":"paragraph","children":[{"type":"wikilink","url":"Idea","children":[{"type":"text","value":"Idea"}]}]}]}"#;

    assert_eq!(
        copied_html(ast.into(), "http://localhost:8087".into(), "tags/".into()),
        "<p><a href=\"http://localhost:8087/pages/Idea\">Idea</a></p>\n"
    );
}
