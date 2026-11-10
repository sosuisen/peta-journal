//! The entrance for a shell in another language, called here from Rust. It only exists with the
//! feature `ffi`: `cargo test -p peta-shell-native --features ffi`.
#![cfg(feature = "ffi")]

use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use peta_core::effects::UiOp;
use peta_core::Event;
use peta_shell_native::ffi::{PetaCore, PetaShell};
use peta_shell_native::ShellEffectFfi;
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
