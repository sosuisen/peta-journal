use crate::{Clock, Host, NativeShell, ShellEffectFfi};
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent, UiOp, UiOutput,
};
use client_core::model::{Page, PageWithItems};
use client_core::rest::ClientId;
use client_core::testing::plain_item;
use client_core::{Event, Route};
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// The waits of the fake clock: one only ends when the test says so.
#[derive(Default)]
struct Waits {
    /// How many waits have begun. A wait is known by its place in this order.
    begun: AtomicUsize,
    /// How many waits, from the first on, may end.
    ended: AtomicUsize,
    /// How many waits have ended.
    over: AtomicUsize,
}

/// A journal server with one page, `notes`, and an event stream the test writes.
struct FakeHost {
    calls: Arc<Mutex<Vec<ApiOp>>>,
    /// The titles of the pages the server has answered for, in the order of the answers.
    answered: Arc<Mutex<Vec<String>>>,
    /// Taken by the first connection. A later one finds no stream and waits for ever.
    stream: Mutex<Option<Receiver<SseEvent>>>,
}

impl Host for FakeHost {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        self.calls.lock().unwrap().push(op.clone());
        if let ApiOp::GetOrCreateDoc { title, .. } = &op {
            // A page the server is slow with: a request made after it could overtake it
            if title == "slow" {
                thread::sleep(Duration::from_millis(50));
            }
            self.answered.lock().unwrap().push(title.clone());
        }
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

/// A clock that stands at one time, and whose waits the test ends.
struct FakeClock {
    waits: Arc<Waits>,
}

impl Clock for FakeClock {
    fn now(&self) -> (LocalDateTimeDto, u64) {
        let local = LocalDateTimeDto {
            year: 2026,
            month: 11,
            day: 6,
            hour: 9,
            minute: 30,
        };
        (local, 1_000)
    }

    fn sleep(&self, _ms: u32) {
        let place = self.waits.begun.fetch_add(1, Ordering::SeqCst);
        while self.waits.ended.load(Ordering::SeqCst) <= place {
            thread::sleep(Duration::from_millis(1));
        }
        self.waits.over.fetch_add(1, Ordering::SeqCst);
    }
}

/// The shell on the other side of the border: it keeps what it was sent.
struct Screen {
    shell: NativeShell,
    requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>>,
    calls: Arc<Mutex<Vec<ApiOp>>>,
    answered: Arc<Mutex<Vec<String>>>,
    stream: Sender<SseEvent>,
    waits: Arc<Waits>,
}

impl Screen {
    fn new() -> Screen {
        let requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>> = Arc::default();
        let calls: Arc<Mutex<Vec<ApiOp>>> = Arc::default();
        let answered: Arc<Mutex<Vec<String>>> = Arc::default();
        let (stream, from_test) = channel();
        let waits: Arc<Waits> = Arc::default();
        let clock = FakeClock {
            waits: waits.clone(),
        };
        let host = FakeHost {
            calls: calls.clone(),
            answered: answered.clone(),
            stream: Mutex::new(Some(from_test)),
        };
        let kept = requests.clone();
        let shell = NativeShell::new(host, clock, ClientId::new(), move |bytes| {
            let sent: Vec<Request<ShellEffectFfi>> = BincodeFfiFormat::deserialize(&bytes).unwrap();
            kept.lock().unwrap().extend(sent);
        });
        Screen {
            shell,
            requests,
            calls,
            answered,
            stream,
            waits,
        }
    }

    /// Types into the search box, and waits until the timer for it runs.
    fn search(&self, query: &str) {
        let timers = self.waits.begun.load(Ordering::SeqCst);
        self.send(&Event::SearchQueryChanged(query.into()));
        wait_until("the timer of the search runs", || {
            self.waits.begun.load(Ordering::SeqCst) == timers + 1
        });
    }

    /// Lets the time of the first `timers` timers pass.
    fn let_time_pass(&self, timers: usize) {
        self.waits.ended.store(timers, Ordering::SeqCst);
    }

    /// The queries the server was asked to search for, oldest first.
    fn searches(&self) -> Vec<String> {
        let calls = self.calls.lock().unwrap();
        calls
            .iter()
            .filter_map(|op| match op {
                ApiOp::SearchCombined { query, .. } => Some(query.clone()),
                _ => None,
            })
            .collect()
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

    /// Answers the request for a screen operation the way a shell does: by its number.
    fn answer(&self, op: &UiOp, output: &UiOutput) {
        wait_until("the shell is asked for this", || self.ui_ops().contains(op));
        let id = {
            let requests = self.requests.lock().unwrap();
            let asked = requests
                .iter()
                .find(|request| matches!(&request.effect, ShellEffectFfi::Ui(o) if o == op));
            asked.expect("the shell was asked for this").id.0
        };
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, output).unwrap();
        self.shell.resolve(id, &bytes).unwrap();
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

    /// The URLs the shell was asked to go to, once there is one. A request does not always come
    /// before the call that caused it returns: when another thread is at work in the core, such
    /// as the one with an answer of the server, it is that thread that hands the request over.
    fn navigations_once_any(&self) -> Vec<String> {
        wait_until("the shell is asked to go somewhere", || {
            !self.navigations().is_empty()
        });
        self.navigations()
    }

    /// The URLs the shell was asked to go to, oldest first.
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

// シェルは、依頼の番号とバイト列で答える。確認に「はい」と答えると、その答えがサーバーに送られる。
#[test]
fn the_answer_of_the_shell_reaches_the_core() {
    let screen = Screen::new();
    let confirm = UiOp::Confirm("Insert?".into());
    screen.server_says(SseEvent::Message(
        r#"{"type":"confirm","requestId":"confirm-7","message":"Insert?"}"#.into(),
    ));
    wait_until("the shell is asked to confirm", || {
        screen.ui_ops().contains(&confirm)
    });

    screen.answer(&confirm, &UiOutput::Bool(true));

    wait_until("the answer is sent to the server", || {
        let calls = screen.calls.lock().unwrap();
        calls.contains(&ApiOp::AnswerConfirm {
            request_id: "confirm-7".into(),
            ok: true,
        })
    });
}

// コアの2つ目の質問は、1つ目が答えられるまでシェルに届かない。1つ目に答えると、2つ目が届く。
#[test]
fn a_second_question_reaches_the_shell_after_the_first_is_answered() {
    let screen = Screen::new();
    let first = UiOp::Confirm("Insert?".into());
    let second = UiOp::Confirm("Replace?".into());
    screen.server_says(SseEvent::Message(
        r#"{"type":"confirm","requestId":"confirm-1","message":"Insert?"}"#.into(),
    ));
    screen.server_says(SseEvent::Message(
        r#"{"type":"confirm","requestId":"confirm-2","message":"Replace?"}"#.into(),
    ));
    wait_until("the core has heard both questions", || {
        screen.calls_named("AnswerConfirm") == 0 && screen.ui_ops().contains(&first)
    });
    // The stream is read on its own thread: give it the time to bring the second one
    thread::sleep(Duration::from_millis(200));
    assert_eq!(screen.ui_ops(), std::slice::from_ref(&first));

    screen.answer(&first, &UiOutput::Bool(false));

    wait_until("the second question reaches the shell", || {
        screen.ui_ops().contains(&second)
    });
    assert_eq!(screen.ui_ops(), [first, second]);
}

// シェルの答えは、待っている質問の数にかかわらず、その質問のものとしてコアに届く。
#[test]
fn every_answer_reaches_the_core_for_its_own_question() {
    let screen = Screen::new();
    let first = UiOp::Confirm("Insert?".into());
    let second = UiOp::Confirm("Replace?".into());
    screen.server_says(SseEvent::Message(
        r#"{"type":"confirm","requestId":"confirm-1","message":"Insert?"}"#.into(),
    ));
    screen.server_says(SseEvent::Message(
        r#"{"type":"confirm","requestId":"confirm-2","message":"Replace?"}"#.into(),
    ));

    screen.answer(&first, &UiOutput::Bool(false));
    screen.answer(&second, &UiOutput::Bool(true));

    wait_until("both answers are sent to the server", || {
        screen.calls_named("AnswerConfirm") == 2
    });
    let calls = screen.calls.lock().unwrap();
    assert!(calls.contains(&ApiOp::AnswerConfirm {
        request_id: "confirm-1".into(),
        ok: false,
    }));
    assert!(calls.contains(&ApiOp::AnswerConfirm {
        request_id: "confirm-2".into(),
        ok: true,
    }));
}

// URLを変える依頼は、バイト列でシェルに届く。
#[test]
fn a_navigation_reaches_the_shell_as_bytes() {
    let screen = Screen::new();

    screen.send(&Event::NavigateHome);

    assert_eq!(screen.navigations_once_any(), ["/today"]);
}

// 今の時刻は、シェルに尋ねずにRust側で答える。名前のない新しいページは、その時刻から名前を得る。
#[test]
fn the_time_is_answered_without_the_shell() {
    let screen = Screen::new();
    let prompt = UiOp::Prompt("Page title:".into());
    screen.send(&Event::CreateNewPage {
        search_query: String::new(),
    });

    screen.answer(&prompt, &UiOutput::Text(Some(String::new())));

    assert_eq!(
        screen.navigations_once_any(),
        ["/pages/Untitled-202611060930"]
    );
    assert!(!screen.ui_ops().contains(&UiOp::LocalNow));
}

// タイマーは、Rust側で時間が過ぎると鳴る。検索は、入力のあとの待ちが明けてから、サーバーに送られる。
#[test]
fn a_timer_fires_when_its_time_has_passed() {
    let screen = Screen::new();

    screen.search("apple");
    assert!(screen.searches().is_empty());
    screen.let_time_pass(1);

    wait_until("the server is asked to search", || {
        screen.searches() == ["apple"]
    });
}

// 取り消されたタイマーは鳴らない。検索の入力を続けると、前の入力の検索はサーバーに送られない。
#[test]
fn a_cancelled_timer_does_not_fire() {
    let screen = Screen::new();
    screen.search("a");
    screen.search("ab");

    // The time of the cancelled timer passes first, so its search would be the first one
    screen.let_time_pass(1);
    wait_until("the time of the first timer is over", || {
        screen.waits.over.load(Ordering::SeqCst) == 1
    });
    screen.let_time_pass(2);

    wait_until("the server is asked to search", || {
        !screen.searches().is_empty()
    });
    assert_eq!(screen.searches(), ["ab"]);
}

// 読めないバイト列は、イベントとしても答えとしても、エラーで返る。コアは止まらない。
#[test]
fn bytes_that_cannot_be_read_are_refused() {
    let screen = Screen::new();

    assert!(screen.shell.update(&[0xff; 3]).is_err());
    assert!(screen.shell.resolve(9_999, &[]).is_err());

    screen.send(&Event::NavigateHome);
    assert_eq!(screen.navigations_once_any(), ["/today"]);
}

// シェルは、URLでページを開ける。今日の日付は、Rust側の時計から入る。
#[test]
fn the_shell_opens_a_page_by_its_url() {
    let screen = Screen::new();

    screen.shell.open("/pages/notes#a").unwrap();

    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    assert!(screen.view().contains("2026-11-06"));
}

// シェルがURLで開いたページは、コアの履歴に入る。戻ると、前のページがもう一度開く（ADR 035）。
#[test]
fn a_page_the_shell_opens_goes_into_the_history_of_the_core() {
    let screen = Screen::new();
    screen.shell.open("/pages/first").unwrap();
    screen.shell.open("/pages/second").unwrap();

    screen.send(&Event::Back {
        today: "2026-11-06".into(),
    });

    wait_until("the page before is opened again", || {
        screen.answered.lock().unwrap().len() == 3
    });
    assert_eq!(
        *screen.answered.lock().unwrap(),
        ["first", "second", "first"]
    );
}

// サーバーへの要求は、コアが出した順に、1つずつ届く。前の要求が遅くても、後の要求が追い越さない。
// 追い越すと、保存より先に結合が届いて、打った文字が消える。
#[test]
fn the_requests_reach_the_server_one_at_a_time_in_the_order_the_core_made_them() {
    let screen = Screen::new();

    screen.shell.open("/pages/slow").unwrap();
    screen.shell.open("/pages/notes").unwrap();

    wait_until("the server has answered for both pages", || {
        screen.answered.lock().unwrap().len() == 2
    });
    assert_eq!(*screen.answered.lock().unwrap(), ["slow", "notes"]);
}

// 何も指さないURLは、開けない。
#[test]
fn a_url_that_names_nothing_is_refused() {
    let screen = Screen::new();

    assert!(screen.shell.open("").is_err());
}
