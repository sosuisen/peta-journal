use crate::app::Event;
use crate::harness::{create_calls, item, update_calls, Harness};
use crate::model::{Page, PageWithItems, TranscludedTrees};
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

#[test]
fn enter_in_the_middle_keeps_the_text_before_the_cursor_and_moves_the_rest_below() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    assert_eq!(h.bodies(), ["a=hello", "new1= world", "b=body of b"]);
    assert_eq!(
        create_calls(&h.api),
        [("page1".into(), "after:a".into(), 1, " world".into(), None)]
    );
    assert!(h.model.state.is_editing("new1"));
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(h.focused_editor_requests(), [Some("new1".to_string())]);
    assert_eq!(h.focus_cursor(), Some(0));
}

#[test]
fn enter_at_the_end_shows_an_empty_item_at_once_and_creates_it_in_the_background() {
    let mut h = harness();
    enter(&mut h, "a", "hello world".len());
    let ids = h.items_ids();
    assert_eq!(ids.len(), 3);
    assert_eq!(ids[0], "a");
    assert_eq!(ids[2], "b");
    let new_id = ids[1].clone();
    assert_ne!(new_id, "new1", "the client made the id");
    let calls = create_calls(&h.api);
    assert_eq!(
        calls,
        [(
            "page1".into(),
            "after:a".into(),
            1,
            "".into(),
            Some(new_id.clone())
        )]
    );
    assert_eq!(
        h.model.state.get_item_by_id(&new_id).unwrap().idx,
        "created1"
    );
    assert_eq!(h.model.state.focused_item_id(), Some(new_id.as_str()));
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("hello world".into()))]
    );
}

#[test]
fn when_the_background_creation_fails_the_page_is_reloaded_from_the_server() {
    let mut h = harness();
    h.api.fail_next("CreateItem", "offline");
    enter(&mut h, "a", "hello world".len());
    assert!(h.api.called("GetPageById"));
    assert_eq!(h.bodies(), ["reloaded=reloaded"]);
}

#[test]
fn enter_at_the_start_puts_an_empty_item_above_and_keeps_editing_the_new_one() {
    let mut h = harness();
    enter(&mut h, "a", 0);
    assert_eq!(h.bodies(), ["new1=", "a=hello world", "b=body of b"]);
    assert_eq!(
        create_calls(&h.api),
        [("page1".into(), "first".into(), 1, "".into(), None)]
    );
    assert_eq!(h.focused_editor_requests(), [Some("new1".to_string())]);
}

#[test]
fn a_new_item_after_an_expanded_parent_is_created_as_its_first_child() {
    let mut h = Harness::with_items(vec![item("a", 1, "parent"), item("c", 2, "")]);
    enter(&mut h, "a", "parent".len());
    let calls = create_calls(&h.api);
    assert_eq!(calls[0].1, "after:a");
    assert_eq!(calls[0].2, 2);
}

#[test]
fn enter_on_an_empty_last_child_outdents_it_instead() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("e", 2, "")]);
    enter(&mut h, "e", 0);
    assert!(!h.api.called("CreateItem"));
    assert!(h.api.called("ShiftItemLevels"));
    assert_eq!(h.model.state.get_item_by_id("e").unwrap().level, 1);
    assert_eq!(h.focused_editor_requests(), [Some("e".to_string())]);
}

#[test]
fn holds_the_keys_pressed_until_the_outline_focuses_the_next_editor() {
    let mut h = harness();
    enter(&mut h, "a", 5);
    // The focus request released the held keys
    assert!(!h.model.key_relay.is_holding());
    assert!(!h.focused_editor_requests().is_empty());
}

#[test]
fn stops_holding_keys_when_the_split_fails() {
    let mut h = harness();
    h.api.fail_next("CreateItem", "offline");
    enter(&mut h, "a", 5);
    assert!(!h.model.key_relay.is_holding());
    assert_eq!(
        h.model.edits.edit_handovers, 0,
        "the handover ended despite the failure"
    );
}

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
