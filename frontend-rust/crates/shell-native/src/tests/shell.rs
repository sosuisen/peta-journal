use crate::{Host, NativeShell, ShellEffectFfi};
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult};
use peta_core::model::{Page, PageWithItems};
use peta_core::rest::ClientId;
use peta_core::testing::plain_item;
use peta_core::{Event, Route};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// A journal server with one page, `notes`.
#[derive(Default)]
struct FakeHost;

impl Host for FakeHost {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        match op {
            ApiOp::GetOrCreateDoc { .. } | ApiOp::GetPageById { .. } => {
                Ok(ApiOutput::Page(PageWithItems {
                    page: Page {
                        id: "p".into(),
                        name: "notes".into(),
                        page_type: "doc".into(),
                        ..Default::default()
                    },
                    items: vec![plain_item("a", 1, "the body of a")],
                    created: false,
                    transcluded_trees: None,
                }))
            }
            _ => Err(ApiError("the fake server does not know this".into())),
        }
    }
}

/// The shell on the other side of the border: it keeps what it was sent.
struct Screen {
    shell: NativeShell,
    requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>>,
}

impl Screen {
    fn new(host: FakeHost) -> Screen {
        let requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>> = Arc::default();
        let kept = requests.clone();
        let shell = NativeShell::new(host, ClientId::new(), move |bytes| {
            let sent: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(&bytes).unwrap();
            kept.lock().unwrap().extend(sent);
        });
        Screen { shell, requests }
    }

    fn send(&self, event: &Event) {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, event).unwrap();
        self.shell.update(&bytes).unwrap();
    }

    fn open_notes(&self) {
        self.send(&Event::Route {
            route: Route::Page {
                name: "notes".into(),
                fragment: None,
            },
            today: "2026-02-10".into(),
        });
    }

    /// The view model as the shell gets it. Text in it is readable in the bytes.
    fn view(&self) -> String {
        String::from_utf8_lossy(&self.shell.view()).into_owned()
    }

    fn renders(&self) -> usize {
        let requests = self.requests.lock().unwrap();
        requests
            .iter()
            .filter(|request| matches!(request.effect, ShellEffectFfi::Render(_)))
            .count()
    }
}

/// Waits for what the threads of the shell do. A test that waits in vain fails here.
fn wait_until(what: &str, done: impl Fn() -> bool) {
    let started = Instant::now();
    while !done() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "never happened: {what}"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

// シェルがページを開くと、Rust側がサーバーから読み、描画の依頼とビューモデルがバイト列でシェルに届く。
#[test]
fn a_page_the_shell_opens_is_read_from_the_server_and_reaches_the_shell_as_bytes() {
    let screen = Screen::new(FakeHost);

    screen.open_notes();

    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    assert!(screen.renders() > 0);
}
