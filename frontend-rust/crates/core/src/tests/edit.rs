use crate::app::Event;
use crate::harness::{item, update_calls, Harness};
use crate::model::TranscludedTrees;
use crate::testing::item_with;

fn typed(h: &mut Harness, item_id: &str, body: &str) {
    let now = h.now_ms;
    h.send(Event::BodyChanged {
        item_id: item_id.into(),
        body: body.into(),
        now_ms: now,
    });
}

#[test]
fn typing_shows_the_text_at_once_and_schedules_the_auto_save() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hel");
    typed(&mut h, "a", "hello");
    assert_eq!(h.bodies(), ["a=hello"]);
    assert!(
        update_calls(&h.api).is_empty(),
        "nothing is sent before the pause"
    );
    assert_eq!(
        h.pending_timer_ids().len(),
        1,
        "the second keystroke replaced the first timer"
    );
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.model.edits.pending_saves.is_empty());
}

#[test]
fn a_transclusion_root_schedules_the_save_for_its_source_item() {
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "src")]);
    h.model.state.set_transcluded_trees(trees);
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "typed");
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("s".into(), Some("typed".into()))]);
}

#[test]
fn finish_edit_saves_typed_text_once_cancelling_the_auto_save_and_closes_the_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hello");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.timers.is_empty(), "the auto-save timer was cancelled");
    assert!(!h.model.state.is_editing("a"));
    h.fire_timers();
    assert_eq!(update_calls(&h.api).len(), 1, "the text is not sent twice");
}

#[test]
fn finish_edit_closes_the_editor_of_a_clean_item_without_a_save() {
    let mut h = Harness::with_items(vec![item("a", 1, "x")]);
    h.model.state.add_editing_item("a");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(h.api.calls.is_empty());
    assert!(!h.model.state.is_editing("a"));
}

#[test]
fn finish_edit_closes_an_open_code_fence_even_when_nothing_was_typed_since() {
    let mut h = Harness::with_items(vec![item("a", 1, "```js")]);
    h.model.state.add_editing_item("a");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("```js\n```".into()))]
    );
}

#[test]
fn finish_edit_is_ignored_while_a_handover_is_saving_the_item_itself() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    h.model.edits.edit_handovers = 1;
    typed(&mut h, "a", "hello");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(update_calls(&h.api).is_empty());
    assert!(h.model.state.is_editing("a"));
}

#[test]
fn clicking_another_item_saves_the_dirty_one_and_opens_the_clicked_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        cursor: 0,
    });
    typed(&mut h, "a", "typed");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        cursor: 0,
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some("typed".into()))]);
    assert!(!h.model.state.is_editing("a"));
    assert!(h.model.state.is_editing("b"));
    assert_eq!(h.model.edits.edit_handovers, 0);
}

#[test]
fn arrow_navigation_saves_and_moves_the_editor_with_the_cursor_column() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "x");
    h.send(Event::NavigateDown {
        item_id: "a".into(),
        cursor: 3,
    });
    assert!(h.model.state.is_editing("b"));
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(h.focus_cursor(), Some(3));
    assert_eq!(update_calls(&h.api), [("a".into(), Some("x".into()))]);
}

#[test]
fn a_file_change_waits_for_the_edit_to_end_and_then_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    h.send(Event::Sse(crate::effects::SseEvent::Message(
        r#"{"type":"pagesUpdated","pageId":"page1","source":"file"}"#.into(),
    )));
    assert!(!h.api.called("GetPageById"), "not while editing");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(
        !h.api.called("GetPageById"),
        "the auto-save gets a moment first"
    );
    h.fire_timers();
    assert!(h.api.called("GetPageById"));
}
