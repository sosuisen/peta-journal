use crate::tests::applied::applied;
use crate::BorderApp;
use crate::{Changes, RowChange};
use crate::{Clock, Host, NativeShell, ShellEffectFfi};
use client_core::drag::DragMove;
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent, UiOp, UiOutput,
};
use client_core::keys::{Chord, EditorState, Held, Key, KeyPress};
use client_core::model::{JournalPage, Page, Uploaded};
use client_core::rest::ClientId;
use client_core::rich_text::RichLink;
use client_core::screens::day::CHECK_MS;
use client_core::screens::search::SEARCH_DELAY_MS;
use client_core::testing::{page_answer, plain_item};
use client_core::typing::Typing;
use client_core::{EditPlace, Event, Route};
use crux_core::bridge::{BincodeFfiFormat, FfiFormat, Request};
use crux_core::Core;
use std::path::Path;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
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
    /// How long each wait is, in the order they began. The timers of the core begin on threads
    /// of their own, so a timer that began earlier may come to wait later: its length tells
    /// which one it is.
    lengths: Mutex<Vec<u32>>,
}

impl Waits {
    /// How many waits of this length have begun.
    fn begun_of(&self, ms: u32) -> usize {
        let lengths = self.lengths.lock().unwrap();
        lengths.iter().filter(|&&length| length == ms).count()
    }
}

/// A journal server with one page, `notes`, and an event stream the test writes.
struct FakeHost {
    calls: Arc<Mutex<Vec<ApiOp>>>,
    /// The titles of the pages the server has answered for, in the order of the answers.
    answered: Arc<Mutex<Vec<String>>>,
    /// Taken by the first connection. A later one finds no stream and waits for ever.
    stream: Mutex<Option<Receiver<SseEvent>>>,
    /// What each file the server took held when it took it.
    uploads: Arc<Mutex<Vec<Vec<u8>>>>,
    /// False for a journal that runs no extension, as the one inside the Android app.
    runs_extensions: bool,
}

impl Host for FakeHost {
    fn app_url(&self) -> Option<String> {
        Some("http://localhost:8080".into())
    }

    fn runs_extensions(&self) -> bool {
        self.runs_extensions
    }

    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        // Text the server is slow to save: it is in the calls only once it has been saved
        if format!("{op:?}").contains("slowly") {
            thread::sleep(Duration::from_millis(100));
        }
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
                let page = Page {
                    id: "p".into(),
                    name: "notes".into(),
                    page_type: "doc".into(),
                    ..Default::default()
                };
                Ok(page_answer(
                    &op,
                    page,
                    vec![plain_item("a", 1, "the body of a")],
                ))
            }
            ApiOp::GetOrCreateJournal { file_name } => Ok(ApiOutput::Journal(JournalPage {
                page: Page {
                    id: format!("page-{file_name}"),
                    name: file_name.clone(),
                    page_type: "journal".into(),
                    ..Default::default()
                },
                ..Default::default()
            })),
            _ => Err(ApiError("the fake server does not know this".into())),
        }
    }

    fn upload(&self, path: &Path) -> ApiResult<ApiOutput> {
        self.uploads
            .lock()
            .unwrap()
            .push(std::fs::read(path).unwrap_or_default());
        let original_name = path.file_name().unwrap().to_string_lossy().into_owned();
        // A file the server cannot save, as when the disk is full
        if original_name.contains("broken") {
            return Err(ApiError("the fake server cannot save this".into()));
        }
        Ok(ApiOutput::Uploaded(Uploaded {
            file_name: "saved.png".into(),
            original_name,
        }))
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
    /// The day of the month the clock says. A test moves it on.
    day: Arc<AtomicU32>,
}

impl Clock for FakeClock {
    fn now(&self) -> (LocalDateTimeDto, u64) {
        let local = LocalDateTimeDto {
            year: 2026,
            month: 11,
            day: self.day.load(Ordering::SeqCst),
            hour: 9,
            minute: 30,
        };
        (local, 1_000)
    }

    fn sleep(&self, ms: u32) {
        // The clock stands at one day, so a look at the date would find nothing new. The core
        // waits to look all the time: its wait is not counted with the timers of the test
        if ms == CHECK_MS {
            loop {
                thread::park();
            }
        }
        let place = {
            // The place and the length are kept together, so that a wait is counted as begun
            // only once its length is known
            let mut lengths = self.waits.lengths.lock().unwrap();
            lengths.push(ms);
            self.waits.begun.fetch_add(1, Ordering::SeqCst)
        };
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
    uploads: Arc<Mutex<Vec<Vec<u8>>>>,
    stream: Sender<SseEvent>,
    waits: Arc<Waits>,
    /// The day of the month of the clock.
    day: Arc<AtomicU32>,
}

impl Screen {
    fn new() -> Screen {
        Screen::on(true)
    }

    /// A screen on a journal that runs no extension.
    fn without_extensions() -> Screen {
        Screen::on(false)
    }

    fn on(runs_extensions: bool) -> Screen {
        let requests: Arc<Mutex<Vec<Request<ShellEffectFfi>>>> = Arc::default();
        let calls: Arc<Mutex<Vec<ApiOp>>> = Arc::default();
        let answered: Arc<Mutex<Vec<String>>> = Arc::default();
        let uploads: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
        let (stream, from_test) = channel();
        let waits: Arc<Waits> = Arc::default();
        let day = Arc::new(AtomicU32::new(6));
        let clock = FakeClock {
            waits: waits.clone(),
            day: day.clone(),
        };
        let host = FakeHost {
            calls: calls.clone(),
            answered: answered.clone(),
            stream: Mutex::new(Some(from_test)),
            uploads: uploads.clone(),
            runs_extensions,
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
            uploads,
            stream,
            waits,
            day,
        }
    }

    /// Types into the search box, and waits until the timer for it runs.
    fn search(&self, query: &str) {
        let timers = self.waits.begun_of(SEARCH_DELAY_MS);
        self.send(&Event::SearchQueryChanged(query.into()));
        wait_until("the timer of the search runs", || {
            self.waits.begun_of(SEARCH_DELAY_MS) == timers + 1
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

    /// The days of the journals the server was asked for, oldest first.
    fn journals_asked(&self) -> Vec<String> {
        let calls = self.calls.lock().unwrap();
        calls
            .iter()
            .filter_map(|op| match op {
                ApiOp::GetOrCreateJournal { file_name } => Some(file_name.clone()),
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

    // The two questions may reach the shell in either order: the core may take the two messages
    // on two threads (#1168). Each is answered once the shell has it
    let answers = [(first, false), (second, true)];
    for answered in 0..answers.len() {
        let asked = || {
            let ops = screen.ui_ops();
            ops.into_iter()
                .filter(|op| answers.iter().any(|(question, _)| question == op))
                .collect::<Vec<_>>()
        };
        wait_until("the shell is asked one more question", || {
            asked().len() > answered
        });
        let question = asked()[answered].clone();
        let ok = answers.iter().find(|(q, _)| *q == question).unwrap().1;
        screen.answer(&question, &UiOutput::Bool(ok));
    }

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

// 時計が次の日になり、シェルが「日付が変わったかもしれない」と知らせると、`/today`は新しい日を開く。日付は、Rust側の時計から入る。
#[test]
fn the_journal_of_today_opens_the_new_day_when_the_shell_tells_of_it() {
    let screen = Screen::new();
    screen.shell.open("/today").unwrap();
    wait_until("the journal of today is in the view", || {
        screen.view().contains("2026年11月6日 金曜日 (Today)")
    });

    screen.day.store(7, Ordering::SeqCst);
    screen.send(&Event::DateMayHaveChanged);

    wait_until("the journal of the new day is asked for", || {
        screen.journals_asked() == ["2026-11-06", "2026-11-07"]
    });
    wait_until("the journal of the new day is in the view", || {
        screen.view().contains("2026年11月7日 土曜日 (Today)")
    });
}

// 拡張を動かさないジャーナルでは、日付が変わっても`/today`は前の日のままで、新しい日をジャーナルに頼まない。テンプレートのない空のジャーナルを作らないためである（#1307）。
#[test]
fn the_journal_of_today_stays_on_its_day_on_a_journal_without_extensions() {
    let screen = Screen::without_extensions();
    screen.shell.open("/today").unwrap();
    wait_until("the journal of today is in the view", || {
        screen.view().contains("2026年11月6日 金曜日 (Today)")
    });

    screen.day.store(7, Ordering::SeqCst);
    screen.send(&Event::DateMayHaveChanged);
    // As long as the journal of the new day takes to be asked for on a journal with them
    thread::sleep(Duration::from_millis(100));

    assert_eq!(screen.journals_asked(), ["2026-11-06"]);
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

// シェルは、戻ると進むを、日付を渡さずに頼める。戻ると前のページが、進むと戻る前のページが、もう一度開く。
#[test]
fn the_shell_goes_back_and_forward_without_a_date() {
    let screen = Screen::new();
    screen.shell.open("/pages/first").unwrap();
    screen.shell.open("/pages/second").unwrap();

    screen.shell.back();
    wait_until("the page before is opened again", || {
        screen.answered.lock().unwrap().len() == 3
    });
    screen.shell.forward();

    wait_until("the page after is opened again", || {
        screen.answered.lock().unwrap().len() == 4
    });
    assert_eq!(
        *screen.answered.lock().unwrap(),
        ["first", "second", "first", "second"]
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

/// Presses a key in the editor of item `a`, whose text is `ab` with the cursor at `cursor`.
fn press(screen: &Screen, key: Key, shift: bool, cursor: u64) -> bool {
    let press = KeyPress::InEditor {
        item_id: "a".into(),
        chord: Chord {
            key,
            shift,
            ctrl: false,
            alt: false,
            cmd: false,
        },
        editor: EditorState {
            text: "ab".into(),
            selection_start: cursor,
            selection_end: cursor,
            on_first_line: true,
            on_last_line: true,
        },
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &press).unwrap();
    screen.shell.press(&bytes).unwrap()
}

// シェルが渡したキーは、コアが意味を決めて送る。Enterは項目を分ける要求になる。
#[test]
fn a_key_of_the_shell_is_sent_as_the_event_the_core_decides() {
    let screen = Screen::new();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });

    assert!(press(&screen, Key::Enter, false, 1));

    wait_until("the item is split", || screen.calls_named("SplitItem") == 1);
}

// アウトラインに意味のないキーは送られず、シェルは編集欄に任せる。Shift+Enterは項目の中で改行し、途中の←は文字の中で動く。
#[test]
fn a_key_that_means_nothing_to_the_outline_is_left_to_the_shell() {
    let screen = Screen::new();

    assert!(!press(&screen, Key::Enter, true, 1));
    assert!(!press(&screen, Key::Left, false, 1));
}

// 読めないキーは、誤りとしてシェルに返る。
#[test]
fn a_key_that_cannot_be_read_is_an_error() {
    let screen = Screen::new();

    assert!(screen.shell.press(&[0xff]).is_err());
}

fn drag(screen: &Screen, step: DragMove) -> bool {
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &step).unwrap();
    screen.shell.drag(&bytes).unwrap()
}

// テキストから出たドラッグはコアに伝わり、シェルは画面を描き直す。テキストの中の動きは伝わらない。
#[test]
fn a_drag_that_leaves_the_text_is_told_to_the_core() {
    let screen = Screen::new();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    let pressed = DragMove::Pressed {
        item_id: "a".into(),
        top: 100.0,
        bottom: 130.0,
    };
    let at = |y| DragMove::Moved {
        y,
        item_under: Some("a".into()),
    };

    assert!(!drag(&screen, pressed));
    assert!(!drag(&screen, at(120.0)));
    assert!(drag(&screen, at(200.0)));
    assert!(drag(&screen, DragMove::Released));
}

// 読めないドラッグは、誤りとしてシェルに返る。
#[test]
fn a_drag_that_cannot_be_read_is_an_error() {
    let screen = Screen::new();

    assert!(screen.shell.drag(&[0xff]).is_err());
}

// シェルが渡した入力は、コアに届いて保存される。
#[test]
fn what_the_shell_hands_as_typed_is_saved_by_the_core() {
    let screen = Screen::new();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    screen.send(&Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    let typing = Typing::Typed {
        item_id: "a".into(),
        body: "typed into a".into(),
        cursor: 12,
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &typing).unwrap();

    screen.shell.typed(&bytes).unwrap();
    screen.send(&Event::EditFinished {
        item_id: "a".into(),
    });

    wait_until("the typed text is saved", || {
        let calls = screen.calls.lock().unwrap();
        calls
            .iter()
            .any(|op| format!("{op:?}").contains("typed into a"))
    });
}

// シェルを閉じると、保存を待っていた本文が、閉じる前に保存される（#766）。
#[test]
fn closing_the_shell_saves_the_body_waiting_to_be_saved() {
    let screen = Screen::new();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    screen.send(&Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Unplaced,
    });
    let typing = Typing::Typed {
        item_id: "a".into(),
        body: "typed slowly into a".into(),
        cursor: 19,
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &typing).unwrap();
    screen.shell.typed(&bytes).unwrap();

    screen.shell.close();

    let calls = screen.calls.lock().unwrap();
    assert!(calls
        .iter()
        .any(|op| format!("{op:?}").contains("typed slowly into a")));
}

// 保存を待つものがなければ、シェルはすぐに閉じる。待つ上限まで止まらない。
#[test]
fn closing_with_nothing_to_save_returns_at_once() {
    let screen = Screen::new();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    let started = Instant::now();

    screen.shell.close();

    assert!(started.elapsed() < Duration::from_secs(1));
}

// 読めない入力は、誤りとしてシェルに返る。
#[test]
fn typing_that_cannot_be_read_is_an_error() {
    let screen = Screen::new();

    assert!(screen.shell.typed(&[0xff]).is_err());
}

// ---- images ----

/// A file with the content of an image, in a temporary folder of the test.
fn image_file(dir: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, b"drawing").unwrap();
    path
}

impl Screen {
    /// The texts the server was asked to paste, oldest first.
    fn pastes(&self) -> Vec<String> {
        let calls = self.calls.lock().unwrap();
        calls
            .iter()
            .filter_map(|op| match op {
                ApiOp::PasteText { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }
}

// シェルがパスで渡した画像は、サーバーに保存され、そのリンクが項目に貼られる（ADR 071）。
#[test]
fn an_image_the_shell_pastes_by_its_path_is_saved_and_linked_in_the_item() {
    let screen = Screen::new();
    let dir = tempfile::tempdir().unwrap();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });

    screen
        .shell
        .paste_image("a", &image_file(&dir, "pasted.png"), 0);

    wait_until("the link is pasted", || !screen.pastes().is_empty());
    assert_eq!(*screen.uploads.lock().unwrap(), [b"drawing".to_vec()]);
    assert_eq!(screen.pastes(), ["![pasted.png](../assets/saved.png)"]);
}

// 貼った画像の一時ファイルは、保存が終わると消える。保存に失敗しても消える（ADR 071）。
#[test]
fn the_file_of_a_pasted_image_is_deleted_once_the_server_has_answered() {
    let screen = Screen::new();
    let dir = tempfile::tempdir().unwrap();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    let saved = image_file(&dir, "pasted.png");
    let refused = image_file(&dir, "broken.png");

    screen.shell.paste_image("a", &saved, 0);
    screen.shell.paste_image("a", &refused, 0);

    wait_until("the server has taken both files", || {
        screen.uploads.lock().unwrap().len() == 2
    });
    wait_until("both files are deleted", || {
        !saved.exists() && !refused.exists()
    });
}

/// The ids of the rows a reset gives the shell.
fn reset_ids(changes: &Changes) -> Vec<String> {
    match changes.rows.as_slice() {
        [RowChange::Reset { items }] => items.iter().map(|row| row.item.id.clone()).collect(),
        other => panic!("not one reset: {other:?}"),
    }
}

/// A screen with the page `notes` open, as the view model shows it.
fn screen_with_notes() -> Screen {
    let screen = Screen::new();
    screen.open_notes();
    wait_until("the page is in the view", || {
        screen.view().contains("the body of a")
    });
    screen
}

// シェルが差分を順に当てた画面は、コアの画面と同じである。ページを開いたとき、入力したとき、検索したときである（ADR 076）。
#[test]
fn the_screen_a_shell_makes_from_the_changes_is_that_of_the_core() {
    let screen = screen_with_notes();
    // What the core shows is read first: its threads may change it meanwhile
    let shows = || {
        let core = screen.shell.whole_view();
        (screen.shell.changes_from(core.clone()), core)
    };
    let (first, core) = shows();
    // Every part of the first view of the shell comes with the first changes
    let mut shell = applied(Core::<BorderApp>::new().view(), first);
    assert_eq!(shell, core);

    let typing = Typing::Changed {
        item_id: "a".into(),
        body: "typed into a".into(),
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &typing).unwrap();
    screen.shell.typed(&bytes).unwrap();
    let (changes, core) = shows();
    shell = applied(shell, changes);
    assert_eq!(shell, core);

    screen.search("notes");
    // Opening the page began timers of its own before the one of the search
    screen.let_time_pass(screen.waits.begun.load(Ordering::SeqCst));
    wait_until("the server is asked to search", || {
        !screen.searches().is_empty()
    });
    assert_eq!(screen.shell.whole_view().search.query, "notes");
    let (changes, core) = shows();
    shell = applied(shell, changes);
    assert_eq!(shell, core);
}

// 開いたページの最初の差分は、行の入れ替えと、全部のパネルである。
#[test]
fn the_first_changes_of_an_opened_page_are_a_reset_and_every_panel() {
    let screen = screen_with_notes();

    let changes = screen.shell.take_changes();

    assert_eq!(reset_ids(&changes), ["a"]);
    assert_eq!(changes.panels.len(), 17);
}

// 続けて読んだ差分は、空である。
#[test]
fn changes_read_again_at_once_are_empty() {
    let screen = screen_with_notes();
    screen.shell.take_changes();

    let changes = screen.shell.take_changes();

    assert!(changes.rows.is_empty() && changes.panels.is_empty());
}

// 入力した本文は、その行の更新として届く。
#[test]
fn a_typed_body_comes_as_an_update_of_its_row() {
    let screen = screen_with_notes();
    screen.shell.take_changes();
    let typing = Typing::Changed {
        item_id: "a".into(),
        body: "typed into a".into(),
    };
    let mut bytes = Vec::new();
    BincodeFfiFormat::serialize(&mut bytes, &typing).unwrap();

    screen.shell.typed(&bytes).unwrap();

    let changes = screen.shell.take_changes();
    let [RowChange::Update { at: 0, item }] = changes.rows.as_slice() else {
        panic!("not one update of the first row: {:?}", changes.rows);
    };
    assert_eq!(item.item.body, "typed into a");
}

// ブラウザで開くjournalサーバーのアドレスには、サーバーのアドレスが前に付く。外へのリンクは、そのまま開く（ADR 083）。
#[test]
fn an_address_of_the_server_opens_in_the_browser_with_the_server_before_it() {
    let screen = Screen::new();
    let click = |link| Event::LinkClicked {
        link,
        held: Held::default(),
    };

    screen.send(&click(RichLink::File {
        path: "assets/a b.png".into(),
    }));
    screen.send(&click(RichLink::Web {
        url: "https://example.com/".into(),
    }));

    let opened = |address: &str| UiOp::OpenInBrowser {
        address: address.into(),
    };
    wait_until("both open in the browser", || {
        let ops = screen.ui_ops();
        ops.contains(&opened("http://localhost:8080/assets/a%20b.png"))
            && ops.contains(&opened("https://example.com/"))
    });
}
