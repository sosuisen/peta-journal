use crate::app::Event;
use crate::harness::{item, merge_calls, Harness};
use crate::outline::UndoOp;
use crate::testing::item_with;

fn harness() -> Harness {
    Harness::with_items(vec![
        item("a", 1, "hello"),
        item("b", 1, " world"),
        item("c", 1, "!"),
    ])
}

// 行頭での Backspace でテキストが上の項目に連結され、その項目は削除されて上の項目の編集が続く。
// サーバーには結合を1回だけ送る。本文の更新と削除を別々には送らない。
#[test]
fn backspace_appends_the_text_to_the_item_above_deletes_the_item_and_keeps_editing_above() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert_eq!(h.bodies(), ["a=hello world", "c=!"]);
    assert_eq!(merge_calls(&h.api), [("b".to_string(), "a".to_string())]);
    assert!(!h.api.called("UpdateItem"));
    assert!(!h.api.called("DeleteItem"));
    assert!(h.model.state.is_editing("a"));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}

// 行頭での Backspace は結合位置にカーソルを戻す一つの Undo エントリとして記録される。
#[test]
fn backspace_records_one_undo_entry_that_puts_the_cursor_at_the_join() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "backspace");
    assert!(matches!(
        entry.ops.as_slice(),
        [UndoOp::Update { .. }, UndoOp::Delete { .. }]
    ));
    assert_eq!(entry.cursor_after.unwrap().position, "hello".len());
    assert!(h.model.undo.pop_undo().is_none());
}

// サーバーが結合を断ると、ページはサーバーの持つ形に読み直される。
#[test]
fn a_merge_the_server_refuses_reloads_the_page() {
    let mut h = harness();
    h.api
        .fail_next("MergeItems", "A transclusion cannot be merged");
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert!(h.api.called("GetPageById"));
    assert_eq!(h.bodies(), ["reloaded=reloaded"]);
}

// 先頭の項目では行頭での Backspace が何もしない。
#[test]
fn backspace_does_nothing_for_the_first_item() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart {
        item_id: "a".into(),
    });
    assert_eq!(h.bodies().len(), 3);
    assert!(h.api.calls.is_empty());
}

// トランスクルージョン項目は行頭での Backspace で結合されて消えることがない。
#[test]
fn backspace_does_not_merge_a_transclusion_item_away() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "a"),
        item_with("t", 1, |i| i.transclude_item_id = Some("src".into())),
    ]);
    h.send(Event::BackspaceAtStart {
        item_id: "t".into(),
    });
    assert_eq!(h.items_ids(), ["a", "t"]);
}

// ジャーナルの日の先頭項目は行頭での Backspace で上の日に結合されない。
#[test]
fn backspace_does_not_merge_the_first_item_of_a_journal_day_into_the_day_above() {
    let day = |id: &str, page: &str, name: &str| {
        item_with(id, 1, |i| {
            i.page_id = page.into();
            i.name = name.into();
            i.item_type = "journal".into();
            i.body = id.into();
        })
    };
    let mut h = Harness::with_items(vec![
        day("x", "p1", "2026-02-10"),
        day("y", "p2", "2026-02-09"),
    ]);
    h.send(Event::BackspaceAtStart {
        item_id: "y".into(),
    });
    assert_eq!(h.items_ids(), ["x", "y"]);
}

// 行末での Delete で下の項目が連結されて削除され、結合位置にカーソルを戻す Undo が記録される。
// サーバーには、下の項目をこの項目に結合する1回の呼び出しだけを送る。
#[test]
fn delete_appends_the_item_below_and_deletes_it() {
    let mut h = harness();
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    assert_eq!(h.bodies(), ["a=hello world", "c=!"]);
    assert_eq!(merge_calls(&h.api), [("b".to_string(), "a".to_string())]);
    assert!(!h.api.called("UpdateItem"));
    assert!(!h.api.called("DeleteItem"));
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "delete");
    assert_eq!(entry.cursor_after.unwrap().position, "hello".len());
}

// サーバーが行末での Delete の結合を断ると、ページは読み直される。
#[test]
fn a_delete_merge_the_server_refuses_reloads_the_page() {
    let mut h = harness();
    h.api.fail_next("MergeItems", "refused");
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    assert!(h.api.called("GetPageById"));
}

// 最後の項目では行末での Delete が何もしない。
#[test]
fn delete_does_nothing_for_the_last_item() {
    let mut h = harness();
    h.send(Event::DeleteAtEnd {
        item_id: "c".into(),
    });
    assert_eq!(h.bodies().len(), 3);
}
