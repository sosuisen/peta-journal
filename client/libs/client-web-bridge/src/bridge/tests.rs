use super::*;
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent, UiOp,
};
use client_core::model::{Page, PageWithItems, Uploaded};
use client_core::rest::ClientId;
use client_core::screens::page::ATLAS_NAME;
use client_core::testing::plain_item;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

/// What the fake browser was asked for, and the answers it still owes.
#[derive(Default)]
struct Browser {
    calls: RefCell<VecDeque<(ApiOp, Done<ApiResult<ApiOutput>>)>>,
    /// Every request to the server, oldest first, answered or not.
    called: RefCell<Vec<ApiOp>>,
    /// What the stream of the server calls with each of its events.
    stream: RefCell<Option<Box<dyn FnMut(SseEvent)>>>,
    /// The timers that run, with what each calls when its time comes.
    timers: RefCell<Vec<(u64, Done<()>)>>,
    /// The files sent to the server, oldest first.
    uploads: RefCell<Vec<String>>,
    /// What the local storage of the browser holds, by its key.
    stored: RefCell<Vec<(&'static str, &'static str)>>,
    /// The URLs the address bar was given, and whether each replaced the one before.
    urls: RefCell<Vec<(String, bool)>>,
    asked: RefCell<Vec<(u32, UiOp)>>,
    renders: Cell<usize>,
}

/// A browser that answers nothing by itself: the test lets the server answer.
struct FakeHost(Rc<Browser>);

impl Host for FakeHost {
    type File = String;

    fn now(&self) -> (LocalDateTimeDto, u64) {
        let local = LocalDateTimeDto {
            year: 2026,
            month: 2,
            day: 10,
            hour: 9,
            minute: 0,
        };
        (local, 1_770_000_000_000)
    }

    fn call(&self, op: ApiOp, done: Done<ApiResult<ApiOutput>>) {
        self.0.called.borrow_mut().push(op.clone());
        self.0.calls.borrow_mut().push_back((op, done));
    }

    fn upload(&self, file: String, done: Done<ApiResult<ApiOutput>>) {
        self.0.uploads.borrow_mut().push(file);
        let uploaded = Uploaded {
            file_name: "saved.png".into(),
            original_name: "picture.png".into(),
        };
        done(Ok(ApiOutput::Uploaded(uploaded)));
    }

    fn listen(&self, heard: Box<dyn FnMut(SseEvent)>) {
        *self.0.stream.borrow_mut() = Some(heard);
    }

    fn start_timer(&self, id: u64, _ms: u32, fired: Done<()>) {
        self.0.timers.borrow_mut().push((id, fired));
    }

    fn cancel_timer(&self, id: u64) {
        self.0
            .timers
            .borrow_mut()
            .retain(|(running, _)| *running != id);
    }

    fn load(&self, key: &str) -> Option<String> {
        let stored = self.0.stored.borrow();
        let found = stored.iter().find(|(stored, _)| *stored == key);
        found.map(|(_, value)| value.to_string())
    }

    fn go_to(&self, url: &str, replace: bool) {
        self.0.urls.borrow_mut().push((url.to_string(), replace));
    }

    fn ask(&self, id: u32, op: &UiOp) {
        self.0.asked.borrow_mut().push((id, op.clone()));
    }

    fn rendered(&self) {
        self.0.renders.set(self.0.renders.get() + 1);
    }
}

/// The answer of the fake server.
fn answer_of(op: &ApiOp) -> ApiResult<ApiOutput> {
    match op {
        ApiOp::GetOrCreateDoc { .. } | ApiOp::GetPageById { .. } => {
            Ok(ApiOutput::Page(PageWithItems {
                page: Page {
                    id: "p".into(),
                    name: "notes".into(),
                    page_type: "doc".into(),
                    ..Default::default()
                },
                items: vec![
                    plain_item("a", 1, "the body of a"),
                    plain_item("b", 1, "the body of b"),
                ],
                created: false,
                transcluded_trees: None,
            }))
        }
        // Every item is transcluded by an item on another page
        ApiOp::TranscludedItems { item_ids } => Ok(ApiOutput::ItemIds(item_ids.clone())),
        _ => Err(ApiError("the fake server does not know this".into())),
    }
}

/// The web shell on a fake browser.
struct Screen {
    bridge: Rc<Bridge<FakeHost>>,
    browser: Rc<Browser>,
}

impl Screen {
    fn new() -> Screen {
        let browser = Rc::new(Browser::default());
        let bridge = Bridge::new(FakeHost(browser.clone()), ClientId::new());
        Screen { bridge, browser }
    }

    /// Lets the server answer every request, those its answers bring too.
    fn serve(&self) {
        loop {
            let next = self.browser.calls.borrow_mut().pop_front();
            let Some((op, done)) = next else {
                return;
            };
            done(answer_of(&op));
        }
    }

    fn renders(&self) -> usize {
        self.browser.renders.get()
    }

    fn open_notes(&self) {
        self.bridge.route("/pages/notes");
        self.serve();
    }

    fn server_says(&self, event: SseEvent) {
        let mut stream = self.browser.stream.borrow_mut();
        let heard = stream.as_mut().expect("the stream of the server is open");
        heard(event);
    }

    /// Lets the time of every timer that runs pass.
    fn let_time_pass(&self) {
        let timers = std::mem::take(&mut *self.browser.timers.borrow_mut());
        for (_, fired) in timers {
            fired(());
        }
    }

    /// The queries the server was asked to search for, oldest first.
    fn searches(&self) -> Vec<String> {
        let called = self.browser.called.borrow();
        let searches = called.iter().filter_map(|op| match op {
            ApiOp::SearchCombined { query, .. } => Some(query.clone()),
            _ => None,
        });
        searches.collect()
    }

    fn urls(&self) -> Vec<(String, bool)> {
        self.browser.urls.borrow().clone()
    }

    /// How often the server was asked for this operation, by the name of its variant.
    fn calls_named(&self, name: &str) -> usize {
        let called = self.browser.called.borrow();
        let named = called
            .iter()
            .filter(|op| format!("{op:?}").starts_with(name));
        named.count()
    }

    /// The operations the shell was asked for, oldest first.
    fn asked(&self) -> Vec<UiOp> {
        let asked = self.browser.asked.borrow();
        asked.iter().map(|(_, op)| op.clone()).collect()
    }

    /// Answers the operation the way the shell does: by its number, with JSON.
    fn answer(&self, op: &UiOp, output: &str) {
        let id = {
            let asked = self.browser.asked.borrow();
            let found = asked.iter().find(|(_, asked)| asked == op);
            found.expect("the shell was asked for this").0
        };
        self.bridge.resolve(id, output).unwrap();
    }
}

// シェルが開いたページは、サーバーから読まれ、答えが来るとシェルに描き直しを知らせる。
#[test]
fn a_page_the_shell_opens_is_read_and_the_shell_is_told_to_draw_it_when_the_answer_comes() {
    let screen = Screen::new();
    screen.bridge.route("/pages/notes");
    assert_eq!(screen.renders(), 0);

    screen.serve();

    assert!(screen.renders() > 0);
    let changes = screen.bridge.changes();
    assert!(changes.contains("the body of a"), "{changes}");
}

// シェルの呼び出しの中で変わった画面は、戻り値の差分で返り、描き直しは知らせない。
#[test]
fn what_a_call_of_the_shell_changes_comes_back_as_its_changes_without_a_render() {
    let screen = Screen::new();
    screen.bridge.route("/pages/notes");
    screen.serve();
    screen.bridge.changes();
    let renders = screen.renders();

    let changes = screen
        .bridge
        .update(r#"{"SearchQueryChanged":"a query"}"#)
        .unwrap();

    assert!(changes.contains("a query"), "{changes}");
    assert_eq!(screen.renders(), renders);
}

// 2つ目の破壊的な確認は、1つ目が答えられるまでシェルに届かない。
#[test]
fn a_second_destructive_confirm_reaches_the_shell_after_the_first_is_answered() {
    let screen = Screen::new();
    screen.open_notes();
    let confirm = UiOp::ConfirmDestructive(
        "1 item to delete is transcluded by another item. \
         Delete anyway? Each transclusion keeps a plain copy."
            .into(),
    );

    screen
        .bridge
        .update(r#"{"DeleteItem":{"item_id":"a"}}"#)
        .unwrap();
    screen
        .bridge
        .update(r#"{"DeleteItem":{"item_id":"b"}}"#)
        .unwrap();
    screen.serve();
    assert_eq!(screen.asked(), [confirm.clone()]);

    screen.answer(&confirm, r#"{"Bool":false}"#);
    screen.serve();

    assert_eq!(screen.asked(), [confirm.clone(), confirm]);
}

// コアが求めたページの移動は、URLを変えてから、そのページを開く。
#[test]
fn a_navigation_the_core_asks_for_changes_the_url_and_then_opens_its_page() {
    let screen = Screen::new();
    screen.open_notes();
    let journals = screen.calls_named("GetOrCreateJournal");

    screen.bridge.update(r#""NavigateHome""#).unwrap();

    assert_eq!(screen.urls(), [("/today".to_string(), false)]);
    assert_eq!(screen.calls_named("GetOrCreateJournal"), journals + 1);
}

// 何も指さないURLは、今日のページに置き換わる。
#[test]
fn a_url_that_names_nothing_is_replaced_by_the_page_of_today() {
    let screen = Screen::new();

    screen.bridge.route("/nothing/here");

    assert_eq!(screen.urls(), [("/today".to_string(), true)]);
    assert_eq!(screen.calls_named("GetOrCreateJournal"), 1);
}

// 貼った画像のファイルは、そのままサーバーへ送られる。
#[test]
fn the_file_of_a_pasted_image_is_sent_to_the_server_as_it_is() {
    let screen = Screen::new();
    screen.open_notes();

    screen
        .bridge
        .paste_image("a", "the file of the picture".into(), 0);
    screen.serve();

    let uploads = screen.browser.uploads.borrow();
    assert_eq!(*uploads, ["the file of the picture"]);
}

// タイマーは、時間が来るとコアに届き、取り消したものは届かない。
#[test]
fn a_timer_reaches_the_core_when_its_time_comes_and_a_cancelled_one_does_not() {
    let screen = Screen::new();
    screen.open_notes();

    screen
        .bridge
        .update(r#"{"SearchQueryChanged":"first"}"#)
        .unwrap();
    screen
        .bridge
        .update(r#"{"SearchQueryChanged":"second"}"#)
        .unwrap();
    screen.let_time_pass();

    assert_eq!(screen.searches(), ["second"]);
}

// サーバーのイベントの流れは、いくつでもコアに届く。開くたびに、最近のページが読み直される。
#[test]
fn every_event_of_the_server_stream_reaches_the_core() {
    let screen = Screen::new();
    screen.bridge.start("/today");
    screen.serve();
    let read = screen.calls_named("GetRecentPages");

    screen.server_says(SseEvent::Open);
    screen.serve();
    screen.server_says(SseEvent::Open);
    screen.serve();

    assert_eq!(screen.calls_named("GetRecentPages"), read + 2);
}

// 起動すると、ブラウザに残した並べ方とアトラスのフォルダがコアに戻る。
#[test]
fn the_start_gives_the_core_the_sorts_and_the_atlas_folders_the_browser_kept() {
    let screen = Screen::new();
    // As the web app in Leptos wrote them
    screen.browser.stored.borrow_mut().extend([
        (
            "sidebar-index-sort",
            r#"{"":{"key":"modified","order":"desc"}}"#,
        ),
        ("atlas-dirs", r#"[""]"#),
    ]);

    let changes = screen.bridge.start("/today");
    screen.bridge.update(r#""NavigateToRoot""#).unwrap();

    assert!(
        changes.contains(r#""index_sort":{"key":"Modified","order":"Desc"}"#),
        "{changes}"
    );
    let urls = screen.urls();
    assert_eq!(urls.last().unwrap().0, format!("/pages/{ATLAS_NAME}"));
}
