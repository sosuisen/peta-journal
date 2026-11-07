use crate::protocol::{Bullet, Envelope, Inbound, Outbound, RowView};
use crate::shell::{Host, Input, Now, Shell};
use peta_core::effects::{ApiError, ApiOp, ApiOutput, LocalDateTimeDto, SseEvent};
use serde_json::{json, Value};
use std::mem::ManuallyDrop;

#[derive(Default)]
struct FakeHost {
    out: Vec<Outbound>,
    apis: Vec<(u64, ApiOp)>,
    timers: Vec<(u64, u32)>,
    streams: Vec<u64>,
}

impl Host for FakeHost {
    fn emit(&mut self, message: Outbound) {
        self.out.push(message);
    }
    fn api(&mut self, id: u64, op: ApiOp) {
        self.apis.push((id, op));
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
                day: 6,
                hour: 9,
                minute: 30,
            },
            now_ms: 1_000,
        }
    }
}

fn item(id: &str, level: i32, body: &str) -> Value {
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

fn page_answer(items: &[Value]) -> ApiOutput {
    serde_json::from_value(json!({
        "Page": { "page": page_info("", "notes"), "items": items }
    }))
    .unwrap()
}

struct Driver {
    /// Never dropped. A test ends with commands of the core still waiting, and one of them
    /// sends an event from its destructor, which panics once the core is gone. The real
    /// process ends with `process::exit`, so this only happens in a test.
    shell: ManuallyDrop<Shell<FakeHost>>,
    seq: u64,
}

impl Driver {
    fn new() -> Driver {
        Driver {
            shell: ManuallyDrop::new(Shell::new(FakeHost::default())),
            seq: 0,
        }
    }

    fn send(&mut self, message: Inbound) {
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
    fn fail_all_waiting_calls(&mut self) {
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

    fn waiting(&self, pick: impl Fn(&ApiOp) -> bool) -> bool {
        self.shell.host.apis.iter().any(|(_, op)| pick(op))
    }

    /// Answers the two calls that load a page: the page, then its items.
    fn load_page(&mut self, items: &[Value]) {
        self.answer(
            |op| matches!(op, ApiOp::GetOrCreateDoc { .. }),
            page_answer(&[]),
        );
        self.answer(
            |op| matches!(op, ApiOp::GetPageById { .. }),
            page_answer(items),
        );
    }

    /// Opens a page with these items and forgets what was said until now.
    fn open(items: &[Value]) -> Driver {
        let mut driver = Driver::new();
        driver.send(Inbound::Start {
            url: "/pages/notes".into(),
        });
        driver.load_page(items);
        driver.shell.host.out.clear();
        driver
    }

    fn sent(&self) -> &[Outbound] {
        &self.shell.host.out
    }

    fn last_rows(&self) -> Option<Vec<RowView>> {
        self.sent().iter().rev().find_map(|m| match m {
            Outbound::Rows { rows, .. } => Some(rows.clone()),
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

// 入力した文は、Emacsが持っているものなので、送り返されない。
#[test]
fn typed_text_is_not_sent_back_to_emacs() {
    let mut driver = Driver::open(&[item("a", 1, "x")]);
    driver.send(Inbound::BodyChanged {
        id: "a".into(),
        body: "xy".into(),
    });
    assert_eq!(driver.rows_sent(), 0);
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

// 末尾でのEnterは、新しい行をすぐに示し、カーソルをその行へ移し、作成をサーバーへ送る。
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
    let rows = driver.last_rows().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].id, "a");
    assert_eq!(rows[1].body, "");
    let new_id = rows[1].id.clone();
    assert!(driver.focuses().iter().any(|(id, _)| *id == new_id));
    assert!(driver.waiting(|op| matches!(op, ApiOp::CreateItem { .. })));
}

// 途中でのEnterの間はキーを止め、新しい項目へカーソルが移ったら、止めるのをやめたと伝える。
#[test]
fn the_end_of_holding_keys_reaches_emacs() {
    let mut driver = Driver::open(&[item("a", 1, "abcd")]);
    driver.send(Inbound::Enter {
        id: "a".into(),
        cursor: 2,
    });
    let holding = |driver: &Driver| {
        driver
            .sent()
            .iter()
            .rev()
            .find_map(|message| match message {
                Outbound::State { holding, .. } => Some(*holding),
                _ => None,
            })
    };
    assert_eq!(holding(&driver), Some(true));
    let answer = |value| ApiOutput::Item(serde_json::from_value(value).unwrap());
    driver.answer(
        |op| matches!(op, ApiOp::UpdateItem { .. }),
        answer(item("a", 1, "ab")),
    );
    driver.answer(
        |op| matches!(op, ApiOp::CreateItem { .. }),
        answer(item("b", 1, "cd")),
    );
    assert_eq!(driver.focuses().last().unwrap().0, "b");
    assert_eq!(holding(&driver), Some(false));
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
    assert_eq!(driver.last_rows().unwrap().len(), 2);
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
    let parsed = serde_json::from_value(json!({ "ParsedItems": [{ "level": 1, "body": "Z" }] }));
    driver.answer(
        |op| matches!(op, ApiOp::ParseFlatMarkdown { .. }),
        parsed.unwrap(),
    );
    assert_eq!(driver.last_rows().unwrap()[0].body, "xZy");
}

// 項目の削除は、参照の確認を経て、子ごとサーバーへ送られる。
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
    assert!(
        driver.waiting(|op| matches!(op, ApiOp::GetOrCreateDoc { title, .. } if title == "notes"))
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
            { "pageId": "p1", "pageTitle": "rust", "itemId": "", "snippet": "" },
            { "pageId": "p2", "pageTitle": "memo", "itemId": "i1", "snippet": "rust本" },
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
    assert_eq!(entries.first().unwrap().index, 0);
    let last = entries.last().unwrap();
    assert_eq!(last.snippet, "rust本");

    driver.send(Inbound::SearchResult { index: last.index });
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
            "Page": { "page": page_info("a/", name), "items": [] }
        }))
        .unwrap();
        driver.answer(|op| matches!(op, ApiOp::GetOrCreateDoc { .. }), answer);
        assert!(driver.page_paths().iter().all(Option::is_none), "{name}");
    }
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
