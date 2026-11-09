use crate::app::Event;
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::{item, update_calls, Harness};
use crate::model::{Item, PasteCursor, PasteMode, Pasted, TranscludedTrees};
use crate::outline::UndoOp;
use crate::testing::item_with;

fn harness() -> Harness {
    Harness::with_items(vec![item("a", 1, "ab|cd"), item("z", 1, "body of z")])
}

fn paste(h: &mut Harness, item_id: &str, text: &str, cursor: usize) {
    send_paste(h, item_id, text, cursor, false);
}

fn paste_raw(h: &mut Harness, item_id: &str, text: &str, cursor: usize) {
    send_paste(h, item_id, text, cursor, true);
}

fn send_paste(h: &mut Harness, item_id: &str, text: &str, cursor: usize, raw: bool) {
    h.send(Event::PasteText {
        item_id: item_id.into(),
        text: text.into(),
        html: String::new(),
        cursor,
        raw,
    });
}

fn typed(h: &mut Harness, item_id: &str, body: &str) {
    let now = h.now_ms;
    h.send(Event::BodyChanged {
        item_id: item_id.into(),
        body: body.into(),
        now_ms: now,
    });
}

/// Makes the server answer the next paste with these items and this cursor.
fn answer_paste(h: &mut Harness, items: Vec<Item>, cursor_item_id: &str, position: usize) {
    let pasted = Pasted {
        items,
        cursor: PasteCursor {
            item_id: cursor_item_id.into(),
            position,
        },
    };
    h.api
        .answer_next("PasteText", Ok(ApiOutput::Pasted(pasted)));
}

/// The text and the mode of every paste request.
fn paste_requests(h: &Harness) -> Vec<(String, PasteMode)> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::PasteText { text, mode, .. } => Some((text.clone(), *mode)),
            _ => None,
        })
        .collect()
}

// 貼り付けは、項目の今の本文とカーソルと文字列を、1つの要求でサーバーに送る。
#[test]
fn a_paste_sends_the_body_the_cursor_and_the_text_in_one_request() {
    let mut h = harness();
    paste(&mut h, "a", "one\ntwo", 2);
    assert_eq!(
        h.api.calls,
        [ApiOp::PasteText {
            item_id: "a".into(),
            body: "ab|cd".into(),
            cursor: 2,
            text: "one\ntwo".into(),
            mode: PasteMode::Auto,
        }]
    );
}

// 一つの断片はカーソル位置の項目に入り、カーソルは貼り付けた文の直後に置かれる。
#[test]
fn one_piece_goes_into_the_item_at_the_cursor() {
    let mut h = harness();
    paste(&mut h, "a", "XY", 2);
    assert_eq!(h.bodies(), ["a=abXY|cd", "z=body of z"]);
    assert!(h.model.edits.dirty_item_ids.contains("a"));
    assert_eq!(h.focused_editor_requests(), [Some("a".into())]);
    assert_eq!(h.focus_cursor(), Some(4));
}

// 貼り付けた項目には、サーバーが新しい本文を解析した結果が表示される。
#[test]
fn the_item_shows_what_the_server_parsed_from_the_new_body() {
    let mut h = harness();
    paste(&mut h, "a", "XY", 2);
    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().ast,
        "ast:abXY|cd"
    );
}

// サーバーが作った項目は、貼り付けた項目の下に順に入る。カーソルは、サーバーが答えた項目と位置に置かれる。
#[test]
fn the_items_the_server_made_go_below_the_item_and_the_cursor_goes_where_the_server_says() {
    let mut h = harness();
    answer_paste(
        &mut h,
        vec![
            item("a", 1, "abone"),
            item("n1", 2, "two"),
            item("n2", 1, "three|cd"),
        ],
        "n2",
        5,
    );
    paste(&mut h, "a", "one\n  two\nthree", 2);
    assert_eq!(
        h.outline(),
        ["a:1=abone", "n1:2=two", "n2:1=three|cd", "z:1=body of z"]
    );
    assert_eq!(h.focused_editor_requests(), [Some("n2".into())]);
    assert_eq!(h.focus_cursor(), Some(5));
}

// 貼り付けは、更新と作られた項目を、一つの undo 項目として記録する。
#[test]
fn records_one_undo_entry_with_the_update_and_the_created_items() {
    let mut h = harness();
    answer_paste(
        &mut h,
        vec![item("a", 1, "abone"), item("n1", 1, "two|cd")],
        "n1",
        3,
    );
    paste(&mut h, "a", "one\ntwo", 2);
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "paste");
    assert!(matches!(
        entry.ops.as_slice(),
        [UndoOp::Update { .. }, UndoOp::Create { .. }]
    ));
    assert_eq!(entry.cursor_before.unwrap().position, 2);
}

// 答えが届いたときに項目がもう画面にないと、何も変わらない。ページを移ったあとに答えが届く場合である。
#[test]
fn an_answer_for_an_item_that_is_no_longer_on_screen_changes_nothing() {
    let mut h = harness();
    h.send(Event::Pasted {
        item_id: "gone".into(),
        cursor_before: 0,
        pasted: Pasted {
            items: vec![item("gone", 1, "x"), item("n1", 1, "y")],
            cursor: PasteCursor {
                item_id: "n1".into(),
                position: 1,
            },
        },
    });
    assert_eq!(h.bodies(), ["a=ab|cd", "z=body of z"]);
    assert!(!h.model.undo.can_undo());
    assert!(h.focused_editor_requests().is_empty());
}

// 貼り付けに失敗すると、画面も undo の記録も変わらない。
#[test]
fn a_failed_paste_changes_neither_the_screen_nor_the_undo_history() {
    let mut h = harness();
    h.api.fail_next("PasteText", "offline");
    paste(&mut h, "a", "one\ntwo", 2);
    assert_eq!(h.bodies(), ["a=ab|cd", "z=body of z"]);
    assert!(!h.model.undo.can_undo());
}

// 入力してまだ保存していない文は、貼り付けの要求に入って届く。その自動保存は取り消される。
#[test]
fn text_typed_and_not_saved_yet_travels_in_the_paste_and_its_auto_save_is_cancelled() {
    let mut h = harness();
    typed(&mut h, "a", "typed");
    paste(&mut h, "a", "X", 5);
    h.fire_timers();
    assert_eq!(
        h.api.calls,
        [ApiOp::PasteText {
            item_id: "a".into(),
            body: "typed".into(),
            cursor: 5,
            text: "X".into(),
            mode: PasteMode::Auto,
        }]
    );
}

// 貼り付けに失敗すると、入力してまだ保存していない文は、あらためて自動保存される。
#[test]
fn after_a_failed_paste_the_text_typed_and_not_saved_yet_is_auto_saved_again() {
    let mut h = harness();
    typed(&mut h, "a", "typed");
    h.api.fail_next("PasteText", "offline");
    paste(&mut h, "a", "X", 5);
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("a".into(), Some("typed".into()))]);
}

// 保存していない文がない項目では、貼り付けに失敗しても何も保存されない。
#[test]
fn a_failed_paste_saves_nothing_when_the_item_had_no_unsaved_text() {
    let mut h = harness();
    h.api.fail_next("PasteText", "offline");
    paste(&mut h, "a", "X", 2);
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
    assert!(!h.model.edits.dirty_item_ids.contains("a"));
}

// このアプリからコピーした文は、保存しておいた Flat Markdown として貼り付けられる。
#[test]
fn text_copied_from_this_app_is_pasted_as_the_stored_flat_markdown() {
    let mut h = harness();
    h.model.clipboard = Some(("flat".into(), "- tree".into()));
    paste(&mut h, "a", "- tree", 5);
    assert_eq!(paste_requests(&h), [("flat".into(), PasteMode::Flat)]);
}

// raw 貼り付けでは、このアプリからコピーした文も、そのままの文として貼り付けられる。
#[test]
fn a_raw_paste_sends_the_text_as_it_is_even_when_it_was_copied_from_this_app() {
    let mut h = harness();
    h.model.clipboard = Some(("flat".into(), "- tree".into()));
    paste_raw(&mut h, "a", "- tree", 2);
    assert_eq!(paste_requests(&h), [("- tree".into(), PasteMode::Raw)]);
}

/// A page with "top", and below it the transclusion root "root" of the source item "s" on
/// the page "src". The child "c" of the source is shown below the root.
fn transclusion_harness() -> Harness {
    let root = item_with("root", 2, |i| {
        i.transclude_item_id = Some("s".into());
        i.body = "shown".into();
    });
    let mut h = Harness::with_items(vec![item("top", 1, ""), root]);
    let mut trees = TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![
            item_with("s", 1, |i| {
                i.page_id = "src".into();
                i.body = "shown".into();
            }),
            item_with("c", 2, |i| {
                i.page_id = "src".into();
                i.body = "child".into();
            }),
        ],
    );
    h.model.state.set_transcluded_trees(trees);
    h
}

// トランスクルージョンのルートに入力してまだ保存していない文も、貼り付けの要求に入って届く。
#[test]
fn text_typed_into_a_transclusion_root_and_not_saved_yet_travels_in_the_paste() {
    let mut h = transclusion_harness();
    typed(&mut h, "root", "shown typed");
    paste(&mut h, "root", "X", 11);
    assert_eq!(
        h.api.calls_named("PasteText"),
        [&ApiOp::PasteText {
            item_id: "root".into(),
            body: "shown typed".into(),
            cursor: 11,
            text: "X".into(),
            mode: PasteMode::Auto,
        }]
    );
}

// 画像はアップロードされてカーソル位置にリンクされ、カーソルはリンクの直後に置かれる。
#[test]
fn an_image_is_uploaded_linked_at_the_cursor_and_the_cursor_goes_after_the_link() {
    let mut h = harness();
    h.send(Event::PasteImage {
        item_id: "a".into(),
        file: 7,
        cursor: 2,
    });
    assert!(h.api.called("UploadAsset"));
    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().body,
        "ab![pic.png](../assets/pic.png)|cd"
    );
    assert_eq!(
        h.focus_cursor(),
        Some(2 + "![pic.png](../assets/pic.png)".len())
    );
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}
