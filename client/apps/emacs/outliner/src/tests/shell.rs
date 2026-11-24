use crate::emacs_text::EmacsText;
use crate::protocol::{
    Bullet, DateUnit, Envelope, FreeKey, Inbound, Outbound, PropertyRow, RowView, SelectionKey,
    SidebarHeading, SidebarView,
};
use crate::shell::{Host, Input, Now, Shell};
use client_core::changes::ListChange;
use client_core::effects::{ApiError, ApiOp, ApiOutput, LocalDateTimeDto, SseEvent};
use client_core::model::{PropertyDefinition, TreeExport, Uploaded};
use client_core::rest::ClientId;
use client_core::BodyModel;
use serde_json::{json, Value};
use std::mem::ManuallyDrop;

/// Applies the changes to the rows in order, as `peta-journal.el` does.
fn apply(rows: &mut Vec<RowView>, changes: &[ListChange<RowView>]) {
    for change in changes {
        match change {
            ListChange::Insert { at, item } => rows.insert(*at as usize, item.clone()),
            ListChange::Remove { at } => {
                rows.remove(*at as usize);
            }
            ListChange::Move { from, to } => {
                let row = rows.remove(*from as usize);
                rows.insert(*to as usize, row);
            }
            ListChange::Update { at, item } => rows[*at as usize] = item.clone(),
            ListChange::Reset { items } => rows.clone_from(items),
        }
    }
}

#[derive(Default)]
pub(super) struct FakeHost {
    pub(super) out: Vec<Outbound>,
    apis: Vec<(u64, ApiOp)>,
    uploads: Vec<(u64, String)>,
    timers: Vec<(u64, u32)>,
    streams: Vec<u64>,
    /// The day of November 2026 that the clock of the host shows. The 6th when not set.
    day: Option<u8>,
    /// The rows as Emacs has them, after the changes it was sent.
    rows: Vec<RowView>,
}

impl Host for FakeHost {
    fn emit(&mut self, message: Outbound) {
        if let Outbound::Rows { changes, .. } = &message {
            apply(&mut self.rows, changes);
        }
        self.out.push(message);
    }
    fn api(&mut self, id: u64, op: ApiOp) {
        self.apis.push((id, op));
    }
    fn upload(&mut self, id: u64, path: String) {
        self.uploads.push((id, path));
    }
    fn start_timer(&mut self, id: u64, ms: u32) {
        self.timers.push((id, ms));
    }
    fn connect_sse(&mut self, stream: u64) {
        self.streams.push(stream);
    }
    fn now(&self) -> Now {
        Now {
            local: LocalDateTimeDto {
                year: 2026,
                month: 11,
                day: self.day.unwrap_or(6).into(),
                hour: 9,
                minute: 30,
            },
            now_ms: 1_000,
        }
    }
}

pub(super) fn item(id: &str, level: i32, body: &str) -> Value {
    json!({
        "id": id, "pageId": "p", "parentItemId": null, "dirPath": "", "name": "notes",
        "type": "doc", "createdDate": 0, "modifiedDate": 0, "idx": id, "level": level,
        "body": body, "ast": "", "collapsed": false, "types": [], "properties": {},
        "transcludeItemId": null,
    })
}

fn page_info(dir_path: &str, name: &str) -> Value {
    json!({
        "id": "p", "parentId": "", "dirPath": dir_path, "name": name, "type": "doc",
        "createdDate": 0, "modifiedDate": 0, "properties": {},
    })
}

/// The answer to a doc lookup of the page "notes", which the server has.
fn doc_answer() -> ApiOutput {
    serde_json::from_value(json!({
        "DocPage": { "page": page_info("", "notes"), "items": [], "created": false }
    }))
    .unwrap()
}

fn page_answer(items: &[Value]) -> ApiOutput {
    serde_json::from_value(json!({
        "Page": { "page": page_info("", "notes"), "items": items, "transcludedTrees": {} }
    }))
    .unwrap()
}

pub(super) struct Driver {
    /// Never dropped. A test ends with commands of the core still waiting, and one of them
    /// sends an event from its destructor, which panics once the core is gone. The real
    /// process ends with `process::exit`, so this only happens in a test.
    pub(super) shell: ManuallyDrop<Shell<FakeHost>>,
    seq: u64,
}

impl Driver {
    fn new() -> Driver {
        Driver {
            shell: ManuallyDrop::new(Shell::new(FakeHost::default(), ClientId::new())),
            seq: 0,
        }
    }

    /// The server tells that the client `by` changed the shown page through the API.
    fn changed_by(&mut self, by: &ClientId) {
        let stream = self.shell.host.streams[0];
        self.shell.handle(Input::Sse {
            stream,
            event: SseEvent::Message(format!(
                r#"{{"type":"pagesUpdated","pageId":"p","source":"app","clientId":"{}"}}"#,
                by.as_str()
            )),
        });
        // The core debounces the reload with a timer
        let timers: Vec<u64> = self.shell.host.timers.iter().map(|t| t.0).collect();
        for id in timers {
            self.shell.handle(Input::TimerFired { id });
        }
    }

    pub(super) fn send(&mut self, message: Inbound) {
        self.seq += 1;
        self.shell.handle(Input::Emacs(Envelope {
            seq: self.seq,
            message,
        }));
    }

    /// Answers the waiting API call the predicate picks. Panics when there is none.
    fn answer(&mut self, pick: impl Fn(&ApiOp) -> bool, output: ApiOutput) {
        let id = self.take_api(pick).expect("no such API call is waiting").0;
        self.shell.handle(Input::ApiDone {
            id,
            result: Box::new(Ok(output)),
        });
    }

    /// Answers every call still waiting with an error, for what a test does not care about.
    pub(super) fn fail_all_waiting_calls(&mut self) {
        while let Some((id, _)) = self.take_api(|_| true) {
            self.shell.handle(Input::ApiDone {
                id,
                result: Box::new(Err(ApiError("not in this test".into()))),
            });
        }
    }

    fn take_api(&mut self, pick: impl Fn(&ApiOp) -> bool) -> Option<(u64, ApiOp)> {
        let apis = &mut self.shell.host.apis;
        let index = apis.iter().position(|(_, op)| pick(op))?;
        Some(apis.remove(index))
    }

    /// Answers that the page going back or forward looks for is there.
    fn find_page(&mut self) {
        let found = serde_json::from_value(json!({ "FoundPage": page_info("", "notes") }));
        self.answer(|op| matches!(op, ApiOp::FindPage { .. }), found.unwrap());
    }

    fn waiting(&self, pick: impl Fn(&ApiOp) -> bool) -> bool {
        self.shell.host.apis.iter().any(|(_, op)| pick(op))
    }

    /// Answers the two calls that load a page: the page, then its items.
    fn load_page(&mut self, items: &[Value]) {
        self.answer(
            |op| matches!(op, ApiOp::GetOrCreateDoc { .. }),
            doc_answer(),
        );
        self.answer(
            |op| matches!(op, ApiOp::GetPageById { .. }),
            page_answer(items),
        );
    }

    /// Opens a page with these items and forgets what was said until now.
    pub(super) fn open(items: &[Value]) -> Driver {
        let mut driver = Driver::new();
        driver.send(Inbound::Start {
            url: "/pages/notes".into(),
        });
        driver.load_page(items);
        driver.shell.host.out.clear();
        driver
    }

    pub(super) fn sent(&self) -> &[Outbound] {
        &self.shell.host.out
    }

    /// Whether the core holds keys, as Emacs was told last.
    fn holding(&self) -> Option<bool> {
        self.sent().iter().rev().find_map(|message| match message {
            Outbound::State { holding, .. } => Some(*holding),
            _ => None,
        })
    }

    fn messages(&self) -> Vec<String> {
        self.sent()
            .iter()
            .filter_map(|message| match message {
                Outbound::Message { text } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// Starts inserting pic.png into the item at the cursor, and gives the id of its upload.
    fn insert_image(&mut self, id: &str, cursor: usize) -> u64 {
        self.send(Inbound::InsertImage {
            id: id.into(),
            path: "c:/pics/pic.png".into(),
            cursor,
        });
        let (upload, path) = self.shell.host.uploads.last().unwrap().clone();
        assert_eq!(path, "c:/pics/pic.png");
        upload
    }

    /// The rows Emacs has, when it was told of the rows since the last clear.
    fn last_rows(&self) -> Option<Vec<RowView>> {
        (self.rows_sent() > 0).then(|| self.shell.host.rows.clone())
    }

    /// The changes of the rows that Emacs was told last.
    fn last_changes(&self) -> Option<Vec<ListChange<RowView>>> {
        self.sent().iter().rev().find_map(|m| match m {
            Outbound::Rows { changes, .. } => Some(changes.clone()),
            _ => None,
        })
    }

    fn rows_sent(&self) -> usize {
        self.sent()
            .iter()
            .filter(|m| matches!(m, Outbound::Rows { .. }))
            .count()
    }

    fn page_paths(&self) -> Vec<Option<String>> {
        self.sent()
            .iter()
            .filter_map(|m| match m {
                Outbound::PagePath { path } => Some(path.clone()),
                _ => None,
            })
            .collect()
    }

    /// The rows and the free keys that Emacs was told last.
    fn last_properties(&self) -> Option<(Option<Vec<PropertyRow>>, Vec<FreeKey>)> {
        self.sent().iter().rev().find_map(|m| match m {
            Outbound::Properties {
                rows, free_keys, ..
            } => Some((rows.clone(), free_keys.clone())),
            _ => None,
        })
    }

    /// The keys that hold a date and a time, as Emacs was told last.
    fn last_datetime_keys(&self) -> Option<Vec<String>> {
        self.sent().iter().rev().find_map(|m| match m {
            Outbound::Properties { datetime_keys, .. } => Some(datetime_keys.clone()),
            _ => None,
        })
    }

    fn focuses(&self) -> Vec<(String, Option<usize>)> {
        self.sent()
            .iter()
            .filter_map(|m| match m {
                Outbound::Focus { id, cursor, .. } => Some((id.clone(), *cursor)),
                _ => None,
            })
            .collect()
    }
}

// 起動すると、サーバーのイベントにつなぎ、指定のURLのページを読みに行く。
#[test]
fn starting_connects_the_events_and_opens_the_url() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    assert_eq!(driver.shell.host.streams.len(), 1);
    assert!(
        driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "notes"))
    );
}

// ページ名で始めると、名前のとおりのページが開く。名前の # や空白は、URLの区切りにならない。
#[test]
fn starting_at_a_page_opens_the_page_of_that_name() {
    let mut driver = Driver::new();
    driver.send(Inbound::StartAtPage {
        path: "C# notes".into(),
    });
    assert!(driver
        .waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "C# notes")));
}

// /todayは、ホストの時計の今日のジャーナルを開く。
#[test]
fn today_opens_the_journal_of_the_hosts_date() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/today".into(),
    });
    assert!(driver.waiting(
        |op| matches!(op, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-11-06")
    ));
}

/// Opens the journal of today, which the server has with no items, and forgets the rest.
fn open_today() -> Driver {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/today".into(),
    });
    let mut page = page_info("", "2026-11-06");
    page["type"] = json!("journal");
    let journal = serde_json::from_value(json!({
        "Journal": { "page": page, "items": [], "created": false, "transcludedTrees": {} }
    }));
    driver.answer(
        |op| matches!(op, ApiOp::GetOrCreateJournal { .. }),
        journal.unwrap(),
    );
    driver.fail_all_waiting_calls();
    driver
}

// 時計を見る待ちが明けて、日付が変わっていれば、/todayは新しい日のジャーナルを開く。日付は、ホストの時計から入る。
#[test]
fn the_clock_watch_opens_the_journal_of_the_new_day() {
    let mut driver = open_today();
    driver.shell.host.day = Some(7);

    fire_timers(&mut driver);

    assert!(driver.waiting(
        |op| matches!(op, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-11-07")
    ));
}

// Emacsが「日付が変わったかもしれない」と知らせると、時計を見る待ちを待たずに、/todayは新しい日のジャーナルを開く。
#[test]
fn a_day_that_may_have_changed_opens_the_journal_of_the_new_day_at_once() {
    let mut driver = open_today();
    driver.shell.host.day = Some(7);

    driver.send(Inbound::DayMayHaveChanged);

    assert!(driver.waiting(
        |op| matches!(op, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-11-07")
    ));
}

/// An image the server saved under the name it came with.
fn uploaded(name: &str) -> Uploaded {
    Uploaded {
        file_name: name.into(),
        original_name: name.into(),
    }
}

// ページが読めると、Emacsに行が送られる。レベルと本文はサーバーのとおりである。
#[test]
fn a_loaded_page_reaches_emacs_as_rows() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    driver.load_page(&[item("a", 1, "親"), item("b", 2, "子")]);
    let rows = driver.last_rows().unwrap();
    let shown: Vec<_> = rows
        .iter()
        .map(|r| (r.id.as_str(), r.level, r.body.as_str(), r.bullet))
        .collect();
    assert_eq!(
        shown,
        [("a", 1, "親", Bullet::Plain), ("b", 2, "子", Bullet::Plain)]
    );
}

// 編集中でない空の項目は、行頭の点のない行としてEmacsに届く。ほかのシェルと同じである（#1042）。
#[test]
fn an_empty_item_that_is_not_edited_reaches_emacs_without_a_bullet() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    driver.load_page(&[item("a", 1, "親"), item("b", 1, "")]);
    let rows = driver.last_rows().unwrap();
    let bullets: Vec<_> = rows.iter().map(|r| r.bullet).collect();
    assert_eq!(bullets, [Bullet::Plain, Bullet::Hidden]);
}

// シェルは、行ごとに、本文のMarkdownと、Emacsが描く形（EmacsText）を送る。
#[test]
fn each_row_reaches_emacs_with_its_markdown_and_its_emacs_text() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    driver.load_page(&[item("a", 1, "**太字**")]);
    let rows = driver.last_rows().unwrap();

    assert_eq!(rows[0].body, "**太字**");
    assert_eq!(rows[0].text, EmacsText::of_markdown("**太字**", "tags/"));
    assert_eq!(rows[0].text.text, "太字\n");
}

// ページを開くと、全部の行が1つの入れ替えで送られる。
#[test]
fn an_opened_page_sends_all_its_rows_as_one_reset() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    driver.load_page(&[item("a", 1, "x"), item("b", 1, "y")]);
    let changes = driver.last_changes().unwrap();
    let [ListChange::Reset { items }] = &changes[..] else {
        panic!("{changes:?}");
    };
    let ids: Vec<&str> = items.iter().map(|row| row.id.as_str()).collect();
    assert_eq!(ids, ["a", "b"]);
}

// 1つの行が変わるキーでは、その行の更新だけが送られる。
#[test]
fn a_key_that_changes_one_row_sends_only_its_update() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y"), item("c", 1, "z")]);
    driver.send(Inbound::Indent { id: "b".into() });
    let changes = driver.last_changes().unwrap();
    assert!(
        matches!(&changes[..], [ListChange::Update { at: 1, item }] if item.id == "b"),
        "{changes:?}"
    );
}

// 入力した文はEmacsが持っているので、そのEmacsTextだけが送り返される。Emacsは、行を離れたときにそれで描く。
#[test]
fn typed_text_comes_back_only_as_its_emacs_text() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    let before = driver.rows_sent();
    driver.send(Inbound::BodyChanged {
        id: "b".into(),
        body: "**xy**".into(),
    });
    assert_eq!(driver.rows_sent(), before + 1);
    let changes = driver.last_changes().unwrap();
    assert!(
        matches!(&changes[..], [ListChange::Update { at: 1, item }] if item.id == "b"),
        "{changes:?}"
    );
    let rows = driver.last_rows().unwrap();
    assert_eq!(rows[1].body, "**xy**");
    assert_eq!(rows[1].text.text, "xy\n");
}

// 入力は、自動保存のタイマーを経て、一度だけ保存される。
#[test]
fn typed_text_is_saved_once_after_the_autosave_timer() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "xy".into(),
    });
    assert!(!driver.waiting(|op| matches!(op, ApiOp::UpdateItem { .. })));

    let (timer, _) = *driver.shell.host.timers.last().unwrap();
    driver.shell.handle(Input::TimerFired { id: timer });
    assert!(driver.waiting(|op| matches!(op, ApiOp::UpdateItem { body: Some(b), .. } if b == "xy")));
}

// 取り消されたタイマーが遅れて鳴っても、何も起きない。
#[test]
fn a_cancelled_timer_that_fires_late_does_nothing() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "xy".into(),
    });
    let (first, _) = *driver.shell.host.timers.last().unwrap();
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "xyz".into(),
    });
    driver.shell.handle(Input::TimerFired { id: first });
    assert!(!driver.waiting(|op| matches!(op, ApiOp::UpdateItem { .. })));
}

// カーソルの移動で編集が始まっても、カーソルを動かせという答えは返らない。
// 返すと、その間に先へ動いたカーソルが引き戻されるため。
#[test]
fn starting_an_edit_from_emacs_does_not_move_the_point_back() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::EditRequested {
        id: "b".into(),
        cursor: 1,
    });
    assert!(driver.focuses().is_empty());
}

// Emacsは、表示の文字の位置で編集を始められる。シェルは、その位置をMarkdownの位置に直して伝える（ADR 077）。
#[test]
fn an_edit_started_at_a_shown_letter_tells_emacs_its_place_in_the_markdown() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "**太字**の後")]);
    driver.send(Inbound::EditAtLetter {
        id: "b".into(),
        letter: 3,
    });
    assert_eq!(driver.focuses(), [("b".to_string(), Some(7))]);
}

/// What the server answers for a split: the item with the text it keeps, and the item made.
fn split_answer(item: Value, created: Value) -> ApiOutput {
    serde_json::from_value(json!({ "Split": { "item": item, "created": created } })).unwrap()
}

// 末尾でのEnterは、分割をサーバーへ送る。答えが届くと、新しい行を示し、カーソルをその行へ移す。
#[test]
fn enter_at_the_end_shows_a_new_row_and_moves_the_point_there() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::EditRequested {
        id: "a".into(),
        cursor: 1,
    });
    driver.send(Inbound::Enter {
        id: "a".into(),
        cursor: 1,
    });
    driver.answer(
        |op| matches!(op, ApiOp::SplitItem { body, cursor: 1, .. } if body == "x"),
        split_answer(item("a", 1, "x"), item("b", 1, "")),
    );
    let rows = driver.last_rows().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, "a");
    assert_eq!((rows[1].id.as_str(), rows[1].body.as_str()), ("b", ""));
    assert_eq!(driver.focuses().last().unwrap().0, "b");
}

// 途中でのEnterの間はキーを止め、新しい項目へカーソルが移ったら、止めるのをやめたと伝える。
#[test]
fn the_end_of_holding_keys_reaches_emacs() {
    let mut driver = Driver::open(&[item("a", 1, "abcd")]);
    driver.send(Inbound::Enter {
        id: "a".into(),
        cursor: 2,
    });
    assert_eq!(driver.holding(), Some(true));
    driver.answer(
        |op| matches!(op, ApiOp::SplitItem { .. }),
        split_answer(item("a", 1, "ab"), item("b", 1, "cd")),
    );
    assert_eq!(driver.focuses().last().unwrap().0, "b");
    assert_eq!(driver.holding(), Some(false));
}

// 閉じていないコードフェンスの中のEnterは、項目を分けずに改行を入れ、カーソルを次の行へ進める。
#[test]
fn enter_inside_an_open_code_fence_adds_a_line_instead_of_a_row() {
    let mut driver = Driver::open(&[item("a", 1, "```\ncode")]);
    driver.send(Inbound::Enter {
        id: "a".into(),
        cursor: 8,
    });
    let rows = driver.last_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].body, "```\ncode\n");
    assert_eq!(driver.focuses(), [("a".to_string(), Some(9))]);
    assert!(!driver.waiting(|op| matches!(op, ApiOp::CreateItem { .. })));
}

// 閉じたフェンスの後のEnterは、ふつうに項目を分ける。
#[test]
fn enter_after_a_closed_code_fence_splits_the_item() {
    let mut driver = Driver::open(&[item("a", 1, "```\ncode\n```")]);
    driver.send(Inbound::Enter {
        id: "a".into(),
        cursor: 12,
    });
    assert!(driver.waiting(|op| matches!(op, ApiOp::SplitItem { cursor: 12, .. })));
}

// `[[`を打ったと伝えると、`]]`を足した本文が返り、カーソルはその間に留まる。
#[test]
fn typing_a_wiki_link_opening_closes_it_and_keeps_the_cursor_inside() {
    let mut driver = Driver::open(&[item("a", 1, "")]);
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "see [[".into(),
    });
    driver.send(Inbound::Typed {
        id: "a".into(),
        cursor: 6,
    });
    assert_eq!(driver.last_rows().unwrap()[0].body, "see [[]]");
    assert_eq!(
        driver.focuses().last().unwrap(),
        &("a".to_string(), Some(6))
    );
}

// リンクの中で`]`を打ったと伝えると、増えた`]`のない本文が返り、カーソルは進んだ位置に留まる。
#[test]
fn typing_over_the_closing_of_a_wiki_link_sends_the_body_back() {
    let mut driver = Driver::open(&[item("a", 1, "")]);
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "[[x]]]".into(),
    });
    driver.send(Inbound::Typed {
        id: "a".into(),
        cursor: 4,
    });
    assert_eq!(driver.last_rows().unwrap()[0].body, "[[x]]");
    assert_eq!(
        driver.focuses().last().unwrap(),
        &("a".to_string(), Some(4))
    );
}

// 補うものがなければ、何も送り返さない。
#[test]
fn typing_with_nothing_to_close_changes_nothing() {
    let mut driver = Driver::open(&[item("a", 1, "x[")]);
    driver.send(Inbound::Typed {
        id: "a".into(),
        cursor: 2,
    });
    assert!(driver.last_rows().is_none());
    assert!(driver.focuses().is_empty());
}

// インデントは、coreの判断のとおりに行のレベルを変えて送り返す。
#[test]
fn indent_comes_back_as_a_row_with_a_new_level() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::Indent { id: "b".into() });
    assert_eq!(driver.last_rows().unwrap()[1].level, 2);
}

// 先頭の項目はインデントできず、行は送られない。
#[test]
fn the_first_item_cannot_be_indented() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::Indent { id: "a".into() });
    assert_eq!(driver.rows_sent(), 0);
}

// 貼り付けは、Emacsが持つ文を、coreの貼り付けとして渡す。
#[test]
fn a_paste_goes_through_the_core() {
    let mut driver = Driver::open(&[item("a", 1, "xy")]);
    driver.send(Inbound::Paste {
        id: "a".into(),
        text: "Z".into(),
        cursor: 1,
    });
    let pasted = serde_json::from_value(json!({ "Pasted": {
        "items": [item("a", 1, "xZy")],
        "cursor": { "itemId": "a", "position": 2 },
    } }));
    driver.answer(
        |op| matches!(op, ApiOp::PasteText { text, cursor: 1, .. } if text == "Z"),
        pasted.unwrap(),
    );
    assert_eq!(driver.last_rows().unwrap()[0].body, "xZy");
}

/// What the server answers to the paste of the link of an image into "a" at the cursor 1
fn link_pasted(body: &str, position: usize) -> ApiOutput {
    serde_json::from_value(json!({ "Pasted": {
        "items": [item("a", 1, body)],
        "cursor": { "itemId": "a", "position": position },
    } }))
    .unwrap()
}

// 画像の挿入は、ファイルをアップロードし、リンクをカーソル位置への貼り付けとして送り、カーソルをリンクの後ろへ移す。
#[test]
fn an_inserted_image_is_uploaded_and_linked_at_the_cursor() {
    let link = "![pic.png](../assets/pic.png)";
    let mut driver = Driver::open(&[item("a", 1, "xy")]);
    let upload = driver.insert_image("a", 1);

    driver.shell.handle(Input::ApiDone {
        id: upload,
        result: Box::new(Ok(ApiOutput::Uploaded(uploaded("pic.png")))),
    });

    let body = format!("x{link}y");
    let behind_the_link = 1 + link.chars().count();
    driver.answer(
        |op| matches!(op, ApiOp::PasteText { text, cursor: 1, .. } if text == link),
        link_pasted(&body, behind_the_link),
    );
    assert_eq!(driver.last_rows().unwrap()[0].body, body);
    assert!(driver
        .focuses()
        .contains(&("a".to_string(), Some(behind_the_link))));
    assert_eq!(driver.messages(), Vec::<String>::new());
}

// 挿入の間はキーを止める。リンクは挿入を始めたときのカーソル位置に入るので、その間に打つと位置がずれるため。
#[test]
fn keys_are_held_until_the_link_of_the_inserted_image_is_pasted() {
    let mut driver = Driver::open(&[item("a", 1, "xy")]);
    let upload = driver.insert_image("a", 1);
    assert_eq!(driver.holding(), Some(true));

    driver.shell.handle(Input::ApiDone {
        id: upload,
        result: Box::new(Ok(ApiOutput::Uploaded(uploaded("pic.png")))),
    });
    assert_eq!(driver.holding(), Some(true));

    driver.answer(
        |op| matches!(op, ApiOp::PasteText { .. }),
        link_pasted("x![pic.png](../assets/pic.png)y", 30),
    );
    assert_eq!(driver.holding(), Some(false));
}

// アップロードに失敗すると、その理由を伝え、本文は変わらず、キーも止めない。
#[test]
fn a_failed_upload_is_reported_and_the_body_stays() {
    let mut driver = Driver::open(&[item("a", 1, "xy")]);
    let upload = driver.insert_image("a", 1);

    driver.shell.handle(Input::ApiDone {
        id: upload,
        result: Box::new(Err(ApiError("Upload failed: 413".into()))),
    });

    assert_eq!(
        driver.messages(),
        ["Could not insert the image: Upload failed: 413"]
    );
    assert_eq!(driver.rows_sent(), 0);
    assert_eq!(driver.holding(), Some(false));
}

// アップロードのあとでリンクの貼り付けに失敗しても、そのことを伝える。
#[test]
fn a_failed_paste_of_the_link_is_reported() {
    let mut driver = Driver::open(&[item("a", 1, "xy")]);
    let upload = driver.insert_image("a", 1);
    driver.shell.handle(Input::ApiDone {
        id: upload,
        result: Box::new(Ok(ApiOutput::Uploaded(uploaded("pic.png")))),
    });

    let (save, _) = driver
        .take_api(|op| matches!(op, ApiOp::PasteText { .. }))
        .expect("the link was not pasted");
    driver.shell.handle(Input::ApiDone {
        id: save,
        result: Box::new(Err(ApiError("server down".into()))),
    });

    assert_eq!(
        driver.messages(),
        ["Could not insert the image: server down"]
    );
    assert_eq!(driver.holding(), Some(false));
}

// 挿入の間でも、関係のない呼び出しの失敗は、画像の失敗として伝えず、キーを止める理由にもしない。
#[test]
fn a_failure_of_another_call_is_not_reported_as_one_of_the_image() {
    let mut driver = Driver::open(&[item("a", 1, "xy")]);
    driver.send(Inbound::DeleteItem { id: "a".into() });
    let upload = driver.insert_image("a", 1);
    driver.fail_all_waiting_calls();
    // The failed deletion is told as itself, with the reason of the server
    assert_eq!(driver.messages(), ["not in this test"]);
    assert_eq!(driver.holding(), Some(true));

    driver.shell.handle(Input::ApiDone {
        id: upload,
        result: Box::new(Err(ApiError("Upload failed: 500".into()))),
    });
    assert_eq!(driver.holding(), Some(false));
}

impl Driver {
    /// Which rows Emacs was told last are selected.
    fn selected(&self) -> Vec<bool> {
        self.last_rows()
            .unwrap()
            .iter()
            .map(|row| row.selected)
            .collect()
    }

    fn key(&mut self, key: SelectionKey) {
        self.send(Inbound::SelectionKey { key });
    }
}

// 選択を始めると、その項目が子ごと選ばれる。
#[test]
fn a_selection_starts_with_the_item_and_its_children() {
    let mut driver = Driver::open(&[item("a", 1, "親"), item("b", 2, "子"), item("c", 1, "他")]);
    driver.send(Inbound::Select { id: "a".into() });
    assert_eq!(driver.selected(), [true, true, false]);
}

// 選択は、下へ伸ばせる。Escapeで、選択は終わる。
#[test]
fn a_selection_grows_down_and_ends_on_escape() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y"), item("c", 1, "z")]);
    driver.send(Inbound::Select { id: "a".into() });
    driver.key(SelectionKey::ExtendDown);
    assert_eq!(driver.selected(), [true, true, false]);
    driver.key(SelectionKey::Escape);
    assert_eq!(driver.selected(), [false, false, false]);
}

// 選んだ項目は、まとめて削除される。
#[test]
fn the_selected_items_are_deleted_together() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y"), item("c", 1, "z")]);
    driver.send(Inbound::Select { id: "a".into() });
    driver.key(SelectionKey::ExtendDown);
    driver.key(SelectionKey::Delete);
    driver.answer(
        |op| matches!(op, ApiOp::TranscludedItems { .. }),
        ApiOutput::ItemIds(Vec::new()),
    );
    let (_, deleted) = driver
        .take_api(|op| matches!(op, ApiOp::DeleteItems { .. }))
        .expect("the deletion was not sent");
    let ApiOp::DeleteItems { mut item_ids } = deleted else {
        unreachable!();
    };
    item_ids.sort();
    assert_eq!(item_ids, ["a", "b"]);
}

// 選んだ項目は、まとめて動かせる。
#[test]
fn the_selected_items_move_together() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y"), item("c", 1, "z")]);
    driver.send(Inbound::Select { id: "a".into() });
    driver.key(SelectionKey::ExtendDown);
    driver.key(SelectionKey::MoveDown);
    assert!(driver.waiting(|op| matches!(op, ApiOp::MoveItems { .. })));
}

// 選んだ項目をコピーすると、そのMarkdownがEmacsに届く。
#[test]
fn copying_the_selection_gives_emacs_its_markdown() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::Select { id: "a".into() });
    driver.key(SelectionKey::Copy);
    driver.answer(
        |op| matches!(op, ApiOp::ExportTreeMarkdown { .. }),
        ApiOutput::TreeExport(TreeExport {
            tree_markdown: "- x
"
            .into(),
            flat_markdown: "x
"
            .into(),
        }),
    );
    assert!(driver.sent().contains(&Outbound::Copy {
        text: "- x
"
        .into()
    }));
}

// 項目のidを貼り付けると、その項目がトランスクルージョンの元としてサーバーへ送られる。
#[test]
fn pasting_an_item_id_makes_a_transclusion_of_it() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "")]);
    driver.send(Inbound::PasteAsTransclusion {
        id: "b".into(),
        text: " abcdefghij
"
        .into(),
    });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::SetTransclusion { item_id, source_item_id }
            if item_id == "b" && source_item_id == "abcdefghij"
    )));
}

// 項目のidに見えない文字を貼り付けても、何も送らない。
#[test]
fn pasting_other_text_makes_no_transclusion() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::PasteAsTransclusion {
        id: "a".into(),
        text: "hello world".into(),
    });
    assert!(!driver.waiting(|op| matches!(op, ApiOp::SetTransclusion { .. })));
}

// 解除すると、その項目がサーバーへ送られ、ふつうの項目に戻る。
#[test]
fn removing_a_transclusion_sends_the_item() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::RemoveTransclusion { id: "a".into() });
    assert!(
        driver.waiting(|op| matches!(op, ApiOp::RemoveTransclusion { item_id } if item_id == "a"))
    );
}

// Stashへ送ると、まだ保存していない打った文字も一緒にサーバーへ送られる。
#[test]
fn a_stash_carries_the_text_typed_into_the_item() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::EditRequested {
        id: "b".into(),
        cursor: 1,
    });
    driver.send(Inbound::BodyChanged {
        id: "b".into(),
        body: "yz".into(),
    });
    driver.send(Inbound::SendToStash { id: "b".into() });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::StashItem { item_id, body } if item_id == "b" && body.as_deref() == Some("yz")
    )));
}

// 項目の削除は、参照の確認を経て、子ごとサーバーへ送られる。行は、サーバーの答えのあとで消える。
#[test]
fn deleting_an_item_takes_its_children_with_it() {
    let mut driver = Driver::open(&[item("a", 1, "親"), item("b", 2, "子"), item("c", 1, "他")]);
    driver.send(Inbound::DeleteItem { id: "a".into() });
    driver.answer(
        |op| matches!(op, ApiOp::TranscludedItems { .. }),
        ApiOutput::ItemIds(Vec::new()),
    );
    let deleted = driver
        .take_api(|op| matches!(op, ApiOp::DeleteItems { .. }))
        .expect("the deletion was not sent");
    driver.shell.handle(Input::ApiDone {
        id: deleted.0,
        result: Box::new(Ok(ApiOutput::Items(Vec::new()))),
    });
    let ApiOp::DeleteItems { item_ids } = deleted.1 else {
        unreachable!();
    };
    assert_eq!(item_ids, ["a", "b"]);
    let rows = driver.last_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, "c");
}

// 他から参照される項目の削除を断ると、何も消えず、選択も残らない。
#[test]
fn refusing_to_delete_a_transcluded_item_leaves_everything_as_it_was() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::DeleteItem { id: "a".into() });
    driver.answer(
        |op| matches!(op, ApiOp::TranscludedItems { .. }),
        ApiOutput::ItemIds(vec!["a".into()]),
    );
    let req = driver
        .sent()
        .iter()
        .find_map(|m| match m {
            Outbound::Ask { req, .. } => Some(*req),
            _ => None,
        })
        .expect("the user was not asked");

    driver.send(Inbound::Answer { req, yes: false });

    assert!(!driver.waiting(|op| matches!(op, ApiOp::DeleteItems { .. })));
    assert!(!driver.shell.core.view().outline.selection_mode);
}

// サーバーが項目の削除を断ると、何も消えず、選択も残らない。
#[test]
fn a_deletion_the_server_refuses_leaves_everything_as_it_was() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::DeleteItem { id: "a".into() });
    driver.answer(
        |op| matches!(op, ApiOp::TranscludedItems { .. }),
        ApiOutput::ItemIds(Vec::new()),
    );
    let deleted = driver
        .take_api(|op| matches!(op, ApiOp::DeleteItems { .. }))
        .expect("the deletion was not sent");

    driver.shell.handle(Input::ApiDone {
        id: deleted.0,
        result: Box::new(Err(ApiError("refused".into()))),
    });

    assert_eq!(driver.shell.core.view().outline.rows.len(), 2);
    assert!(!driver.shell.core.view().outline.selection_mode);
}

// 移動の通知は、そのURLのページを開く。Backは一つ前のページへ戻る。
#[test]
fn navigation_opens_pages_and_back_returns_to_the_previous_one() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::GoToPage {
        path: "other".into(),
    });
    assert!(
        driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "other"))
    );

    driver.send(Inbound::Back);
    driver.find_page();
    assert!(
        driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "notes"))
    );
}

// 戻ったあとのForwardは、戻る前のページをもう一度開く（ADR 035）。
#[test]
fn forward_after_back_opens_the_page_left_again() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::GoToPage {
        path: "other".into(),
    });
    driver.send(Inbound::Back);
    driver.find_page();
    driver.take_api(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "other"));

    driver.send(Inbound::Forward);
    driver.find_page();

    assert!(
        driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "other"))
    );
}

// カーソルの下のタグは、そのページを開く。
#[test]
fn the_tag_at_the_cursor_opens_its_page() {
    let mut driver = Driver::open(&[item("a", 1, "about #rust")]);
    driver.send(Inbound::OpenLinkAt {
        id: "a".into(),
        cursor: 8,
    });
    assert!(
        driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "rust"))
    );
}

// カーソルの下にリンクがなければ、そのことをEmacsに伝える。
#[test]
fn emacs_is_told_when_the_cursor_is_not_on_a_link() {
    let mut driver = Driver::open(&[item("a", 1, "plain")]);
    driver.send(Inbound::OpenLinkAt {
        id: "a".into(),
        cursor: 2,
    });
    assert!(driver
        .sent()
        .iter()
        .any(|m| matches!(m, Outbound::Message { text } if text == "No link here")));
}

// 処理した行の番号がEmacsに伝わる。Emacsはこれを待って次のキーを処理する。
#[test]
fn every_line_is_acknowledged() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    let seq = driver.seq + 1;
    driver.send(Inbound::Undo);
    assert!(driver
        .sent()
        .iter()
        .any(|m| matches!(m, Outbound::State { ack, .. } if *ack == seq)));
}

// イベントのストリームは、古い接続の知らせを無視する。
#[test]
fn events_of_a_replaced_stream_are_ignored() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    let stream = driver.shell.host.streams[0];
    driver.shell.handle(Input::Sse {
        stream: stream + 100,
        event: SseEvent::Open,
    });
    assert!(!driver
        .sent()
        .iter()
        .any(|m| matches!(m, Outbound::Status { connection } if connection == "connected")));

    driver.shell.handle(Input::Sse {
        stream,
        event: SseEvent::Open,
    });
    assert!(driver
        .sent()
        .iter()
        .any(|m| matches!(m, Outbound::Status { connection } if connection == "connected")));
}

// 別のクライアントの変更の知らせで、開いているページが読み直される。
#[test]
fn a_change_elsewhere_reloads_the_shown_page() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    let stream = driver.shell.host.streams[0];
    driver.shell.handle(Input::Sse {
        stream,
        event: SseEvent::Message(
            r#"{"type":"pagesUpdated","pageId":"p","source":"file"}"#.to_string(),
        ),
    });
    // The core debounces the reload with a timer
    let timers: Vec<u64> = driver.shell.host.timers.iter().map(|t| t.0).collect();
    for id in timers {
        driver.shell.handle(Input::TimerFired { id });
    }
    driver.answer(
        |op| matches!(op, ApiOp::GetPageById { page_id } if page_id == "p"),
        page_answer(&[item("a", 1, "x"), item("b", 1, "他で追加")]),
    );
    assert_eq!(driver.last_rows().unwrap()[1].body, "他で追加");
}

// シェルは自分のIDをコアに渡すので、自分の変更の知らせでは、ページは読み直されない。
#[test]
fn a_change_by_this_shell_does_not_reload_the_shown_page() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    let me = driver.shell.client_id().clone();

    driver.changed_by(&me);

    assert!(!driver.waiting(|op| matches!(op, ApiOp::GetPageById { .. })));
}

// 検索の結果は、ページ、作成、項目の順に通し番号が付く。番号を送り返すと、その結果を開く。
#[test]
fn search_results_are_numbered_and_open_by_their_number() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::Search { query: "ru".into() });
    let timers: Vec<u64> = driver.shell.host.timers.iter().map(|t| t.0).collect();
    for id in timers {
        driver.shell.handle(Input::TimerFired { id });
    }
    let results = serde_json::from_value(json!({
        "SearchResults": [
            { "pageId": "p1", "pageTitle": "rust", "itemId": "", "snippet": "", "score": 0.0 },
            { "pageId": "p2", "pageTitle": "memo", "itemId": "i1", "snippet": "rust本", "score": 0.0 },
        ]
    }))
    .unwrap();
    driver.answer(|op| matches!(op, ApiOp::SearchCombined { .. }), results);

    let entries = driver
        .sent()
        .iter()
        .rev()
        .find_map(|m| match m {
            Outbound::Search { results, .. } => Some(results.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(entries.first().unwrap().title, "rust");
    assert_eq!(entries.first().unwrap().position, 0);
    let last = entries.last().unwrap();
    assert_eq!(last.snippet, "rust本");

    driver.send(Inbound::SearchResult {
        position: last.position,
    });
    assert!(
        driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "memo"))
    );
}

/// Whether the last word to Emacs about the search says that it is still running.
fn searching(driver: &Driver) -> bool {
    driver
        .sent()
        .iter()
        .rev()
        .find_map(|m| match m {
            Outbound::Search { searching, .. } => Some(*searching),
            _ => None,
        })
        .unwrap()
}

fn fire_timers(driver: &mut Driver) {
    let timers: Vec<u64> = driver.shell.host.timers.iter().map(|t| t.0).collect();
    for id in timers {
        driver.shell.handle(Input::TimerFired { id });
    }
}

fn no_results() -> ApiOutput {
    serde_json::from_value(json!({ "SearchResults": [] })).unwrap()
}

// 検索語を受けてから答えが届くまでは、検索中と伝える。古い結果を開かないため。
#[test]
fn a_search_is_running_until_its_answer_arrives() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::Search { query: "ru".into() });
    assert!(searching(&driver));
    fire_timers(&mut driver);
    assert!(searching(&driver));
    driver.answer(
        |op| matches!(op, ApiOp::SearchCombined { .. }),
        no_results(),
    );
    assert!(!searching(&driver));
}

// 前の検索語の答えが届いても、今の検索語の答えが届くまでは検索中のままである。
#[test]
fn the_answer_to_an_older_query_does_not_end_the_search() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::Search { query: "r".into() });
    fire_timers(&mut driver);
    driver.send(Inbound::Search { query: "ru".into() });
    driver.answer(
        |op| matches!(op, ApiOp::SearchCombined { query, .. } if query == "r"),
        no_results(),
    );
    assert!(searching(&driver));
    fire_timers(&mut driver);
    driver.answer(
        |op| matches!(op, ApiOp::SearchCombined { query, .. } if query == "ru"),
        no_results(),
    );
    assert!(!searching(&driver));
}

// 検索が失敗しても検索中は終わる。画面は描き直されないが、Emacsは終わりを待っている。
#[test]
fn a_failed_search_ends_the_search() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.fail_all_waiting_calls();
    driver.send(Inbound::Search { query: "ru".into() });
    fire_timers(&mut driver);
    driver.fail_all_waiting_calls();
    assert!(!searching(&driver));
}

// 同じ検索語がまた届いても、検索中のまま止まらない。
#[test]
fn the_same_query_again_does_not_stay_searching() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::Search { query: "ru".into() });
    fire_timers(&mut driver);
    driver.answer(
        |op| matches!(op, ApiOp::SearchCombined { .. }),
        no_results(),
    );
    driver.send(Inbound::Search { query: "ru".into() });
    fire_timers(&mut driver);
    driver.fail_all_waiting_calls();
    assert!(!searching(&driver));
}

// 空の検索語は検索しないので、検索中にならない。
#[test]
fn an_empty_query_is_not_a_running_search() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::Search { query: "ru".into() });
    driver.send(Inbound::Search { query: " ".into() });
    assert!(!searching(&driver));
}

// フォルダを含むパスで名前を変えると、サーバーへ新しい名前とフォルダが送られる。
#[test]
fn renaming_sends_the_new_name_and_folder_to_the_server() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::RenamePage {
        path: "work/memo".into(),
    });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::UpdatePage { page_id, title, dir_path, overwrite: false }
            if page_id == "p"
                && title.as_deref() == Some("memo")
                && dir_path.as_deref() == Some("work/")
    )));
}

// 読み込んだdocページのパスを、Emacsに伝える。Emacsはこれを名前の変更の初期値にする。
#[test]
fn a_loaded_doc_page_tells_emacs_its_path() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    driver.load_page(&[]);
    assert_eq!(driver.page_paths().last(), Some(&Some("notes".to_string())));
}

// indexとアトラスのページには、変えられるパスがない。名前で役割が決まるページだからである。
#[test]
fn an_index_or_atlas_page_has_no_path_to_rename() {
    for name in ["index", "index_atlas"] {
        let mut driver = Driver::new();
        driver.send(Inbound::Start {
            url: format!("/pages/a/{name}"),
        });
        let answer = serde_json::from_value(json!({
            "DocPage": { "page": page_info("a/", name), "items": [], "created": false }
        }))
        .unwrap();
        driver.answer(|op| matches!(op, ApiOp::GetOrCreateDoc { .. }), answer);
        assert!(driver.page_paths().iter().all(Option::is_none), "{name}");
    }
}

impl Driver {
    /// Opens the index page of the folder `a/b/`, and answers whether pages are under it.
    fn open_index(children: Vec<Value>) -> Driver {
        let mut driver = Driver::new();
        driver.send(Inbound::Start {
            url: "/pages/a/b/index".into(),
        });
        let answer = serde_json::from_value(json!({
            "DocPage": { "page": page_info("a/b/", "index"), "items": [], "created": false }
        }))
        .unwrap();
        driver.answer(|op| matches!(op, ApiOp::GetOrCreateDoc { .. }), answer);
        driver.answer_child_pages(&children);
        driver
    }

    /// Answers every call that asks for the pages under a page.
    fn answer_child_pages(&mut self, children: &[Value]) {
        while self.waiting(|op| matches!(op, ApiOp::GetChildPages { .. })) {
            let pages = serde_json::from_value(json!({ "Pages": children })).unwrap();
            self.answer(|op| matches!(op, ApiOp::GetChildPages { .. }), pages);
        }
    }

    /// What Emacs was told last of the index page on screen.
    fn index(&self) -> Option<(Option<String>, Option<String>)> {
        self.sent().iter().rev().find_map(|m| match m {
            Outbound::Index {
                folder,
                demote_name,
            } => Some((folder.clone(), demote_name.clone())),
            _ => None,
        })
    }
}

// indexページを開くと、そのフォルダのパスがEmacsに届く。下にページがなければ、降格して付く名前も届く。
#[test]
fn an_index_page_tells_emacs_its_folder_and_the_name_it_can_take() {
    let driver = Driver::open_index(vec![]);
    assert_eq!(
        driver.index(),
        Some((Some("a/b".to_string()), Some("b".to_string())))
    );
}

// docページは、indexページではないので、フォルダのパスは届かない。
#[test]
fn a_doc_page_tells_emacs_no_folder() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    driver.load_page(&[]);
    assert!(matches!(driver.index(), None | Some((None, None))));
}

// 下にページがあるindexページは、降格できない。
#[test]
fn an_index_page_with_pages_under_it_cannot_be_demoted() {
    let driver = Driver::open_index(vec![page_info("a/b/", "memo")]);
    assert_eq!(driver.index(), Some((Some("a/b".to_string()), None)));
}

// indexページから離れると、フォルダのパスはなくなる。
#[test]
fn leaving_an_index_page_takes_its_folder_away() {
    let mut driver = Driver::open_index(vec![]);
    driver.send(Inbound::Go {
        url: "/today".into(),
    });
    assert_eq!(driver.index(), Some((None, None)));
}

// フォルダ名を変えると、新しいフォルダがサーバーへ送られる。ページの名前はindexのままである。
#[test]
fn renaming_the_folder_sends_the_new_folder_to_the_server() {
    let mut driver = Driver::open_index(vec![]);
    driver.send(Inbound::RenameFolder { path: "a/c".into() });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::UpdatePage { page_id, title: None, dir_path, .. }
            if page_id == "p" && dir_path.as_deref() == Some("a/c/")
    )));
}

// 降格すると、indexページはフォルダの名前のページになる。
#[test]
fn demoting_sends_the_page_to_the_server() {
    let mut driver = Driver::open_index(vec![]);
    driver.send(Inbound::DemoteIndex);
    driver.answer_child_pages(&[]);
    assert!(driver.waiting(|op| matches!(op, ApiOp::DemotePage { page_id, .. } if page_id == "p")));
}

// ジャーナルへ移ると、変えられるパスはなくなる。
#[test]
fn moving_to_a_journal_takes_the_path_away() {
    let mut driver = Driver::open(&[]);
    driver.send(Inbound::Go {
        url: "/today".into(),
    });
    assert_eq!(driver.page_paths(), [None]);
}

// 名前を変えると、新しいパスと題名がEmacsに届き、項目は読み直されない。
#[test]
fn a_renamed_page_tells_emacs_the_new_path_and_title() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.fail_all_waiting_calls();
    driver.send(Inbound::RenamePage {
        path: "work/memo".into(),
    });
    let answer = serde_json::from_value(json!({ "PageInfo": page_info("work/", "memo") })).unwrap();
    driver.answer(|op| matches!(op, ApiOp::UpdatePage { .. }), answer);

    assert_eq!(driver.page_paths(), [Some("work/memo".to_string())]);
    assert!(driver
        .sent()
        .iter()
        .any(|m| matches!(m, Outbound::Title { title } if title == "work/memo")));
    assert!(!driver.waiting(|op| matches!(op, ApiOp::GetPageById { .. })));
}

// 同じ名前のページがあれば、上書きしてよいかを尋ねる。「はい」なら上書きして変える。
#[test]
fn renaming_over_another_page_asks_and_overwrites_on_yes() {
    let mut driver = Driver::open(&[]);
    driver.fail_all_waiting_calls();
    driver.send(Inbound::RenamePage {
        path: "taken".into(),
    });
    driver.answer(
        |op| matches!(op, ApiOp::UpdatePage { .. }),
        ApiOutput::PageNameTaken("taken".into()),
    );

    let req = driver
        .sent()
        .iter()
        .find_map(|m| match m {
            Outbound::Ask { req, .. } => Some(*req),
            _ => None,
        })
        .expect("the shell asks before it overwrites");
    driver.send(Inbound::Answer { req, yes: true });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::UpdatePage {
            overwrite: true,
            ..
        }
    )));
}

/// The question that the shell sent last.
fn last_ask(driver: &Driver) -> Option<(u64, String)> {
    driver.sent().iter().rev().find_map(|m| match m {
        Outbound::Ask { req, message } => Some((*req, message.clone())),
        _ => None,
    })
}

// 今のページを消すときは、ページの名前を挙げて尋ね、「はい」ならサーバーで消す。
#[test]
fn deleting_the_page_asks_and_deletes_it_on_yes() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::DeletePage);
    let (req, message) = last_ask(&driver).expect("the shell asks before it deletes");
    assert!(message.contains("notes"), "{message}");
    driver.send(Inbound::Answer { req, yes: true });
    assert!(driver.waiting(|op| matches!(op, ApiOp::DeletePage { page_id } if page_id == "p")));
}

// 「いいえ」と答えると、ページは消えない。
#[test]
fn deleting_the_page_does_nothing_on_no() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::DeletePage);
    let (req, _) = last_ask(&driver).expect("the shell asks before it deletes");
    driver.send(Inbound::Answer { req, yes: false });
    assert!(!driver.waiting(|op| matches!(op, ApiOp::DeletePage { .. })));
}

/// The numbers of the questions that the shell sent, oldest first.
fn asks(driver: &Driver) -> Vec<u64> {
    let ask = |m: &Outbound| match m {
        Outbound::Ask { req, .. } => Some(*req),
        _ => None,
    };
    driver.sent().iter().filter_map(ask).collect()
}

// 質問は1つずつEmacsに届く。2つ目は、1つ目に答えたあとに届く。ミニバッファの質問は、次の質問が来ると置き換わるためである。
#[test]
fn a_second_question_reaches_emacs_after_the_first_is_answered() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::DeletePage);
    driver.send(Inbound::DeletePage);
    let first = asks(&driver);
    assert_eq!(first.len(), 1, "the second question waits");

    driver.send(Inbound::Answer {
        req: first[0],
        yes: false,
    });
    let both = asks(&driver);
    assert_eq!(both.len(), 2, "the second question follows the answer");
    assert!(!driver.waiting(|op| matches!(op, ApiOp::DeletePage { .. })));

    driver.send(Inbound::Answer {
        req: both[1],
        yes: true,
    });
    assert_eq!(asks(&driver).len(), 2);
    assert!(driver.waiting(|op| matches!(op, ApiOp::DeletePage { page_id } if page_id == "p")));
}

fn definitions_answer() -> ApiOutput {
    let definition = |key: &str, value_type: &str, page: bool| PropertyDefinition {
        key: key.into(),
        value_type: value_type.into(),
        system: false,
        page,
    };
    ApiOutput::PropertyDefinitions(vec![
        definition("rating", "number", true),
        definition("topic_list", "list", true),
        definition("plan", "date", false),
        definition("published", "datetime", true),
        definition("met", "datetime", true),
        definition("seen", "datetime", false),
        definition("score_n", "number", false),
    ])
}

fn answer_definitions(driver: &mut Driver) {
    driver.answer(
        |op| matches!(op, ApiOp::GetPropertyDefinitions),
        definitions_answer(),
    );
}

// プロパティを設定すると、定義の型とともにサーバーへ送られる。
#[test]
fn setting_a_property_sends_the_type_of_its_definition() {
    let mut driver = Driver::open(&[]);
    answer_definitions(&mut driver);
    driver.send(Inbound::SetProperty {
        key: "rating".into(),
        value: "5".into(),
    });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::UpsertPageProperty { page_id, key, value_type, value }
            if page_id == "p" && key == "rating" && value_type == "number" && value == "5"
    )));
}

/// Opens the page `notes` with these properties and these definitions.
fn open_with_properties(properties: Value) -> Driver {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes".into(),
    });
    let mut page = page_info("", "notes");
    page["properties"] = properties;
    let answer: ApiOutput = serde_json::from_value(json!({
        "DocPage": { "page": page, "items": [], "created": false }
    }))
    .unwrap();
    driver.answer(|op| matches!(op, ApiOp::GetOrCreateDoc { .. }), answer);
    answer_definitions(&mut driver);
    driver
}

// 今の値がリストのプロパティは、定義がなくても、名前が_listで終わるキーのリストとして送られる。
#[test]
fn a_property_that_holds_a_list_is_sent_as_a_list() {
    let mut driver = open_with_properties(json!({ "label_list": ["a"] }));
    driver.send(Inbound::SetProperty {
        key: "label_list".into(),
        value: "a, b".into(),
    });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::UpsertPageProperty { key, value_type, .. } if key == "label_list" && value_type == "list"
    )));
}

// 定義のないプロパティは、文として送られる。
#[test]
fn a_property_without_a_definition_is_sent_as_text() {
    let mut driver = open_with_properties(json!({ "old": "x" }));
    driver.send(Inbound::SetProperty {
        key: "old".into(),
        value: "y".into(),
    });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::UpsertPageProperty { key, value_type, .. } if key == "old" && value_type == "text"
    )));
}

// プロパティを削除すると、そのキーがサーバーへ送られる。
#[test]
fn deleting_a_property_sends_its_key() {
    let mut driver = open_with_properties(json!({ "old": "x" }));
    driver.send(Inbound::DeleteProperty { key: "old".into() });
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::DeletePageProperty { page_id, key } if page_id == "p" && key == "old"
    )));
}

// 開いたページのプロパティと、まだ付けられるキーをEmacsに伝える。ページに使えない定義と、使用中のキーは除く。
#[test]
fn a_doc_page_tells_emacs_its_properties_and_free_keys() {
    let driver = open_with_properties(json!({ "rating": "5", "label_list": ["a", "b"] }));
    let (rows, free_keys) = driver.last_properties().unwrap();
    let rows = rows.expect("a doc page has property rows");
    let labels = rows.iter().find(|r| r.key == "label_list").unwrap();
    assert_eq!(labels.value, "a, b");
    assert!(labels.removable && labels.editable);
    let modified = rows.iter().find(|r| r.key == "modified").unwrap();
    assert!(!modified.removable && !modified.editable);
    let created = rows.iter().find(|r| r.key == "created").unwrap();
    assert!(!created.removable && created.editable);
    let free_keys: Vec<&str> = free_keys.iter().map(|k| k.key.as_str()).collect();
    assert_eq!(free_keys, ["topic_list", "published", "met"]);
}

// プロパティの行は、キーの名前の本体と型を、見出しとしてEmacsに伝える。
#[test]
fn a_row_tells_emacs_the_body_and_the_type_of_its_key() {
    let driver = open_with_properties(json!({ "score_n": "5" }));
    let rows = driver.last_properties().unwrap().0.unwrap();
    let score = rows.iter().find(|r| r.key == "score_n").unwrap();
    assert_eq!(score.label, "score (number)");
}

// まだ付けられるキーも、名前の本体と型の見出しとともにEmacsに伝える。
#[test]
fn a_free_key_reaches_emacs_with_the_body_and_the_type_of_its_name() {
    let driver = open_with_properties(json!({}));
    let (_, free_keys) = driver.last_properties().unwrap();
    let topics = free_keys.iter().find(|k| k.key == "topic_list").unwrap();
    assert_eq!(topics.label, "topic (list)");
}

// 日時のキーをEmacsに伝える。ページ自身の日付、日時の定義を持つ使用中のキー、まだ付けられるキーである。
#[test]
fn the_keys_that_hold_a_date_and_a_time_reach_emacs() {
    let driver = open_with_properties(json!({ "met": "2026-11-08T00:00:00Z", "rating": "5" }));
    assert_eq!(
        driver.last_datetime_keys().unwrap(),
        ["created", "modified", "met", "published"]
    );
}

// 日時の定義を持っていても、今の値がリストなら日時として扱わない。リストとして保存されるため。
#[test]
fn a_datetime_key_that_holds_a_list_is_not_a_datetime_key() {
    let driver = open_with_properties(json!({ "met": ["a", "b"] }));
    let keys = driver.last_datetime_keys().unwrap();
    assert!(!keys.contains(&"met".to_string()));
}

// ジャーナルへ移ると、プロパティの行はなくなる。ジャーナルには付けられないからである。
#[test]
fn a_journal_has_no_property_rows() {
    let mut driver = open_with_properties(json!({}));
    driver.send(Inbound::Go {
        url: "/today".into(),
    });
    assert_eq!(driver.last_properties().unwrap().0, None);
}

// 同じ内容は、二度送らない。
#[test]
fn unchanged_properties_are_told_once() {
    let mut driver = open_with_properties(json!({ "rating": "5" }));
    driver.shell.host.out.clear();
    driver.send(Inbound::Scrolled { top: 0, bottom: 0 });
    assert_eq!(driver.last_properties(), None);
}

// サーバーの答えの値が、Emacsに届く。
#[test]
fn a_saved_property_reaches_emacs() {
    let mut driver = open_with_properties(json!({}));
    driver.fail_all_waiting_calls();
    driver.send(Inbound::SetProperty {
        key: "rating".into(),
        value: "5".into(),
    });
    let mut page = page_info("", "notes");
    page["properties"] = json!({ "rating": "5" });
    let answer = serde_json::from_value(json!({ "PageInfo": page })).unwrap();
    driver.answer(|op| matches!(op, ApiOp::UpsertPageProperty { .. }), answer);
    let rows = driver.last_properties().unwrap().0.unwrap();
    assert_eq!(rows.iter().find(|r| r.key == "rating").unwrap().value, "5");
}

// 終了の前に編集を閉じ、書きかけの文を保存する。答えが返るまで、終わってよい状態にならない。
#[test]
fn shutting_down_saves_the_edit_and_waits_for_the_answer() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.fail_all_waiting_calls();
    driver.send(Inbound::EditRequested {
        id: "a".into(),
        cursor: 1,
    });
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "書きかけ".into(),
    });

    driver.shell.shutdown();

    assert!(driver
        .waiting(|op| matches!(op, ApiOp::UpdateItem { body: Some(b), .. } if b == "書きかけ")));
    assert!(!driver.shell.is_idle());

    let updated = serde_json::from_value(json!({ "Item": item("a", 1, "書きかけ") })).unwrap();
    driver.answer(|op| matches!(op, ApiOp::UpdateItem { .. }), updated);
    driver.fail_all_waiting_calls();
    assert!(driver.shell.is_idle());
}

// 自動保存を待つ間は、終わってよい状態にならない。時計を見る待ちだけが残れば、終わってよい状態になる。
#[test]
fn a_waiting_auto_save_keeps_the_shell_busy_but_the_clock_watch_does_not() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.fail_all_waiting_calls();
    driver.send(Inbound::EditRequested {
        id: "a".into(),
        cursor: 1,
    });
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "書きかけ".into(),
    });
    driver.fail_all_waiting_calls();
    assert!(!driver.shell.is_idle());

    fire_timers(&mut driver);
    driver.fail_all_waiting_calls();
    assert!(driver.shell.is_idle());
}

// 編集を終えると、入力した文は、自動保存のタイマーを待たずに保存される。
#[test]
fn ending_an_edit_saves_the_typed_text_at_once() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::EditRequested {
        id: "a".into(),
        cursor: 1,
    });
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "xy".into(),
    });

    driver.send(Inbound::EditFinished { id: "a".into() });

    assert!(driver.waiting(|op| matches!(op, ApiOp::UpdateItem { body: Some(b), .. } if b == "xy")));
}

// 本文の先頭のBackspaceは、項目を上の項目につなげる。
#[test]
fn backspace_at_the_start_joins_the_item_to_the_one_above() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::BackspaceAtStart { id: "b".into() });
    driver.answer(
        |op| matches!(op, ApiOp::TranscludedItems { .. }),
        ApiOutput::ItemIds(Vec::new()),
    );
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::MergeItems { item_id, into_id, .. } if item_id == "b" && into_id == "a"
    )));
}

// 本文の末尾のDeleteは、下の項目をこの項目につなげる。
#[test]
fn delete_at_the_end_joins_the_item_below_to_this_one() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::DeleteAtEnd { id: "a".into() });
    driver.answer(
        |op| matches!(op, ApiOp::TranscludedItems { .. }),
        ApiOutput::ItemIds(Vec::new()),
    );
    assert!(driver.waiting(|op| matches!(
        op,
        ApiOp::MergeItems { item_id, into_id, .. } if item_id == "b" && into_id == "a"
    )));
}

// アウトデントは、レベルが変わった行として返る。
#[test]
fn outdent_comes_back_as_a_row_with_a_new_level() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 2, "y")]);
    driver.send(Inbound::Outdent { id: "b".into() });
    assert_eq!(driver.last_rows().unwrap()[1].level, 1);
}

// 項目を上へ動かすと、上の項目の前への移動がサーバーに送られる。
#[test]
fn moving_an_item_up_sends_the_move_to_the_server() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::MoveUp { id: "b".into() });
    assert!(driver.waiting(
        |op| matches!(op, ApiOp::MoveItems { item_ids, .. } if item_ids == &["b".to_string()])
    ));
}

// 項目を下へ動かすと、下の項目の後ろへの移動がサーバーに送られる。
#[test]
fn moving_an_item_down_sends_the_move_to_the_server() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::MoveDown { id: "a".into() });
    assert!(driver.waiting(
        |op| matches!(op, ApiOp::MoveItems { item_ids, .. } if item_ids == &["a".to_string()])
    ));
}

// 子を持つ項目をたたむと、子の行が消え、たたんだ印が付く。
#[test]
fn collapsing_an_item_hides_its_children() {
    let mut driver = Driver::open(&[item("a", 1, "親"), item("b", 2, "子")]);
    driver.send(Inbound::CollapseToggle { id: "a".into() });
    let rows = driver.last_rows().unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].collapsed);
}

// 取り消した操作は、やり直せる。
#[test]
fn an_undone_indent_is_done_again_by_redo() {
    let mut driver = Driver::open(&[item("a", 1, "x"), item("b", 1, "y")]);
    driver.send(Inbound::Indent { id: "b".into() });
    driver.fail_all_waiting_calls();
    driver.send(Inbound::Undo);
    assert_eq!(driver.last_rows().unwrap()[1].level, 1);

    driver.send(Inbound::Redo);

    assert_eq!(driver.last_rows().unwrap()[1].level, 2);
}

// 最初のページでのBackは、戻る先がないので、何も開かない。
#[test]
fn back_on_the_first_page_opens_nothing() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::Back);
    assert!(!driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { .. })));
}

// URLの#の後ろの項目は、ページが読めたあとに、Emacsがそこへスクロールする。
#[test]
fn the_item_after_the_hash_of_a_url_is_scrolled_to() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/notes#b".into(),
    });
    driver.load_page(&[item("a", 1, "x"), item("b", 1, "y")]);
    assert!(driver
        .sent()
        .iter()
        .any(|m| matches!(m, Outbound::Scroll { id } if id == "b")));
}

// ジャーナルでは、前のジャーナルへ移れる。
#[test]
fn the_previous_journal_is_asked_for_by_its_date() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/2026-11-05".into(),
    });
    driver.send(Inbound::JournalNav { forward: false });
    assert!(driver.waiting(
        |op| matches!(op, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-11-04")
    ));
}

// ジャーナルでは、次のジャーナルへ移れる。
#[test]
fn the_next_journal_is_asked_for_by_its_date() {
    let mut driver = Driver::new();
    driver.send(Inbound::Start {
        url: "/pages/2026-11-05".into(),
    });
    driver.send(Inbound::JournalNav { forward: true });
    assert!(driver.waiting(
        |op| matches!(op, ApiOp::GetOrCreateJournal { file_name } if file_name == "2026-11-06")
    ));
}

// 最近のページは、パスの一覧としてEmacsに届く。
#[test]
fn the_recent_pages_reach_emacs_as_paths() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    let pages = serde_json::from_value(json!({
        "Pages": [page_info("", "notes"), page_info("work/", "plan")]
    }));
    driver.answer(
        |op| matches!(op, ApiOp::GetRecentPages { .. }),
        pages.unwrap(),
    );
    let recent = driver.sent().iter().rev().find_map(|m| match m {
        Outbound::Recent { paths } => Some(paths.clone()),
        _ => None,
    });
    assert_eq!(recent.unwrap(), ["notes", "work/plan"]);
}

// このページへリンクするページは、ページごとにまとまり、リンクする項目の文と行き先が付く。
#[test]
fn the_backlinks_reach_emacs_grouped_by_page() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    let mut source = item("s", 1, "see [[notes]]");
    source["pageId"] = json!("q");
    source["name"] = json!("other");
    let mut other = page_info("", "other");
    other["id"] = json!("q");
    let backlinks = serde_json::from_value(json!({
        "Backlinks": [{ "sourcePage": other, "sourceItem": source }]
    }));
    driver.answer(
        |op| matches!(op, ApiOp::GetBacklinks { .. }),
        backlinks.unwrap(),
    );
    let groups = driver.sent().iter().rev().find_map(|m| match m {
        Outbound::Backlinks { groups } => Some(groups.clone()),
        _ => None,
    });
    let groups = groups.unwrap();
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].title, "other");
    assert_eq!(groups[0].url, "/pages/other");
    assert_eq!(groups[0].items.len(), 1);
    assert_eq!(groups[0].items[0].text, "see [[notes]]");
    assert_eq!(groups[0].items[0].url, "/pages/other#s");
}

/// The journal that is asked for after Emacs picks `unit` of the date in the calendar.
fn journal_asked_for(driver: &mut Driver, date: &str, unit: DateUnit) -> String {
    driver.send(Inbound::GoToDate {
        date: date.into(),
        unit,
    });
    let asked = driver.take_api(|op| matches!(op, ApiOp::GetOrCreateJournal { .. }));
    match asked {
        Some((_, ApiOp::GetOrCreateJournal { file_name })) => file_name,
        _ => panic!("no journal was asked for"),
    }
}

// カレンダーで選んだ日は、その日のジャーナルを開く。
#[test]
fn a_day_of_the_calendar_opens_its_journal() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    let asked = journal_asked_for(&mut driver, "2026-02-10", DateUnit::Day);
    assert_eq!(asked, "2026-02-10");
}

// カレンダーで選んだ日の月と年は、月のページと年のページを開く。
#[test]
fn the_month_and_the_year_of_a_day_of_the_calendar_open_their_pages() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    assert_eq!(
        journal_asked_for(&mut driver, "2026-02-10", DateUnit::Month),
        "2026-02"
    );
    assert_eq!(
        journal_asked_for(&mut driver, "2026-02-10", DateUnit::Year),
        "2026"
    );
}

// カレンダーで選んだ日の週は、週のページを開く。週の数え方は、サーバーの設定に従う。
#[test]
fn the_week_of_a_day_of_the_calendar_opens_the_week_page_of_the_servers_numbering() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    // A Sunday: the start of week 7 in the US numbering, the end of week 6 in the ISO one
    assert_eq!(
        journal_asked_for(&mut driver, "2026-02-08", DateUnit::Week),
        "2026-W07"
    );

    let config = serde_json::from_value(json!({ "Config": {
        "markdownDir": "", "weekNumbering": "iso", "logicalOutdent": false, "tagsDir": "",
        "homeLink": ""
    } }));
    driver.answer(|op| matches!(op, ApiOp::GetConfig), config.unwrap());

    assert_eq!(
        journal_asked_for(&mut driver, "2026-02-08", DateUnit::Week),
        "2026-W06"
    );
}

/// Asks for the page names of a wiki link, as Emacs does, and lets the pause of the core end.
fn ask_for_page_names(driver: &mut Driver, query: &str) {
    driver.send(Inbound::PageNames {
        id: "a".into(),
        query: query.into(),
    });
    fire_timers(driver);
}

fn page_names_sent(driver: &Driver) -> Vec<(String, Vec<String>)> {
    driver
        .sent()
        .iter()
        .filter_map(|message| match message {
            Outbound::PageNames { query, titles } => Some((query.clone(), titles.clone())),
            _ => None,
        })
        .collect()
}

// ウィキリンクの中の文字で探したページ名は、その文字と一緒にEmacsに届く。
#[test]
fn the_page_names_for_a_wiki_link_reach_emacs_with_what_was_typed() {
    let mut driver = Driver::open(&[item("a", 1, "see [[no]]")]);
    ask_for_page_names(&mut driver, "no");
    let found = serde_json::from_value(json!({ "SearchResults": [
        { "pageId": "p", "pageTitle": "notes", "itemId": "", "snippet": "", "score": 0.0 },
        { "pageId": "q", "pageTitle": "work/notebook", "itemId": "", "snippet": "", "score": 0.0 },
    ] }));
    driver.answer(
        |op| matches!(op, ApiOp::SearchPageNames { query, .. } if query == "no"),
        found.unwrap(),
    );
    assert_eq!(
        page_names_sent(&driver),
        [(
            "no".to_string(),
            vec!["notes".to_string(), "work/notebook".to_string()]
        )]
    );
}

// ページ名が見つからなくても、探し終わったことがEmacsに届く。Emacsは答えを待っているためである。
#[test]
fn a_search_for_page_names_that_finds_nothing_is_answered_too() {
    let mut driver = Driver::open(&[item("a", 1, "see [[zz]]")]);
    ask_for_page_names(&mut driver, "zz");
    driver.answer(
        |op| matches!(op, ApiOp::SearchPageNames { .. }),
        no_results(),
    );
    assert_eq!(page_names_sent(&driver), [("zz".to_string(), Vec::new())]);
}

// ページ名の検索が失敗しても、候補なしとしてEmacsに届く。
#[test]
fn a_failed_search_for_page_names_is_answered_with_no_names() {
    let mut driver = Driver::open(&[item("a", 1, "see [[no]]")]);
    ask_for_page_names(&mut driver, "no");
    let (id, _) = driver
        .take_api(|op| matches!(op, ApiOp::SearchPageNames { .. }))
        .expect("the search was not sent");
    driver.shell.handle(Input::ApiDone {
        id,
        result: Box::new(Err(ApiError("refused".into()))),
    });
    assert_eq!(page_names_sent(&driver), [("no".to_string(), Vec::new())]);
}

/// The sidebar Emacs was told last.
fn sidebar_sent(driver: &Driver) -> Option<SidebarView> {
    driver.sent().iter().rev().find_map(|m| match m {
        Outbound::Sidebar(sidebar) => Some(sidebar.clone()),
        _ => None,
    })
}

// サイドバーは、ページの見出しと最近のページを、Webと同じ並びでEmacsに伝える。
#[test]
fn the_sidebar_tells_emacs_the_headings_and_the_recent_pages() {
    // The server marks the type of a heading
    let mut heading = item("a", 1, "# Top");
    heading["types"] = json!(["heading"]);
    let mut driver = Driver::open(&[heading, item("b", 2, "text")]);
    let pages = serde_json::from_value(json!({
        "Pages": [page_info("", "notes"), page_info("work/", "plan")]
    }));
    driver.answer(
        |op| matches!(op, ApiOp::GetRecentPages { .. }),
        pages.unwrap(),
    );

    let sidebar = sidebar_sent(&driver).unwrap();
    assert_eq!(
        sidebar.headings,
        [SidebarHeading {
            id: "a".into(),
            text: "Top".into(),
            level: 1
        }]
    );
    let recent: Vec<(Option<String>, String, String)> = sidebar
        .recent
        .iter()
        .map(|line| (line.dir.clone(), line.label.clone(), line.path.clone()))
        .collect();
    assert_eq!(
        recent,
        [
            (None, "notes".to_string(), "notes".to_string()),
            (
                Some("work/".to_string()),
                "plan".to_string(),
                "work/plan".to_string()
            )
        ]
    );
    assert!(sidebar.index.is_none());
}

// 変わらないサイドバーは、もう一度は伝えない。
#[test]
fn an_unchanged_sidebar_is_not_told_again() {
    let mut driver = Driver::open(&[item("a", 1, "# Top")]);
    driver.send(Inbound::Scrolled { top: 0, bottom: 1 });
    driver.send(Inbound::Scrolled { top: 0, bottom: 2 });
    let told = driver
        .sent()
        .iter()
        .filter(|m| matches!(m, Outbound::Sidebar(_)))
        .count();
    assert_eq!(told, 0);
}
