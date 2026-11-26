use crate::app::Event;
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::{batch_calls, item, typed, update_calls, Harness};
use crate::model::{
    BatchOpInput, Item, ItemPosition, Page, PageWithItems, PasteCursor, PasteMode, Pasted,
    TranscludedTrees, Uploaded,
};
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

// 貼り付けた項目は、サーバーが返した新しい本文の描画モデルで表示される。
#[test]
fn the_item_shows_the_new_body() {
    let mut h = harness();
    paste(&mut h, "a", "XY", 2);
    let rows = h.view().outline.rows;
    let row = rows.iter().find(|r| r.item.id == "a").unwrap();
    assert_eq!(row.body_model, crate::testing::shown("abXY|cd"));
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

// 開いたブロックの中に貼るとき、要求が運ぶ本文は閉じる記号を補ったものである。貼る位置は変わらない（ADR 050）。
#[test]
fn a_paste_into_an_open_block_carries_the_body_closed() {
    let mut h = harness();
    typed(&mut h, "a", ">>>\nab");
    paste(&mut h, "a", "X", 6);
    assert_eq!(
        h.api.calls,
        [ApiOp::PasteText {
            item_id: "a".into(),
            body: ">>>\nab\n>>>".into(),
            cursor: 6,
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

/// An item of the source page "src".
fn source_item(id: &str, level: i32, body: &str) -> Item {
    item_with(id, level, |i| {
        i.page_id = "src".into();
        i.body = body.into();
    })
}

/// Makes the server answer the next reload of the page with "top", the root, and this tree
/// of the source item.
fn answer_reload(h: &mut Harness, source_tree: Vec<Item>) {
    let root = item_with("root", 2, |i| {
        i.transclude_item_id = Some("s".into());
        i.body = source_tree[0].body.clone();
    });
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), source_tree);
    let page = PageWithItems {
        page: Page {
            id: "page1".into(),
            ..Default::default()
        },
        items: vec![item("top", 1, ""), root],
        transcluded_trees: trees,
    };
    h.api.answer_next("GetPageById", Ok(ApiOutput::Page(page)));
}

/// `id:level=body` of every item on screen, the transcluded ones too.
fn shown(h: &Harness) -> Vec<String> {
    let items = h.model.state.visible_items();
    items
        .iter()
        .map(|i| format!("{}:{}={}", i.id, i.level, i.body))
        .collect()
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

// トランスクルージョンのルートへの貼り付けのあとは、ページが読み直される。作られた項目は参照元ページのものなので、このページの項目としては入らない。
#[test]
fn after_a_paste_into_a_transclusion_root_the_page_is_reloaded() {
    let mut h = transclusion_harness();
    answer_paste(
        &mut h,
        vec![source_item("s", 1, "shownone"), source_item("n1", 2, "two")],
        "n1",
        3,
    );
    answer_reload(
        &mut h,
        vec![
            source_item("s", 1, "shownone"),
            source_item("n1", 2, "two"),
            source_item("c", 2, "child"),
        ],
    );
    paste(&mut h, "root", "one\n  two", 5);
    assert_eq!(h.items_ids(), ["top", "root"]);
    assert_eq!(
        shown(&h),
        ["top:1=", "root:2=shownone", "n1:3=two", "c:3=child"]
    );
}

// トランスクルードされた子孫への貼り付けのあとも、ページが読み直される。作られた項目は、このページの項目としては入らない。
#[test]
fn after_a_paste_into_a_transcluded_descendant_the_page_is_reloaded() {
    let mut h = transclusion_harness();
    answer_paste(
        &mut h,
        vec![source_item("c", 2, "childone"), source_item("n1", 2, "two")],
        "n1",
        3,
    );
    answer_reload(
        &mut h,
        vec![
            source_item("s", 1, "shown"),
            source_item("c", 2, "childone"),
            source_item("n1", 2, "two"),
        ],
    );
    paste(&mut h, "c", "one\ntwo", 5);
    assert_eq!(h.items_ids(), ["top", "root"]);
    assert_eq!(
        shown(&h),
        ["top:1=", "root:2=shown", "c:3=childone", "n1:3=two"]
    );
}

// 読み直しに失敗しても、トランスクルージョンのルートは、元の項目の新しい本文を表示する。
#[test]
fn a_transclusion_root_shows_the_new_body_of_its_source_even_when_the_reload_fails() {
    let mut h = transclusion_harness();
    answer_paste(&mut h, vec![source_item("s", 1, "shownX")], "s", 6);
    h.api.fail_next("GetPageById", "offline");
    paste(&mut h, "root", "X", 5);
    assert_eq!(shown(&h), ["top:1=", "root:2=shownX", "c:3=child"]);
}

// トランスクルージョンのルートへの貼り付けは、元の項目の更新として undo に記録される。元に戻す要求を、元の項目に送るためである。
#[test]
fn a_paste_into_a_transclusion_root_is_recorded_as_an_update_of_the_source_item() {
    let mut h = transclusion_harness();
    answer_paste(&mut h, vec![source_item("s", 1, "shownX")], "s", 6);
    answer_reload(
        &mut h,
        vec![source_item("s", 1, "shownX"), source_item("c", 2, "child")],
    );
    paste(&mut h, "root", "X", 5);
    let entry = h.model.undo.pop_undo().unwrap();
    let [UndoOp::Update { before, after }] = entry.ops.as_slice() else {
        panic!("not one update: {:?}", entry.ops);
    };
    assert_eq!(
        (before.id.as_str(), before.level, before.body.as_str()),
        ("s", 1, "shown")
    );
    assert_eq!(
        (after.id.as_str(), after.level, after.body.as_str()),
        ("s", 1, "shownX")
    );
}

// トランスクルージョンのルートへの貼り付けを元に戻すと、元の本文が元の項目に、元のページでの階層のまま送られる。作られた項目は削除される。
#[test]
fn undoing_a_paste_into_a_transclusion_root_restores_the_source_item_and_deletes_what_was_made() {
    let mut h = transclusion_harness();
    answer_paste(
        &mut h,
        vec![source_item("s", 1, "shownone"), source_item("n1", 2, "two")],
        "n1",
        3,
    );
    answer_reload(
        &mut h,
        vec![
            source_item("s", 1, "shownone"),
            source_item("n1", 2, "two"),
            source_item("c", 2, "child"),
        ],
    );
    paste(&mut h, "root", "one\n  two", 5);
    h.send(Event::Undo);
    assert_eq!(
        batch_calls(&h.api),
        [[
            BatchOpInput::Update {
                id: "s".into(),
                body: Some("shown".into()),
                level: Some(1),
                collapsed: Some(false),
                task: None,
            },
            BatchOpInput::Delete { id: "n1".into() },
        ]]
    );
    assert_eq!(shown(&h), ["top:1=", "root:2=shown", "c:3=child"]);
}

// トランスクルージョンのルートへの貼り付けのあと、カーソルはルートに置かれる。サーバーの答えは元の項目を指すが、画面にあるのはルートである。
#[test]
fn after_a_paste_into_a_transclusion_root_the_cursor_goes_into_the_root() {
    let mut h = transclusion_harness();
    answer_paste(&mut h, vec![source_item("s", 1, "shownX")], "s", 6);
    answer_reload(
        &mut h,
        vec![source_item("s", 1, "shownX"), source_item("c", 2, "child")],
    );
    paste(&mut h, "root", "X", 5);
    assert_eq!(h.focused_editor_requests(), [Some("root".into())]);
    assert_eq!(h.model.state.focused_item_id(), Some("root"));
    assert_eq!(h.focus_cursor(), Some(6));
}

// 作られた項目が元の項目の兄弟になると、読み直したページには現れない。カーソルは、ルートの本文の末尾に置かれる。
#[test]
fn the_cursor_goes_to_the_end_of_the_root_when_the_item_it_was_to_go_to_is_not_shown() {
    let mut h = transclusion_harness();
    answer_paste(
        &mut h,
        vec![source_item("s", 1, "shownone"), source_item("n1", 1, "two")],
        "n1",
        3,
    );
    answer_reload(&mut h, vec![source_item("s", 1, "shownone")]);
    paste(&mut h, "root", "one\ntwo", 5);
    assert_eq!(h.focused_editor_requests(), [Some("root".into())]);
    assert_eq!(h.model.state.focused_item_id(), Some("root"));
    assert_eq!(h.focus_cursor(), Some("shownone".len()));
}

// 作られた項目が元の項目の子孫になると、読み直したページに現れる。カーソルは、サーバーが答えたその項目に置かれる。
#[test]
fn the_cursor_goes_to_the_item_the_server_named_when_the_reloaded_page_shows_it() {
    let mut h = transclusion_harness();
    answer_paste(
        &mut h,
        vec![source_item("s", 1, "shownone"), source_item("n1", 2, "two")],
        "n1",
        3,
    );
    answer_reload(
        &mut h,
        vec![
            source_item("s", 1, "shownone"),
            source_item("n1", 2, "two"),
            source_item("c", 2, "child"),
        ],
    );
    paste(&mut h, "root", "one\n  two", 5);
    assert_eq!(h.focused_editor_requests(), [Some("n1".into())]);
    assert_eq!(h.focus_cursor(), Some(3));
}

// トランスクルージョンのルートへの貼り付けに失敗すると、ルートに入力してまだ保存していない文は、あらためて元の項目に自動保存される。
#[test]
fn after_a_failed_paste_into_a_transclusion_root_the_typed_text_is_saved_to_the_source_again() {
    let mut h = transclusion_harness();
    typed(&mut h, "root", "shown typed");
    h.api.fail_next("PasteText", "offline");
    paste(&mut h, "root", "X", 11);
    h.fire_timers();
    assert_eq!(
        update_calls(&h.api),
        [("s".into(), Some("shown typed".into()))]
    );
}

const LINK: &str = "![pic.png](../assets/pic.png)";

/// An image the server saved under the name it came with, as the fake server answers.
fn uploaded(name: &str) -> Uploaded {
    Uploaded {
        file_name: name.into(),
        original_name: name.into(),
    }
}

fn paste_image(h: &mut Harness, item_id: &str, cursor: usize) {
    h.send(Event::PasteImage {
        item_id: item_id.into(),
        file: 7,
        cursor,
    });
}

// 画像はアップロードされ、そのリンクが、そのまま1項目に入れる貼り付けとして1つの要求で送られる。項目の保存は別には送らない。
// リンクはカーソル位置に入り、カーソルはリンクの直後に置かれる。
#[test]
fn an_image_is_uploaded_and_its_link_is_pasted_at_the_cursor_with_one_request() {
    let mut h = harness();
    paste_image(&mut h, "a", 2);

    assert_eq!(
        h.api.calls,
        [
            ApiOp::UploadAsset { file: 7 },
            ApiOp::PasteText {
                item_id: "a".into(),
                body: "ab|cd".into(),
                cursor: 2,
                text: LINK.into(),
                mode: PasteMode::Raw,
            }
        ]
    );
    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().body,
        format!("ab{LINK}|cd")
    );
    assert_eq!(h.focus_cursor(), Some(2 + LINK.len()));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}

// リンクの文字は元の名前で、リンク先はサーバーが保存した名前である。
// 元の名前の`[`、`]`、`\`は、リンクを壊さないように、前に`\`を付ける。
#[test]
fn the_link_of_an_image_shows_the_original_name_and_points_to_the_saved_file() {
    let mut h = harness();
    h.api.answer_next(
        "UploadAsset",
        Ok(ApiOutput::Uploaded(Uploaded {
            file_name: "20261110090501_QZ998PX.png".into(),
            original_name: r"構成図 [v2]\a.png".into(),
        })),
    );
    paste_image(&mut h, "a", 2);

    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().body,
        r"ab![構成図 \[v2\]\\a.png](../assets/20261110090501_QZ998PX.png)|cd"
    );
}

// 画像の貼り付けは、1つのUndoエントリとして記録される。
#[test]
fn an_image_paste_is_recorded_as_one_undo_entry() {
    let mut h = harness();
    paste_image(&mut h, "a", 2);

    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "paste");
    assert!(h.model.undo.pop_undo().is_none());
}

// 本文は、アップロードの答えが届いたときのものを送る。待つ間に打ってまだ保存していない文は、貼り付けの要求に入り、その自動保存は取り消される。
#[test]
fn text_typed_while_the_image_uploads_travels_in_the_paste() {
    let mut h = harness();
    typed(&mut h, "a", "typed");
    h.send(Event::ImageUploaded {
        item_id: "a".into(),
        image: uploaded("pic.png"),
        cursor: 5,
    });
    h.fire_timers();

    assert_eq!(
        h.api.calls,
        [ApiOp::PasteText {
            item_id: "a".into(),
            body: "typed".into(),
            cursor: 5,
            text: LINK.into(),
            mode: PasteMode::Raw,
        }]
    );
}

// アップロードを待つ間に同じ項目に打つと、リンクは、打ち終えたときのカーソルの位置に入る。打った文字の途中には入らない。
#[test]
fn the_link_goes_where_the_cursor_is_after_typing_while_the_image_uploads() {
    let mut h = harness();
    h.hold("UploadAsset");
    paste_image(&mut h, "a", 2);
    h.model.state.add_editing_item("a");
    let now = h.now_ms;
    h.send(Event::Typed {
        item_id: "a".into(),
        body: "abXY|cd".into(),
        cursor: 4,
        now_ms: now,
    });

    h.release_held();

    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().body,
        format!("abXY{LINK}|cd")
    );
}

// アップロードを待つ間に別の項目に打っても、リンクは貼り付けたときの位置に入る。
#[test]
fn typing_in_another_item_while_the_image_uploads_leaves_the_place_of_the_link() {
    let mut h = harness();
    h.hold("UploadAsset");
    paste_image(&mut h, "a", 2);
    h.model.state.add_editing_item("z");
    let now = h.now_ms;
    h.send(Event::Typed {
        item_id: "z".into(),
        body: "zz".into(),
        cursor: 1,
        now_ms: now,
    });

    h.release_held();

    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().body,
        format!("ab{LINK}|cd")
    );
}

// 1つの項目に2つの画像を貼り付けて、後の画像が先に届いても、そのリンクは自分を貼り付けた位置に入る。
#[test]
fn of_two_images_pasted_into_one_item_the_later_one_arriving_first_goes_to_its_own_place() {
    let mut h = harness();
    h.hold("UploadAsset");
    paste_image(&mut h, "a", 2);
    paste_image(&mut h, "a", 5);

    h.send(Event::ImageUploaded {
        item_id: "a".into(),
        image: uploaded("pic.png"),
        cursor: 5,
    });

    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().body,
        format!("ab|cd{LINK}")
    );
}

// アップロードに失敗した画像は忘れられ、後で同じ位置に貼り付けた画像のリンクの位置を変えない。
#[test]
fn a_failed_upload_does_not_move_the_link_of_a_later_one() {
    let mut h = harness();
    h.api.fail_next("UploadAsset", "too large");
    paste_image(&mut h, "a", 2);
    h.model.state.add_editing_item("a");
    let now = h.now_ms;
    h.send(Event::Typed {
        item_id: "a".into(),
        body: "abXY|cd".into(),
        cursor: 4,
        now_ms: now,
    });
    h.hold("UploadAsset");
    paste_image(&mut h, "a", 2);

    h.release_held();

    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().body,
        format!("ab{LINK}XY|cd")
    );
}

// アップロードに失敗すると、貼り付けは送られず、画面もUndoも変わらない。
#[test]
fn a_failed_upload_pastes_nothing() {
    let mut h = harness();
    h.api.fail_next("UploadAsset", "too large");
    paste_image(&mut h, "a", 2);

    assert!(!h.api.called("PasteText"));
    assert_eq!(h.bodies(), ["a=ab|cd", "z=body of z"]);
    assert!(!h.model.undo.can_undo());
}

// アップロードを待つ間に項目が画面から消えていれば、リンクは貼り付けない。
#[test]
fn the_link_of_an_image_is_not_pasted_into_an_item_that_is_gone() {
    let mut h = harness();
    h.send(Event::ImageUploaded {
        item_id: "gone".into(),
        image: uploaded("pic.png"),
        cursor: 0,
    });

    assert!(h.api.calls.is_empty());
}

// 貼り付けで作られた項目の並び順の値が、上と下の項目の値の間にないときは、ページの並び順の値を読み直す。サーバーが周りの項目の値を振り直したためである。
#[test]
fn a_paste_whose_new_item_is_out_of_place_reads_the_indices_again() {
    let mut h = harness();
    let at = |id: &str, body: &str, idx: &str| {
        item_with(id, 1, |i| {
            i.body = body.into();
            i.idx = idx.into();
        })
    };
    answer_paste(
        &mut h,
        vec![at("a", "abx", "a"), at("p1", "y", "zz")],
        "p1",
        1,
    );

    paste(&mut h, "a", "x\ny", 2);

    assert!(h.api.called("GetPageById"));
}

// 貼り付けで作られた項目の並び順の値が、上と下の項目の値の間にあるときは、読み直さない。
#[test]
fn a_paste_whose_new_item_is_in_place_does_not_read_again() {
    let mut h = harness();
    let at = |id: &str, body: &str, idx: &str| {
        item_with(id, 1, |i| {
            i.body = body.into();
            i.idx = idx.into();
        })
    };
    answer_paste(
        &mut h,
        vec![at("a", "abx", "a"), at("p1", "y", "m")],
        "p1",
        1,
    );

    paste(&mut h, "a", "x\ny", 2);

    assert!(!h.api.called("GetPageById"));
}

/// An item of the source page "src" at the index `idx`.
fn source_item_at(id: &str, level: i32, body: &str, idx: &str) -> Item {
    let mut item = source_item(id, level, body);
    item.idx = idx.into();
    item
}

// トランスクルージョンのルートへの貼り付けのRedoは、作られた項目を元のページに、トランスクルージョンが見せる木の中で上にある項目の後ろに作り直す。このページの項目としては入れず、ページを読み直す（#736）。
#[test]
fn redoing_a_paste_into_a_transclusion_root_makes_the_item_again_on_the_source_page() {
    let mut h = transclusion_harness();
    let tree = vec![
        source_item_at("s", 1, "shownone", "a"),
        source_item_at("n1", 2, "two", "b"),
        source_item_at("c", 2, "child", "c"),
    ];
    answer_paste(&mut h, tree[..2].to_vec(), "n1", 3);
    answer_reload(&mut h, tree.clone());
    paste(&mut h, "root", "one\n  two", 5);
    h.send(Event::Undo);
    let sent = batch_calls(&h.api).len();
    answer_reload(&mut h, tree);

    h.send(Event::Redo);

    let redo = &batch_calls(&h.api)[sent];
    assert!(
        redo.iter().any(|op| matches!(op,
            BatchOpInput::Create { id, page_id, position, .. }
                if id == "n1" && page_id == "src" && *position == ItemPosition::after("s"))),
        "{redo:?}"
    );
    assert_eq!(h.items_ids(), ["top", "root"]);
    assert_eq!(
        shown(&h),
        ["top:1=", "root:2=shownone", "n1:3=two", "c:3=child"]
    );
}
