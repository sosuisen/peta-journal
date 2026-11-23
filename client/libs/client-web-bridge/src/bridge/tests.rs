use super::*;
use client_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, UiOp};
use client_core::model::{Page, PageWithItems, Uploaded};
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
    /// The files sent to the server, oldest first.
    uploads: RefCell<Vec<String>>,
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
        let bridge = Bridge::new(FakeHost(browser.clone()));
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
