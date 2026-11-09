use crate::app::Event;
use crate::effects::ApiOutput;
use crate::harness::{batch_calls, create_calls, item, split_calls, update_calls, Harness};
use crate::model::{BatchOp, Page, PageWithItems, Split, TranscludedTrees};
use crate::testing::item_with;

fn harness() -> Harness {
    let mut h = Harness::with_items(vec![item("a", 1, "hello world"), item("b", 1, "body of b")]);
    h.model.state.add_editing_item("a");
    h
}

fn typed(h: &mut Harness, item_id: &str, body: &str) {
    let now = h.now_ms;
    h.send(Event::BodyChanged {
        item_id: item_id.into(),
        body: body.into(),
        now_ms: now,
    });
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
    h.model.state.toggle_collapsed("a");
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
        matches!(&batch[0], BatchOp::Update { id, body, .. } if id == "a" && body.as_deref() == Some("hello world")),
        "{batch:?}"
    );
    assert_eq!(batch[1], BatchOp::Delete { id: "new1".into() });
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
        [[BatchOp::Delete { id: "new1".into() }]]
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

// 元の項目は、サーバーが解析した結果で表示される。分割で本文が変わるためである。
#[test]
fn the_item_that_was_split_shows_what_the_server_parsed_from_the_text_it_keeps() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().ast, "ast:hello");
}

// トランスクルージョンの中では、新しい項目は参照元ページに作られ、ページが読み直される。
#[test]
fn inside_a_transclusion_the_new_item_is_created_on_the_source_page_and_the_page_reloads() {
    let root = item_with("root", 2, |i| {
        i.transclude_item_id = Some("s".into());
        i.body = "shown".into();
    });
    let mut h = Harness::with_items(vec![item("a", 1, ""), root]);
    let mut trees = TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![item_with("s", 1, |i| {
            i.page_id = "src".into();
            i.body = "shown".into();
        })],
    );
    h.model.state.set_transcluded_trees(trees);
    h.api.answer_next(
        "GetPageById",
        Ok(crate::effects::ApiOutput::Page(PageWithItems {
            page: Page {
                id: "page1".into(),
                ..Default::default()
            },
            items: vec![item("a", 1, ""), item("root", 2, "")],
            ..Default::default()
        })),
    );
    enter(&mut h, "root", "sho".len());
    assert!(update_calls(&h.api).contains(&("s".to_string(), Some("sho".to_string()))));
    assert_eq!(
        create_calls(&h.api),
        [("src".into(), "after:s".into(), 1, "wn".into(), None)]
    );
    assert!(h.api.called("GetPageById"));
}

/// A host page with "a", then the transclusion root "root" of source item "s", whose child
/// "c" is the last transcluded item.
fn transclusion_harness(own_after_root: Vec<crate::model::Item>) -> Harness {
    let root = item_with("root", 1, |i| i.transclude_item_id = Some("s".into()));
    let mut own = vec![item("a", 1, ""), root];
    own.extend(own_after_root);
    let mut h = Harness::with_items(own);
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

// 最後のトランスクルード項目の末尾での Enter は、新しい項目をホストページのルートの直後に作る。
#[test]
fn enter_at_the_end_of_the_last_transcluded_item_creates_the_new_item_on_the_host_page() {
    let mut h = transclusion_harness(vec![item("b", 1, "")]);
    enter(&mut h, "c", "child".len());
    let calls = create_calls(&h.api);
    assert_eq!(calls.len(), 1);
    let (page_id, position, level, _, new_id) = &calls[0];
    assert_eq!(
        (page_id.as_str(), position.as_str(), *level),
        ("page1", "after:root", 1)
    );
    assert_eq!(h.items_ids(), ["a", "root", new_id.as_ref().unwrap(), "b"]);
}

// 最後のトランスクルード項目の末尾での Enter は、ルートに自前の子があればその子に加わる。
#[test]
fn enter_at_the_end_of_the_last_transcluded_item_joins_the_own_children_of_the_root() {
    let mut h = transclusion_harness(vec![item("own", 2, "")]);
    enter(&mut h, "c", "child".len());
    let calls = create_calls(&h.api);
    let (page_id, position, level, _, _) = &calls[0];
    assert_eq!(
        (page_id.as_str(), position.as_str(), *level),
        ("page1", "after:root", 2)
    );
}

// 下にまだトランスクルード項目があるとき、その項目の末尾での Enter は参照元ページに作る。
#[test]
fn enter_at_the_end_of_a_transcluded_item_with_one_below_still_creates_on_the_source_page() {
    let mut h = transclusion_harness(Vec::new());
    enter(&mut h, "root", "shown".len());
    let calls = create_calls(&h.api);
    assert_eq!(calls[0].0, "src");
}

// トランスクルージョンの中での分割に失敗すると、キーの保留は終わり、編集の引き継ぎも終わる。
#[test]
fn a_split_inside_a_transclusion_that_fails_stops_holding_keys() {
    let mut h = transclusion_harness(Vec::new());
    h.api.fail_next("CreateItem", "offline");
    enter(&mut h, "c", 2);
    assert!(!h.model.key_relay.is_holding());
    assert_eq!(
        h.model.edits.edit_handovers, 0,
        "the handover ended despite the failure"
    );
}

// 最後のトランスクルード項目の末尾での Enter は、空の項目をすぐに示す。その作成に失敗すると、ページはサーバーから読み直される。
#[test]
fn when_the_creation_below_a_transclusion_fails_the_page_is_reloaded_from_the_server() {
    let mut h = transclusion_harness(Vec::new());
    h.api.fail_next("CreateItem", "offline");
    enter(&mut h, "c", "child".len());
    assert!(h.api.called("GetPageById"));
    assert_eq!(h.bodies(), ["reloaded=reloaded"]);
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

// ルートの行頭での Enter は、ホストページのルートの直前に空の項目を作る。参照元ページには何も作らない。
#[test]
fn enter_at_the_start_of_a_transclusion_root_puts_the_empty_item_before_it_on_the_host_page() {
    let mut h = transclusion_harness(Vec::new());
    enter(&mut h, "root", 0);
    let calls = create_calls(&h.api);
    assert_eq!(calls.len(), 1);
    let (page_id, position, level, body, _) = &calls[0];
    assert_eq!(
        (page_id.as_str(), position.as_str(), *level, body.as_str()),
        ("page1", "after:a", 1, "")
    );
    assert_eq!(h.items_ids(), ["a", "new1", "root"]);
}

// 埋め込まれた子孫の行頭での Enter は、参照元ページのその項目の直前に空の項目を作り、ページを読み直す。
#[test]
fn enter_at_the_start_of_a_transcluded_descendant_puts_the_empty_item_before_it_on_the_source_page()
{
    let mut h = transclusion_harness(Vec::new());
    let mut trees = TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![
            item_with("s", 1, |i| i.page_id = "src".into()),
            item_with("c", 2, |i| i.page_id = "src".into()),
            item_with("d", 2, |i| {
                i.page_id = "src".into();
                i.body = "second".into();
            }),
        ],
    );
    h.model.state.set_transcluded_trees(trees);
    enter(&mut h, "d", 0);
    let calls = create_calls(&h.api);
    assert_eq!(calls.len(), 1);
    let (page_id, position, level, body, _) = &calls[0];
    assert_eq!(
        (page_id.as_str(), position.as_str(), *level, body.as_str()),
        ("src", "after:c", 2, "")
    );
    assert!(h.api.called("GetPageById"));
}

// 元の木の最初の子の行頭での Enter は、参照元ページの元の項目の直後に空の項目を作る。ルートのidは参照元ページにないからである。
#[test]
fn enter_at_the_start_of_the_first_transcluded_child_puts_the_empty_item_after_the_source_item() {
    let mut h = transclusion_harness(Vec::new());
    enter(&mut h, "c", 0);
    let calls = create_calls(&h.api);
    assert_eq!(calls.len(), 1);
    let (page_id, position, level, _, _) = &calls[0];
    assert_eq!(
        (page_id.as_str(), position.as_str(), *level),
        ("src", "after:s", 2)
    );
}

// 元の項目が消えた（壊れた）ルートの行頭での Enter も、ホストページのルートの直前に空の項目を作る。
#[test]
fn enter_at_the_start_of_a_broken_transclusion_root_puts_the_empty_item_before_it_on_the_host_page()
{
    let root = item_with("root", 1, |i| {
        i.transclude_item_id = Some("gone".into());
        i.body = "shown".into();
    });
    let mut h = Harness::with_items(vec![item("a", 1, ""), root]);
    enter(&mut h, "root", 0);
    let calls = create_calls(&h.api);
    let (page_id, position, level, _, _) = &calls[0];
    assert_eq!(
        (page_id.as_str(), position.as_str(), *level),
        ("page1", "after:a", 1)
    );
    assert_eq!(h.items_ids(), ["a", "new1", "root"]);
}
