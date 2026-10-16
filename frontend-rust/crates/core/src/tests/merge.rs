use crate::app::Event;
use crate::harness::{item, update_calls, Harness};
use crate::outline::UndoOp;
use crate::testing::item_with;

fn harness() -> Harness {
    Harness::with_items(vec![item("a", 1, "hello"), item("b", 1, " world"), item("c", 1, "!")])
}

#[test]
fn backspace_appends_the_text_to_the_item_above_deletes_the_item_and_keeps_editing_above() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart { item_id: "b".into() });
    assert_eq!(h.bodies(), ["a=hello world", "c=!"]);
    assert!(update_calls(&h.api).contains(&("a".to_string(), Some("hello world".to_string()))));
    assert!(h.api.called("DeleteItem"));
    assert!(h.model.state.is_editing("a"));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}

#[test]
fn backspace_records_one_undo_entry_that_puts_the_cursor_at_the_join() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart { item_id: "b".into() });
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "backspace");
    assert!(matches!(entry.ops.as_slice(), [UndoOp::Update { .. }, UndoOp::Delete { .. }]));
    assert_eq!(entry.cursor_after.unwrap().position, "hello".len());
    assert!(h.model.undo.pop_undo().is_none());
}

#[test]
fn backspace_does_nothing_for_the_first_item() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart { item_id: "a".into() });
    assert_eq!(h.bodies().len(), 3);
    assert!(h.api.calls.is_empty());
}

#[test]
fn backspace_does_not_merge_a_transclusion_item_away() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "a"),
        item_with("t", 1, |i| i.transclude_item_id = Some("src".into())),
    ]);
    h.send(Event::BackspaceAtStart { item_id: "t".into() });
    assert_eq!(h.items_ids(), ["a", "t"]);
}

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
    let mut h = Harness::with_items(vec![day("x", "p1", "2026-02-10"), day("y", "p2", "2026-02-09")]);
    h.send(Event::BackspaceAtStart { item_id: "y".into() });
    assert_eq!(h.items_ids(), ["x", "y"]);
}

#[test]
fn delete_appends_the_item_below_and_deletes_it() {
    let mut h = harness();
    h.send(Event::DeleteAtEnd { item_id: "a".into() });
    assert_eq!(h.bodies(), ["a=hello world", "c=!"]);
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "delete");
    assert_eq!(entry.cursor_after.unwrap().position, "hello".len());
}

#[test]
fn delete_does_nothing_for_the_last_item() {
    let mut h = harness();
    h.send(Event::DeleteAtEnd { item_id: "c".into() });
    assert_eq!(h.bodies().len(), 3);
}
