use super::*;
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent, UiOp,
};
use client_core::model::{Page, SearchResult, Split, TreeExport, Uploaded};
use client_core::rest::ClientId;
use client_core::screens::page::ATLAS_NAME;
use client_core::testing::{page_answer, plain_item};
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
    stream: RefCell<Option<Listener>>,
    /// The timers that run, with what each calls when its time comes.
    timers: RefCell<Vec<(u64, Done<()>)>>,
    /// What was written to the clipboard: the Markdown and the HTML.
    copied: RefCell<Vec<(String, String)>>,
    /// The files sent to the server, oldest first.
    uploads: RefCell<Vec<String>>,
    /// What the local storage of the browser holds, by its key.
    stored: RefCell<Vec<(&'static str, &'static str)>>,
    /// The URLs the address bar was given, and whether each replaced the one before.
    urls: RefCell<Vec<(String, bool)>>,
    asked: RefCell<Vec<(u32, UiOp)>>,
    /// The operations of the screen the browser did itself, oldest first.
    done_by_window: RefCell<Vec<UiOp>>,
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

    fn listen(&self, heard: Listener) {
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

    fn screen(&self, op: &UiOp, done: Done<UiOutput>) {
        self.0.done_by_window.borrow_mut().push(op.clone());
        // The user gives no name to the page
        if let UiOp::Prompt(_) = op {
            done(UiOutput::Text(Some(String::new())));
        }
    }

    fn origin(&self) -> String {
        "http://app.test".into()
    }

    fn copy(&self, markdown: &str, html: &str) {
        let copied = (markdown.to_string(), html.to_string());
        self.0.copied.borrow_mut().push(copied);
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
            let page = Page {
                id: "p".into(),
                name: "notes".into(),
                page_type: "doc".into(),
                ..Default::default()
            };
            let items = vec![
                plain_item("a", 1, "the body of a"),
                plain_item("b", 1, "the body of b"),
            ];
            Ok(page_answer(op, page, items))
        }
        // The text of the item stays in it, and the new item below is empty
        ApiOp::SplitItem { item_id, body, .. } => {
            let mut created = plain_item("new", 1, "");
            created.idx = format!("{item_id}\u{1}");
            Ok(ApiOutput::Split(Box::new(Split {
                item: plain_item(item_id, 1, body),
                created,
            })))
        }
        ApiOp::SearchPageNames { .. } => Ok(ApiOutput::SearchResults(
            ["notes", "novel"]
                .map(|title| SearchResult {
                    page_id: title.into(),
                    page_title: title.into(),
                    ..SearchResult::default()
                })
                .to_vec(),
        )),
        ApiOp::ExportTreeMarkdown { .. } => Ok(ApiOutput::TreeExport(TreeExport {
            tree_markdown: "- see [[notes]]".into(),
            flat_markdown: "see [[notes]]".into(),
        })),
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
        self.go_and_serve("/pages/notes");
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

    fn go_and_serve(&self, url: &str) {
        self.bridge.go(url);
        self.serve();
    }

    /// The number the shell was given the operation with.
    fn id_of(&self, op: &UiOp) -> u32 {
        let asked = self.browser.asked.borrow();
        let found = asked.iter().find(|(_, asked)| asked == op);
        found.expect("the shell was asked for this").0
    }

    /// Answers the operation the way the shell does: by its number, with JSON.
    fn answer(&self, op: &UiOp, output: &str) {
        self.bridge.resolve(self.id_of(op), output).unwrap();
    }
}

// シェルが開いたページは、サーバーから読まれ、答えが来るとシェルに描き直しを知らせる。
#[test]
fn a_page_the_shell_opens_is_read_and_the_shell_is_told_to_draw_it_when_the_answer_comes() {
    let screen = Screen::new();
    screen.bridge.go("/pages/notes");
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
    screen.open_notes();
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
    assert_eq!(screen.asked(), std::slice::from_ref(&confirm));

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

    let urls = screen.urls();
    assert_eq!(urls.last(), Some(&("/today".to_string(), false)));
    assert_eq!(screen.calls_named("GetOrCreateJournal"), journals + 1);
}

// 何も指さないURLは、今日のページに置き換わる。
#[test]
fn a_url_that_names_nothing_is_replaced_by_the_page_of_today() {
    let screen = Screen::new();

    screen.bridge.popped("/nothing/here");

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
        changes.contains(r#""sort":{"key":"Modified","order":"Desc"}"#),
        "{changes}"
    );
    let urls = screen.urls();
    assert_eq!(urls.last().unwrap().0, format!("/pages/{ATLAS_NAME}"));
}

// windowのAPIで済む画面の操作はブラウザがこなし、今の時刻はシェルに尋ねずに答える。名前のない新しいページは、その時刻から名前を得る。
#[test]
fn the_window_does_its_operations_and_the_time_is_answered_without_the_shell() {
    let screen = Screen::new();

    screen
        .bridge
        .update(r#"{"CreateNewPage":{"search_query":""}}"#)
        .unwrap();

    let by_window = screen.browser.done_by_window.borrow().clone();
    assert_eq!(by_window, [UiOp::Prompt("Page title:".into())]);
    assert!(screen.asked().is_empty());
    let url = "/pages/Untitled-202602100900".to_string();
    assert_eq!(screen.urls(), [(url, false)]);
}

/// A key pressed in the editor of the item `a`, whose text is `ab`, with the caret at its end.
fn key_in_a(key: &str) -> String {
    format!(
        r#"{{"InEditor":{{"item_id":"a",
            "chord":{{"key":{key},"shift":false,"ctrl":false,"alt":false,"cmd":false}},
            "editor":{{"text":"ab","selection_start":2,"selection_end":2,
                      "on_first_line":true,"on_last_line":true}}}}}}"#
    )
}

// アウトラインに意味のないキーは、シェルの編集欄に任される。意味のあるキーは、差分を返す。
#[test]
fn a_key_that_means_nothing_to_the_outline_is_left_to_the_editor_and_one_that_does_gives_changes() {
    let screen = Screen::new();
    screen.open_notes();
    screen.bridge.changes();

    let letter = screen.bridge.press(&key_in_a(r#"{"Letter":"x"}"#)).unwrap();
    let down = screen.bridge.press(&key_in_a(r#""Down""#)).unwrap();

    assert_eq!(letter, None);
    let changes = down.expect("the key moves to the next item");
    assert!(changes.contains(r#""focused_item_id":"b""#), "{changes}");
}

// 編集欄への入力は、行の本文を変える。
#[test]
fn typing_in_an_editor_changes_the_body_of_its_row() {
    let screen = Screen::new();
    screen.open_notes();
    screen.bridge.changes();

    let changes = screen
        .bridge
        .typed(r#"{"Typed":{"item_id":"a","body":"typed text","cursor":10}}"#)
        .unwrap();

    assert!(changes.contains("typed text"), "{changes}");
}

// 本文から出たドラッグは、項目の選択に変わる。本文の中のドラッグは、シェルの編集欄に任される。
#[test]
fn a_drag_that_leaves_the_text_selects_items() {
    let screen = Screen::new();
    screen.open_notes();
    screen.bridge.changes();
    let pressed = screen
        .bridge
        .drag(r#"{"Pressed":{"item_id":"a","top":0.0,"bottom":20.0}}"#)
        .unwrap();
    let in_text = screen
        .bridge
        .drag(r#"{"Moved":{"y":10.0,"item_under":"a"}}"#)
        .unwrap();

    let beyond = screen
        .bridge
        .drag(r#"{"Moved":{"y":60.0,"item_under":"b"}}"#)
        .unwrap();

    assert_eq!((pressed, in_text), (None, None));
    let changes = beyond.expect("the drag selects items");
    assert!(changes.contains(r#""is_selected":true"#), "{changes}");
}

// コピーしたMarkdownは、アプリのページへのリンクを持つHTMLとともに、クリップボードに書かれる。
#[test]
fn copied_markdown_is_written_with_html_whose_links_open_the_pages_of_the_app() {
    let screen = Screen::new();
    screen.open_notes();
    let select = [
        r#"{"Pressed":{"item_id":"a","top":0.0,"bottom":20.0}}"#,
        r#"{"Moved":{"y":60.0,"item_under":"a"}}"#,
        r#""Released""#,
    ];
    for step in select {
        screen.bridge.drag(step).unwrap();
    }

    screen.bridge.update(r#"{"SelectionKey":"Copy"}"#).unwrap();
    screen.serve();

    let copied = screen.browser.copied.borrow();
    let [(markdown, html)] = copied.as_slice() else {
        panic!("one copy is written: {copied:?}");
    };
    assert_eq!(markdown, "- see [[notes]]");
    assert!(
        html.contains(r#"href="http://app.test/pages/notes""#),
        "{html}"
    );
}

// シェルが開いたリンクは、URLを履歴に積んでから、そのページを開く。
#[test]
fn a_link_the_shell_opens_goes_into_the_history_and_its_page_opens() {
    let screen = Screen::new();

    screen.bridge.go("/pages/notes");
    screen.serve();

    assert_eq!(screen.urls(), [("/pages/notes".to_string(), false)]);
    let changes = screen.bridge.changes();
    assert!(changes.contains("the body of a"), "{changes}");
}

// ブラウザの履歴で戻ると、そのページが開き、シェルに描き直しを知らせる。
#[test]
fn going_back_in_the_history_opens_the_page_and_tells_the_shell_to_draw_it() {
    let screen = Screen::new();
    screen.bridge.start("/today");
    screen.serve();
    screen.bridge.changes();
    let renders = screen.renders();

    screen.bridge.popped("/pages/notes");
    screen.serve();

    assert!(screen.renders() > renders);
    let changes = screen.bridge.changes();
    assert!(changes.contains("the body of a"), "{changes}");
}

// 編集欄へのフォーカスは番号つきでシェルに渡り、シェルの答えがコアに届く。
#[test]
fn the_focus_of_an_editor_reaches_the_shell_with_a_number_and_its_answer_reaches_the_core() {
    let screen = Screen::new();
    screen.open_notes();

    screen.bridge.press(&key_in_a(r#""Enter""#)).unwrap();
    screen.serve();
    let focus = screen
        .asked()
        .into_iter()
        .find(|op| matches!(op, UiOp::FocusEditor { item_id: Some(id), .. } if id == "new"));
    let focus = focus.expect("the editor of the new item is to have the focus");

    screen.answer(&focus, r#""Done""#);
    // Answered once: the core waits for it no more
    let again = screen.bridge.resolve(screen.id_of(&focus), r#""Done""#);
    assert!(again.is_err());
}

// 項目までのスクロールはシェルに渡る。知らせなので、シェルの答えは待たない。
#[test]
fn a_scroll_to_an_item_reaches_the_shell_and_waits_for_no_answer() {
    let screen = Screen::new();
    screen.go_and_serve("/pages/notes#b");

    let scroll = UiOp::ScrollToItem {
        item_id: "b".into(),
        smooth: false,
    };
    assert!(screen.asked().contains(&scroll), "{:?}", screen.asked());
    let answer = screen.bridge.resolve(screen.id_of(&scroll), r#""Done""#);
    assert!(answer.is_err());
}

// 貼られていないファイルの貼り付けは、サーバーに送られない。
#[test]
fn a_paste_of_a_file_that_was_never_pasted_sends_nothing_to_the_server() {
    let screen = Screen::new();
    screen.open_notes();

    screen
        .bridge
        .update(r#"{"PasteImage":{"item_id":"a","file":99,"cursor":0}}"#)
        .unwrap();
    screen.serve();

    assert!(screen.browser.uploads.borrow().is_empty());
}

// シェルがなくなったあとに来た答えとタイマーは、何もしない。
#[test]
fn an_answer_or_a_timer_that_comes_after_the_shell_is_gone_does_nothing() {
    let screen = Screen::new();
    screen.bridge.start("/pages/notes");
    screen
        .bridge
        .update(r#"{"SearchQueryChanged":"a query"}"#)
        .unwrap();
    let Screen { bridge, browser } = screen;

    drop(bridge);

    let calls = std::mem::take(&mut *browser.calls.borrow_mut());
    for (op, done) in calls {
        done(answer_of(&op));
    }
    let timers = std::mem::take(&mut *browser.timers.borrow_mut());
    for (_, fired) in timers {
        fired(());
    }
    let mut stream = browser.stream.borrow_mut();
    (stream.as_mut().expect("the stream was opened"))(SseEvent::Open);
}

// サーバーが忙しい間に続けて貼った2つの画像は、どちらもそのファイルのままサーバーへ送られる。
#[test]
fn two_images_pasted_while_the_server_is_busy_are_both_sent_as_they_are() {
    let screen = Screen::new();
    screen.open_notes();
    // The server has not answered yet when the images are pasted
    screen
        .bridge
        .update(r#"{"DeleteItem":{"item_id":"a"}}"#)
        .unwrap();

    screen
        .bridge
        .paste_image("b", "the first picture".into(), 0);
    screen
        .bridge
        .paste_image("b", "the second picture".into(), 0);
    screen.serve();

    let uploads = screen.browser.uploads.borrow();
    assert_eq!(*uploads, ["the first picture", "the second picture"]);
}

// シェルに同時に渡した操作は、それぞれの番号で答えられる。
#[test]
fn operations_the_shell_holds_at_once_are_each_answered_by_their_own_number() {
    let screen = Screen::new();
    screen.open_notes();
    screen.bridge.press(&key_in_a(r#""Enter""#)).unwrap();
    screen.serve();
    screen
        .bridge
        .update(r#"{"DeleteItem":{"item_id":"b"}}"#)
        .unwrap();
    screen.serve();
    let [UiOp::FocusEditor { .. }, UiOp::ConfirmDestructive(_)] = screen.asked().as_slice() else {
        panic!(
            "the shell holds a focus and a confirm: {:?}",
            screen.asked()
        );
    };
    let (focus, confirm) = (screen.asked()[0].clone(), screen.asked()[1].clone());

    screen.answer(&confirm, r#"{"Bool":true}"#);
    screen.answer(&focus, r#""Done""#);

    screen.serve();
    assert_eq!(screen.calls_named("DeleteItems"), 1);
}

/// Opens the editor of the item a and types a wiki link query there. With `served`, the
/// page names found for it are offered.
fn query_in_a(screen: &Screen, served: bool) {
    screen.open_notes();
    screen
        .bridge
        .update(r#"{"EditRequested":{"item_id":"a","place":"Unplaced"}}"#)
        .unwrap();
    let query = r#"{"WikiQuery":{"item_id":"a","context":{"query":"no","x":1.0,"y":2.0}}}"#;
    screen.bridge.update(query).unwrap();
    screen.let_time_pass();
    if served {
        screen.serve();
    }
}

// [[の補完の候補が出ている間、上下のキーは候補を選び、編集欄のカーソルは動かさない。上は、先頭から末尾へ回る。
#[test]
fn while_page_names_are_offered_the_arrow_keys_pick_one() {
    let screen = Screen::new();
    query_in_a(&screen, true);

    let up = screen.bridge.press(&key_in_a(r#""Up""#)).unwrap();
    let down = screen.bridge.press(&key_in_a(r#""Down""#)).unwrap();

    let up = up.expect("the popup takes Up");
    let down = down.expect("the popup takes Down");
    assert!(up.contains(r#""selected_index":1"#), "{up}");
    assert!(down.contains(r#""selected_index":0"#), "{down}");
}

// 候補が出ている間、Enterは選んだ候補を入れて候補を閉じ、Escapeは候補を閉じる。
#[test]
fn while_page_names_are_offered_enter_and_escape_close_them() {
    for key in [r#""Enter""#, r#""Escape""#] {
        let screen = Screen::new();
        query_in_a(&screen, true);

        let changes = screen.bridge.press(&key_in_a(key)).unwrap();

        let changes = changes.expect("the popup takes the key");
        assert!(changes.contains(r#""suggestions":[]"#), "{key}: {changes}");
    }
}

// 候補がまだ届いていない間は、下のキーはアウトラインのキーとして次の項目へ移る。
#[test]
fn before_page_names_arrive_the_arrow_keys_move_in_the_outline() {
    let screen = Screen::new();
    query_in_a(&screen, false);

    let changes = screen.bridge.press(&key_in_a(r#""Down""#)).unwrap();

    let changes = changes.expect("the outline takes the key");
    assert!(changes.contains(r#""focused_item_id":"b""#), "{changes}");
}
