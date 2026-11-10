use crate::{Host, NativeShell, ShellEffectFfi};
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, SseEvent, UiOp};
use peta_core::model::{Page, PageWithItems};
use peta_core::rest::ClientId;
use peta_core::testing::plain_item;
use peta_core::{Event, Route};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// A journal server with one page, `notes`, and an event stream the test writes.
struct FakeHost {
    calls: Arc<Mutex<Vec<ApiOp>>>,
    /// Taken by the first connection. A later one finds no stream and waits for ever.
    stream: Mutex<Option<Receiver<SseEvent>>>,
}

impl Host for FakeHost {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        self.calls.lock().unwrap().push(op.clone());
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

    fn events(&self, send: &mut dyn FnMut(SseEvent)) {
        let stream = self.stream.lock().unwrap().take();
        for event in stream.into_iter().flatten() {
            send(event);
        }
    }
}

/// The shell on the other side of the border: it keeps what it was sent.
struct Screen {
    shell: NativeShell,
    requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>>,
    calls: Arc<Mutex<Vec<ApiOp>>>,
    stream: Sender<SseEvent>,
}

impl Screen {
    fn new() -> Screen {
        let requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>> = Arc::default();
        let calls: Arc<Mutex<Vec<ApiOp>>> = Arc::default();
        let (stream, from_test) = channel();
        let host = FakeHost {
            calls: calls.clone(),
            stream: Mutex::new(Some(from_test)),
        };
        let kept = requests.clone();
        let shell = NativeShell::new(host, ClientId::new(), move |bytes| {
            let sent: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(&bytes).unwrap();
            kept.lock().unwrap().extend(sent);
        });
        Screen {
            shell,
            requests,
            calls,
            stream,
        }
    }

    /// How often the server was asked for this operation, by the name of its variant.
    fn calls_named(&self, name: &str) -> usize {
        let calls = self.calls.lock().unwrap();
        calls
            .iter()
            .filter(|op| format!("{op:?}").starts_with(name))
            .count()
    }

    fn server_says(&self, event: SseEvent) {
        self.stream.send(event).unwrap();
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

    /// The screen operations the shell was asked for, oldest first.
    fn ui_ops(&self) -> Vec<UiOp> {
        let requests = self.requests.lock().unwrap();
        requests
            .iter()
            .filter_map(|request| match &request.effect {
                ShellEffectFfi::Ui(op) => Some(op.clone()),
                _ => None,
            })
            .collect()
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
    let screen = Screen::new();

    screen.open_notes();

    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    assert!(screen.renders() > 0);
}

// 画面の操作は、バイト列でシェルに届く。ページを開くと、タイトルを変える依頼が届く。
#[test]
fn an_operation_of_the_screen_reaches_the_shell_as_bytes() {
    let screen = Screen::new();

    screen.open_notes();

    wait_until("the shell is asked to set the title", || {
        screen
            .ui_ops()
            .iter()
            .any(|op| matches!(op, UiOp::SetTitle(title) if title.contains("notes")))
    });
}

// サーバーのストリームの出来事は、1つの依頼に何度でも届く。開くと最近のページが読み直され、そのあとの確認の依頼はシェルに届く。
#[test]
fn every_event_of_the_server_stream_reaches_the_core() {
    let screen = Screen::new();
    wait_until("the start has read the recent pages", || {
        screen.calls_named("GetRecentPages") == 1
    });

    screen.server_says(SseEvent::Open);
    wait_until("the open stream reads the recent pages again", || {
        screen.calls_named("GetRecentPages") == 2
    });
    screen.server_says(SseEvent::Message(
        r#"{"type":"confirm","requestId":"confirm-7","message":"Insert?"}"#.into(),
    ));

    wait_until("the shell is asked to confirm", || {
        screen.ui_ops().contains(&UiOp::Confirm("Insert?".into()))
    });
}
