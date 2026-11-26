use crate::app::Event;
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::{batch_calls, create_calls, item, split_calls, typed, update_calls, Harness};
use crate::model::{BatchOpInput, Item, Page, PageWithItems, Split, TranscludedTrees};
use crate::testing::item_with;

fn harness() -> Harness {
    let mut h = Harness::with_items(vec![item("a", 1, "hello world"), item("b", 1, "body of b")]);
    h.model.state.add_editing_item("a");
    h
}

fn enter(h: &mut Harness, item_id: &str, cursor: usize) {
    let now = h.now_ms;
    h.send(Event::EnterPressed {
        item_id: item_id.into(),
        cursor,
        now_ms: now,
    });
}

// 途中での Enter は、本文とカーソルを1つの要求でサーバーに送る。カーソルより前の文が残り、残りは下の新しい項目に移る。編集は新しい項目に移り、カーソルはその先頭に置かれる。
#[test]
fn enter_in_the_middle_keeps_the_text_before_the_cursor_and_moves_the_rest_below() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    assert_eq!(h.bodies(), ["a=hello", "new1= world", "b=body of b"]);
    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "hello world".to_string(), 5)]
    );
    assert_eq!(h.api.calls.len(), 1, "one request does it all");
    assert!(h.model.state.is_editing("new1"));
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(h.focused_editor_requests(), [Some("new1".to_string())]);
    assert_eq!(h.focus_cursor(), Some(0));
}

// 2行目以降の行頭でのEnterは、カーソルの直前の改行を除いて分割する。上の項目の最後に改行は残らない（#927）。
#[test]
fn enter_at_the_start_of_a_later_line_leaves_no_newline_at_the_end_of_the_item_above() {
    let mut h = Harness::with_items(vec![item("a", 1, "abc\ndef")]);
    h.model.state.add_editing_item("a");
    enter(&mut h, "a", 4);
    assert_eq!(h.bodies(), ["a=abc", "new1=def"]);
    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "abcdef".to_string(), 3)]
    );
}

// 閉じていないコードフェンスの中のEnterは、項目を分けずに改行を入れ、カーソルを次の行へ進める。改行はふつうの入力と同じく後で保存される。
#[test]
fn enter_inside_an_open_code_fence_adds_a_line_instead_of_an_item() {
    let mut h = Harness::with_items(vec![item("a", 1, "```\n日本語\n```")]);
    h.model.state.add_editing_item("a");
    enter(&mut h, "a", 7);
    assert_eq!(h.bodies(), ["a=```\n日本語\n\n```"]);
    assert_eq!(h.focus_cursor(), Some(8));
    assert!(h.model.state.is_editing("a"));
    assert!(!h.model.key_relay.is_holding());
    assert!(create_calls(&h.api).is_empty());
    assert!(update_calls(&h.api).is_empty());
    h.fire_timers();
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("```\n日本語\n\n```".into()))]
    );
}

// 開いたブロックの中のEnterは、どの種類のブロックでも改行になる。数式のブロック、複数行の引用、HTMLのブロックである（ADR 050）。
#[test]
fn enter_inside_any_open_block_adds_a_line_instead_of_an_item() {
    for body in ["```math\nx\n```", ">>>\nab\n>>>", "<!--\nab\n-->"] {
        let mut h = Harness::with_items(vec![item("a", 1, body)]);
        h.model.state.add_editing_item("a");
        let cursor = body.find('\n').unwrap() + 2;
        enter(&mut h, "a", cursor);
        let (before, after) = body.split_at(cursor);
        assert_eq!(h.bodies(), [format!("a={before}\n{after}")], "{body:?}");
        assert!(split_calls(&h.api).is_empty(), "{body:?}");
    }
}

// 閉じていないブロックより前でのEnterは項目を分け、分割の要求は閉じる記号を補った本文を運ぶ。サーバーは閉じていない本文を断るからである（ADR 050）。
#[test]
fn enter_before_an_unclosed_block_carries_the_body_closed() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "a\n>>>\nb");
    enter(&mut h, "a", 1);
    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "a\n>>>\nb\n>>>".to_string(), 1)]
    );
}

// 閉じたコードフェンスの後のEnterは、ふつうに項目を分ける。
#[test]
fn enter_after_a_closed_code_fence_splits_the_item() {
    let mut h = Harness::with_items(vec![item("a", 1, "```\ncode\n```\nafter")]);
    h.model.state.add_editing_item("a");
    enter(&mut h, "a", 12);
    assert_eq!(h.bodies(), ["a=```\ncode\n```", "new1=\nafter"]);
}

// 末尾での Enter は、空の項目を下に置き、その項目が編集される。サーバーの答えを待ってから表示する。
#[test]
fn enter_at_the_end_puts_an_empty_item_below_and_edits_it() {
    let mut h = harness();
    enter(&mut h, "a", "hello world".len());
    assert_eq!(h.bodies(), ["a=hello world", "new1=", "b=body of b"]);
    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "hello world".to_string(), 11)]
    );
    assert_eq!(h.api.calls.len(), 1);
    assert!(h.model.state.is_editing("new1"));
    assert_eq!(h.model.state.focused_item_id(), Some("new1"));
}

// カーソルが本文より後ろにあっても、本文の末尾として送る。
#[test]
fn a_cursor_past_the_end_of_the_body_is_sent_as_its_end() {
    let mut h = harness();
    enter(&mut h, "a", 99);
    assert_eq!(split_calls(&h.api)[0].2, 11);
}

// 分割に失敗すると、画面は変わらず、元の項目の編集が続く。キーの保留は終わる。
#[test]
fn a_split_that_fails_changes_nothing_on_screen_and_stops_holding_keys() {
    let mut h = harness();
    h.api.fail_next("SplitItem", "offline");
    enter(&mut h, "a", 5);
    assert_eq!(h.bodies(), ["a=hello world", "b=body of b"]);
    assert!(h.model.state.is_editing("a"));
    assert!(!h.model.key_relay.is_holding());
    assert!(h.focused_editor_requests().is_empty());
    assert!(!h.model.undo.can_undo());
}

// 先頭での Enter は空の項目を上に置き、その新しい項目が編集される。
#[test]
fn enter_at_the_start_puts_an_empty_item_above_and_keeps_editing_the_new_one() {
    let mut h = harness();
    enter(&mut h, "a", 0);
    assert_eq!(h.bodies(), ["new1=", "a=hello world", "b=body of b"]);
    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "hello world".to_string(), 0)]
    );
    assert_eq!(h.focused_editor_requests(), [Some("new1".to_string())]);
    assert!(h.model.state.is_editing("new1"));
    assert!(!h.model.state.is_editing("a"));
}

// 2番目以降の項目の先頭での Enter でも、空の項目はその項目の直前に入る。
#[test]
fn enter_at_the_start_of_a_later_item_puts_the_empty_item_right_above_it() {
    let mut h = harness();
    enter(&mut h, "b", 0);
    assert_eq!(h.bodies(), ["a=hello world", "new1=", "b=body of b"]);
}

/// What the server answers for a split of `id` that keeps `kept`: the new item `n` with the
/// given index and level.
fn split_answer(
    id: &str,
    kept: &str,
    idx: &str,
    level: i32,
) -> crate::effects::ApiResult<ApiOutput> {
    Ok(ApiOutput::Split(Box::new(Split {
        item: item(id, 1, kept),
        created: item_with("n", level, |i| i.idx = idx.into()),
    })))
}

// サーバーが新しい項目を1段深い階層で作ったときは、元の項目の最初の子として表示される。
#[test]
fn a_new_item_one_level_deeper_is_shown_as_the_first_child_of_the_item() {
    let mut h = Harness::with_items(vec![item("a", 1, "parent"), item("c", 2, "")]);
    h.api
        .answer_next("SplitItem", split_answer("a", "parent", "a5", 2));
    enter(&mut h, "a", "parent".len());
    assert_eq!(h.outline(), ["a:1=parent", "n:2=", "c:2="]);
}

// サーバーが新しい項目を同じ階層で作ったときは、元の項目の子孫の後ろに表示される。折りたたまれた親の分割である。
#[test]
fn a_new_item_of_the_same_level_is_shown_after_the_descendants_of_the_item() {
    let mut h = Harness::with_items(vec![
        item_with("a", 1, |i| {
            i.body = "parent".into();
            i.collapsed = true;
        }),
        item("c", 2, ""),
        item("d", 3, ""),
        item("e", 1, ""),
    ]);
    h.api
        .answer_next("SplitItem", split_answer("a", "parent", "d5", 1));
    enter(&mut h, "a", "parent".len());
    assert_eq!(h.items_ids(), ["a", "c", "d", "n", "e"]);
}

// 空の最後の子での Enter は、項目を作る代わりにその子をアウトデントする。
#[test]
fn enter_on_an_empty_last_child_outdents_it_instead() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("e", 2, "")]);
    enter(&mut h, "e", 0);
    assert!(!h.api.called("CreateItem"));
    assert!(h.api.called("ShiftItemLevels"));
    assert_eq!(h.model.state.get_item_by_id("e").unwrap().level, 1);
    assert_eq!(h.focused_editor_requests(), [Some("e".to_string())]);
}

// エディタにフォーカスする指示は、番号を付けてビューモデルにも入る。番号は指示のたびに増える。カーソルの位置が変わらないアウトデントでも入る。
#[test]
fn every_order_to_focus_an_editor_is_numbered_in_the_view_model() {
    let mut h = Harness::with_items(vec![item("a", 1, "hello"), item("e", 2, "")]);
    assert_eq!(h.view().outline.focus_order, None);

    enter(&mut h, "e", 0);
    let first = h.view().outline.focus_order.unwrap();
    assert_eq!(first.item_id.as_deref(), Some("e"));

    enter(&mut h, "a", 5);
    let second = h.view().outline.focus_order.unwrap();
    assert_eq!(second.item_id.as_deref(), Some("new1"));
    assert!(second.number > first.number);
}

// 押されたキーは、アウトラインが次のエディタにフォーカスするまで保留される。フォーカス要求が出た時点で保留は解かれている。
#[test]
fn holds_the_keys_pressed_until_the_outline_focuses_the_next_editor() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    // The focus request released the held keys
    assert!(!h.model.key_relay.is_holding());
    assert!(!h.focused_editor_requests().is_empty());
}

// 途中での Enter の undo は、元の本文を戻し、作られた項目を消す。カーソルは Enter を押した位置に戻る。
#[test]
fn undo_of_an_enter_in_the_middle_puts_the_body_back_and_removes_the_new_item() {
    let mut h = harness();
    enter(&mut h, "a", 5);

    h.send(Event::Undo);

    assert_eq!(h.bodies(), ["a=hello world", "b=body of b"]);
    assert!(h.model.state.is_editing("a"));
    assert_eq!(h.focus_cursor(), Some(5));
    let batch = &batch_calls(&h.api)[0];
    assert!(
        matches!(&batch[0], BatchOpInput::Update { id, body, .. } if id == "a" && body.as_deref() == Some("hello world")),
        "{batch:?}"
    );
    assert_eq!(batch[1], BatchOpInput::Delete { id: "new1".into() });
}

// 末尾や先頭での Enter は本文を変えないので、undo には作成だけが記録される。
#[test]
fn an_enter_that_leaves_the_body_as_it_is_records_only_the_creation() {
    let mut h = harness();
    enter(&mut h, "a", 0);

    h.send(Event::Undo);

    assert_eq!(h.bodies(), ["a=hello world", "b=body of b"]);
    assert_eq!(
        batch_calls(&h.api),
        [[BatchOpInput::Delete { id: "new1".into() }]]
    );
}

// 入力してまだ保存していない文は、分割の要求に入って届く。その自動保存は取り消される。
#[test]
fn typed_text_not_saved_yet_travels_in_the_split_and_its_auto_save_is_dropped() {
    let mut h = harness();
    typed(&mut h, "a", "hello world!");
    enter(&mut h, "a", 5);
    assert_eq!(
        split_calls(&h.api),
        [("a".to_string(), "hello world!".to_string(), 5)]
    );
    assert!(h.pending_timer_ids().is_empty());
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
    assert_eq!(h.bodies(), ["a=hello", "new1= world!", "b=body of b"]);
}

// 分割に失敗すると、要求に入っていた未保存の入力は、もう一度自動保存にかけられる。
#[test]
fn after_a_failed_split_the_text_typed_and_not_saved_yet_is_auto_saved_again() {
    let mut h = harness();
    typed(&mut h, "a", "hello world!");
    h.api.fail_next("SplitItem", "offline");
    enter(&mut h, "a", 5);
    h.fire_timers();
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("hello world!".into()))]
    );
}

// 保存済みの本文での分割が失敗しても、何も保存されない。
#[test]
fn a_failed_split_saves_nothing_when_the_item_had_no_unsaved_text() {
    let mut h = harness();
    h.api.fail_next("SplitItem", "offline");
    enter(&mut h, "a", 5);
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
}

// 答えが届いたときに元の項目がもう画面に無ければ、何も表示しない。待っている間に別のページへ移った場合である。キーの保留は終わる。
#[test]
fn the_answer_to_a_split_of_an_item_no_longer_shown_changes_nothing() {
    let mut h = harness();
    h.model.key_relay.hold_until_focus();
    h.send(Event::SplitDone {
        item_id: "gone".into(),
        cursor_before: 0,
        split: Box::new(Split {
            item: item("gone", 1, ""),
            created: item("n", 1, ""),
        }),
    });
    assert_eq!(h.items_ids(), ["a", "b"]);
    assert!(!h.model.undo.can_undo());
    assert!(!h.model.key_relay.is_holding());
    assert!(h.focused_editor_requests().is_empty());
}

// 元の項目は、残した本文の描画モデルで表示される。分割で本文が変わるためである。
#[test]
fn the_item_that_was_split_shows_the_text_it_keeps() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    let rows = h.view().outline.rows;
    let row = rows.iter().find(|r| r.item.id == "a").unwrap();
    assert_eq!(row.body_model, crate::testing::shown("hello"));
}

/// The transclusion of the tests below: an item of this page that shows the item "s"
fn root() -> Item {
    item_with("root", 1, |i| i.transclude_item_id = Some("s".into()))
}

/// An item of the source page
fn source(id: &str, level: i32, body: &str) -> Item {
    item_with(id, level, |i| {
        i.page_id = "src".into();
        i.body = body.into();
    })
}

/// A host page with "a", then the transclusion root "root" of source item "s", whose child
/// "c" is the last transcluded item.
fn transclusion_harness(own_after_root: Vec<Item>) -> Harness {
    let mut own = vec![item("a", 1, ""), root()];
    own.extend(own_after_root);
    let mut h = Harness::with_items(own);
    let mut trees = TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![source("s", 1, "shown"), source("c", 2, "child")],
    );
    h.model.state.set_transcluded_trees(trees);
    h
}

/// What the server answers when the host page is read again: its items, and the items the
/// transclusion shows.
fn reloaded(own: Vec<Item>, tree: Vec<Item>) -> crate::effects::ApiResult<ApiOutput> {
    Ok(ApiOutput::Page(PageWithItems {
        page: Page {
            id: "page1".into(),
            ..Default::default()
        },
        items: own,
        transcluded_trees: TranscludedTrees::from([("s".to_string(), tree)]),
    }))
}

/// What the server answers for a split inside the transclusion of `transclusion_harness`:
/// the item written to, of the source page, with the body it keeps, and the new item `n`
/// on the given page.
fn split_on_source_answer(
    written_id: &str,
    kept: &str,
    new_page_id: &str,
) -> crate::effects::ApiResult<ApiOutput> {
    Ok(ApiOutput::Split(Box::new(Split {
        item: item_with(written_id, 1, |i| {
            i.page_id = "src".into();
            i.body = kept.into();
        }),
        created: item_with("n", 1, |i| i.page_id = new_page_id.into()),
    })))
}

/// The names of the requests sent, in their order
fn requests(h: &Harness) -> Vec<&'static str> {
    h.api.calls.iter().map(crate::harness::op_name).collect()
}

// トランスクルージョンの途中での Enter も、本文とカーソルを1つの要求でサーバーに送る。元の項目の更新や項目の作成は、別には送らない。答えが届くと、ページをサーバーから読み直す。
#[test]
fn enter_in_a_transclusion_sends_one_split_and_then_reads_the_page_again() {
    let mut h = transclusion_harness(Vec::new());
    h.api
        .answer_next("SplitItem", split_on_source_answer("s", "sho", "src"));
    enter(&mut h, "root", "sho".len());
    assert_eq!(
        h.api.calls[0],
        ApiOp::SplitItem {
            item_id: "root".into(),
            root_id: None,
            body: "shown".into(),
            cursor: 3,
        }
    );
    assert_eq!(requests(&h), ["SplitItem", "GetPageById"]);
}

// 見せている最後の項目の末尾での Enter でサーバーがこのページに作った項目は、読み直したページに現れ、編集はその項目に移る。
#[test]
fn the_edit_moves_to_the_new_item_below_the_transclusion_once_the_page_is_read_again() {
    let mut h = transclusion_harness(vec![item("b", 1, "")]);
    h.model.state.add_editing_item("c");
    h.api
        .answer_next("SplitItem", split_on_source_answer("c", "child", "page1"));
    h.api.answer_next(
        "GetPageById",
        reloaded(
            vec![item("a", 1, ""), root(), item("n", 1, ""), item("b", 1, "")],
            vec![source("s", 1, "shown"), source("c", 2, "child")],
        ),
    );
    enter(&mut h, "c", "child".len());
    assert_eq!(h.items_ids(), ["a", "root", "n", "b"]);
    assert!(h.model.state.is_editing("n"));
    assert!(!h.model.state.is_editing("c"));
    assert_eq!(h.focused_editor_requests(), [Some("n".to_string())]);
    assert_eq!(h.focus_cursor(), Some(0));
    assert!(!h.model.key_relay.is_holding());
}

// トランスクルージョンの行頭での Enter は、カーソル0の分割として送る。サーバーがこのページでその直前に作った空の項目が、読み直しのあと編集される。
#[test]
fn enter_at_the_start_of_a_transclusion_edits_the_empty_item_the_server_put_above_it() {
    let mut h = transclusion_harness(Vec::new());
    h.api
        .answer_next("SplitItem", split_on_source_answer("s", "shown", "page1"));
    h.api.answer_next(
        "GetPageById",
        reloaded(
            vec![item("a", 1, ""), item("n", 1, ""), root()],
            vec![source("s", 1, "shown"), source("c", 2, "child")],
        ),
    );
    enter(&mut h, "root", 0);
    assert_eq!(
        split_calls(&h.api),
        [("root".to_string(), "shown".to_string(), 0)]
    );
    assert_eq!(h.items_ids(), ["a", "n", "root"]);
    assert_eq!(h.focused_editor_requests(), [Some("n".to_string())]);
}

// 見せている項目の行頭での Enter も、カーソル0の分割として送り、ページを読み直す。空の項目を元のページのどこに置くかは、サーバーが決める。
#[test]
fn enter_at_the_start_of_an_item_a_transclusion_shows_is_sent_as_a_split_at_the_start() {
    let mut h = transclusion_harness(Vec::new());
    enter(&mut h, "c", 0);
    assert_eq!(
        split_calls(&h.api),
        [("c".to_string(), "child".to_string(), 0)]
    );
    assert_eq!(requests(&h), ["SplitItem", "GetPageById"]);
}

// トランスクルージョンが見せている項目での Enter は、どのトランスクルージョンが見せているかも送る。サーバーは、見せている最後の項目の末尾でなら、空の項目をトランスクルージョンのページに置く。
#[test]
fn enter_in_an_item_a_transclusion_shows_names_the_transclusion() {
    let mut h = transclusion_harness(Vec::new());
    enter(&mut h, "c", "child".len());
    assert_eq!(
        h.api.calls[0],
        ApiOp::SplitItem {
            item_id: "c".into(),
            root_id: Some("root".into()),
            body: "child".into(),
            cursor: 5,
        }
    );
}

/// A host page with "a" and the transclusion "root" of the source item "s", which has no
/// children. The server answers a split in the middle of "root" with the new item "n" on the
/// source page, below "s", where the host page does not show it.
fn split_that_leaves_the_page() -> Harness {
    let mut h = Harness::with_items(vec![item("a", 1, ""), root()]);
    h.model
        .state
        .set_transcluded_trees(TranscludedTrees::from([(
            "s".to_string(),
            vec![source("s", 1, "shown")],
        )]));
    h.model.state.add_editing_item("root");
    h.api
        .answer_next("SplitItem", split_on_source_answer("s", "sho", "src"));
    h.api.answer_next(
        "GetPageById",
        reloaded(vec![item("a", 1, ""), root()], vec![source("s", 1, "sho")]),
    );
    h
}

// サーバーが新しい項目を元のページに作り、このページがそれを見せないときは、編集は分割した項目に残り、カーソルは残った文の末尾に置かれる。後ろの文は、このページからは見えなくなる。
#[test]
fn the_edit_stays_in_the_transclusion_when_the_page_does_not_show_the_new_item() {
    let mut h = split_that_leaves_the_page();
    enter(&mut h, "root", "sho".len());
    assert_eq!(h.model.current_item("root").unwrap().body, "sho");
    assert!(h.model.state.is_editing("root"));
    assert_eq!(h.focused_editor_requests(), [Some("root".to_string())]);
    assert_eq!(h.focus_cursor(), Some(3));
    assert!(!h.model.key_relay.is_holding());
}

// トランスクルージョンの分割の undo は、元の項目の本文を戻し、作られた項目を消す。このページに見えていない項目も消す。
#[test]
fn undo_of_a_split_transclusion_puts_the_body_of_the_source_back_and_removes_the_new_item() {
    let mut h = split_that_leaves_the_page();
    enter(&mut h, "root", "sho".len());

    h.send(Event::Undo);

    let batch = &batch_calls(&h.api)[0];
    assert!(
        matches!(&batch[0], BatchOpInput::Update { id, body, .. } if id == "s" && body.as_deref() == Some("shown")),
        "{batch:?}"
    );
    assert_eq!(batch[1], BatchOpInput::Delete { id: "n".into() });
}

// 分割のあとの読み直しの答えより先に取り消すと、編集とカーソルは取り消しが置いた所に残る。読み直しのあとの編集の移動は行わない。
#[test]
fn an_undo_that_overtakes_the_reading_after_a_split_keeps_the_cursor_where_the_undo_put_it() {
    // The split is answered and recorded, and the page is read again
    let mut h = split_that_leaves_the_page();
    enter(&mut h, "root", "sho".len());
    let asked_after_the_split = crate::ops::file_reload::AskedAt::now(&h.model);
    h.send(Event::Undo);
    assert_eq!(h.model.current_item("root").unwrap().body, "shown");
    let focused = h.focused_editor_requests().len();
    // The page is read once more at once, and the server has the undo by then
    h.api.answer_next(
        "GetPageById",
        reloaded(
            vec![item("a", 1, ""), root()],
            vec![source("s", 1, "shown")],
        ),
    );

    // The answers of the reading that was asked for before the undo
    h.send(Event::PageReplaced {
        page_id: "page1".into(),
        loaded: Box::new(PageWithItems {
            items: vec![item("a", 1, ""), root()],
            transcluded_trees: TranscludedTrees::from([(
                "s".to_string(),
                vec![source("s", 1, "sho")],
            )]),
            ..Default::default()
        }),
        asked_at: asked_after_the_split,
    });
    h.send(Event::SplitPageReloaded {
        item_id: "root".into(),
        created_id: "n".into(),
        asked_at: asked_after_the_split,
    });

    assert_eq!(h.model.current_item("root").unwrap().body, "shown");
    assert_eq!(h.focus_cursor(), Some(3));
    assert_eq!(h.focused_editor_requests().len(), focused);
    assert!(!h.model.key_relay.is_holding());
}

// 答えのあと読み直すのは、トランスクルージョンのあるページである。ジャーナルは何日ものページを見せるので、画面のページとは限らない。
#[test]
fn the_page_read_again_is_the_page_of_the_transclusion() {
    for split_id in ["root", "c"] {
        let mut h = transclusion_harness(Vec::new());
        h.model
            .state
            .update_item("root", |i| i.page_id = "day2".into());
        enter(&mut h, split_id, 2);
        assert_eq!(
            h.api.calls[1],
            ApiOp::GetPageById {
                page_id: "day2".into()
            },
            "after a split of {split_id}"
        );
    }
}

// トランスクルージョンに入力してまだ保存していない文も、分割の要求に入って届く。トランスクルージョンは元の項目の本文を見せ、打った文字は保存されるまで自分で持つ。
#[test]
fn text_typed_into_a_transclusion_and_not_saved_yet_travels_in_the_split() {
    let mut h = transclusion_harness(Vec::new());
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "shown typed");
    enter(&mut h, "root", "shown ty".len());
    assert_eq!(
        split_calls(&h.api),
        [("root".to_string(), "shown typed".to_string(), 8)]
    );
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
}

// トランスクルージョンの分割に失敗すると、要求に入っていた未保存の入力は、もう一度自動保存にかけられ、元の項目に保存される。
#[test]
fn after_a_failed_split_the_text_typed_into_a_transclusion_is_auto_saved_again() {
    let mut h = transclusion_harness(Vec::new());
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "shown typed");
    h.api.fail_next("SplitItem", "offline");
    enter(&mut h, "root", 8);
    h.fire_timers();
    assert_eq!(
        update_calls(&h.api),
        [("s".into(), Some("shown typed".into()))]
    );
}

// 元の項目が消えた（壊れた）トランスクルージョンでの Enter も、分割としてサーバーに送る。どう分けるかは、サーバーが今のデータで決める。
#[test]
fn enter_in_a_broken_transclusion_is_sent_to_the_server_as_a_split() {
    let broken = item_with("root", 1, |i| {
        i.transclude_item_id = Some("gone".into());
        i.body = "shown".into();
    });
    let mut h = Harness::with_items(vec![item("a", 1, ""), broken]);
    enter(&mut h, "root", 0);
    assert_eq!(
        split_calls(&h.api),
        [("root".to_string(), "shown".to_string(), 0)]
    );
}

// トランスクルージョンの中での分割に失敗すると、画面は変わらず、ページも読み直さない。キーの保留は終わる。
#[test]
fn a_split_inside_a_transclusion_that_fails_changes_nothing_and_stops_holding_keys() {
    let mut h = transclusion_harness(Vec::new());
    h.api.fail_next("SplitItem", "offline");
    enter(&mut h, "c", 2);
    assert!(!h.model.key_relay.is_holding());
    assert_eq!(requests(&h), ["SplitItem"]);
    assert_eq!(h.items_ids(), ["a", "root"]);
}

// 次のエディタを待つ間に押されたキーは、そのエディタに渡される。待っていなければキーは捨てられる。
// The fake server answers at once, so the wait is started by hand and ended by a merge.
#[test]
fn keys_pressed_while_waiting_for_the_next_editor_are_played_into_it() {
    let held = |k: &str| crate::model::DeferredKey {
        key: k.into(),
        code: format!("Key{}", k.to_uppercase()),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
    };
    let mut h = harness();
    h.send(Event::KeyHeld(held("z")));
    h.model.key_relay.hold_until_focus();
    h.send(Event::KeyHeld(held("x")));
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });

    let replayed: Vec<Vec<String>> =
        h.ui.iter()
            .filter_map(|u| match u {
                crate::effects::UiOp::FocusEditor { replay_keys, .. } => {
                    Some(replay_keys.iter().map(|k| k.key.clone()).collect())
                }
                _ => None,
            })
            .collect();
    assert_eq!(replayed, [vec!["x".to_string()]]);
}

// 分割の答えの新しい項目の並び順の値が、上と下の項目の値の間にないときは、ページを読み直す。サーバーが周りの項目の値を振り直したためで、コアの持つ値が古い。
#[test]
fn a_split_whose_new_index_is_not_between_its_neighbours_reads_the_page_again() {
    let mut h = harness();
    // The server gave the item below a new index, and the new item one past the old "b"
    h.api.answer_next(
        "SplitItem",
        Ok(ApiOutput::Split(Box::new(Split {
            item: item("a", 1, "hello world"),
            created: item_with("new1", 1, |i| i.idx = "c".into()),
        }))),
    );

    enter(&mut h, "a", 11);

    assert!(h.api.called("GetPageById"));
}

// 並び順の値のための読み直しは、項目の並び順の値だけを受け取り、本文は変えない。分割の直後は新しい項目を編集中で、保存前の入力を上書きしないためである。
#[test]
fn a_page_read_again_for_its_indices_takes_only_the_indices() {
    let mut h = harness();
    h.api.answer_next(
        "SplitItem",
        Ok(ApiOutput::Split(Box::new(Split {
            item: item("a", 1, "hello world"),
            created: item_with("new1", 1, |i| i.idx = "c".into()),
        }))),
    );
    let server_has = |id: &str, idx: &str| item_with(id, 1, |i| i.idx = idx.into());
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(PageWithItems {
            page: Page {
                id: "page1".into(),
                ..Default::default()
            },
            items: vec![
                server_has("a", "a"),
                server_has("new1", "c"),
                server_has("b", "d"),
            ],
            ..Default::default()
        })),
    );

    enter(&mut h, "a", 11);

    assert_eq!(h.bodies(), ["a=hello world", "new1=", "b=body of b"]);
    let idx = |id: &str| h.model.state.get_item_by_id(id).unwrap().idx.clone();
    assert_eq!([idx("a"), idx("new1"), idx("b")], ["a", "c", "d"]);
}

// サーバーが分割で周りの項目の並び順の値を振り直しても、そのEnterのUndoとRedoは、新しい項目を元の位置に作り直す。
#[test]
fn undo_and_redo_of_an_enter_make_the_item_again_where_it_was_after_the_server_reindexed() {
    // b holds an index that sorts before the one the server gives the new item
    let mut h = Harness::with_items(vec![
        item("a", 1, "hello world"),
        item_with("b", 1, |i| {
            i.body = "body of b".into();
            i.idx = "a5".into();
        }),
    ]);
    h.model.state.add_editing_item("a");
    let at = |id: &str, idx: &str| item_with(id, 1, |i| i.idx = idx.into());
    h.api.answer_next(
        "SplitItem",
        Ok(ApiOutput::Split(Box::new(Split {
            item: item("a", 1, "hello world"),
            created: at("new1", "b"),
        }))),
    );
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(PageWithItems {
            page: Page {
                id: "page1".into(),
                ..Default::default()
            },
            items: vec![at("a", "a"), at("new1", "b"), at("b", "c")],
            ..Default::default()
        })),
    );
    enter(&mut h, "a", 11);

    h.send(Event::Undo);
    h.send(Event::Redo);

    let BatchOpInput::Create { position, .. } = &batch_calls(&h.api)[1][0] else {
        panic!("not a create: {:?}", batch_calls(&h.api));
    };
    assert_eq!(position, &crate::model::ItemPosition::after("a"));
    assert_eq!(h.items_ids(), ["a", "new1", "b"]);
}

// 分割の答えの新しい項目の並び順の値が、上と下の項目の値の間にあるときは、ページを読み直さない。
#[test]
fn a_split_whose_new_index_is_between_its_neighbours_does_not_read_the_page_again() {
    let mut h = harness();

    enter(&mut h, "a", 11);

    assert!(!h.api.called("GetPageById"));
}
