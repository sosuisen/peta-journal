use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{create_calls, item, Harness};
use crate::outline::UndoOp;

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

fn parse_calls(h: &Harness) -> Vec<String> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::ParseFlatMarkdown { markdown } => Some(markdown.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn one_piece_goes_into_the_item_at_the_cursor() {
    let mut h = harness();
    paste(&mut h, "a", "XY", 2);
    assert_eq!(h.bodies(), ["a=abXY|cd", "z=body of z"]);
    assert!(h.model.edits.dirty_item_ids.contains("a"));
    assert_eq!(h.focused_editor_requests(), [Some("a".into())]);
    assert_eq!(h.focus_cursor(), Some(4));
}

#[test]
fn several_pieces_become_items_below_the_last_one_taking_the_text_after_the_cursor() {
    let mut h = harness();
    paste(&mut h, "a", "one\n  two\nthree", 2);
    assert_eq!(
        h.outline(),
        [
            "a:1=abone",
            "new1:2=two",
            "new2:1=three|cd",
            "z:1=body of z"
        ]
    );
    let calls = create_calls(&h.api);
    assert_eq!(
        calls[0],
        ("page1".into(), "after:a".into(), 2, "two".into(), None)
    );
    assert_eq!(
        calls[1],
        (
            "page1".into(),
            "after:new1".into(),
            1,
            "three|cd".into(),
            None
        )
    );
    // The cursor goes to the last item, right after the pasted text
    assert_eq!(h.focused_editor_requests(), [Some("new2".into())]);
    assert_eq!(h.focus_cursor(), Some(5));
}

#[test]
fn records_one_undo_entry_with_the_update_and_the_created_items() {
    let mut h = harness();
    paste(&mut h, "a", "one\ntwo", 2);
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "paste");
    assert!(matches!(
        entry.ops.as_slice(),
        [UndoOp::Update { .. }, UndoOp::Create { .. }]
    ));
    assert_eq!(entry.cursor_before.unwrap().position, 2);
}

#[test]
fn text_copied_from_this_app_is_pasted_from_the_stored_flat_markdown() {
    let mut h = harness();
    h.model.clipboard = Some(("flat".into(), "- tree".into()));
    paste(&mut h, "a", "- tree", 5);
    assert_eq!(parse_calls(&h), ["flat"]);
    assert!(!h.api.called("ImportTreeMarkdown"));
}

#[test]
fn tree_markdown_from_elsewhere_is_converted_by_the_server_first() {
    let mut h = harness();
    paste(&mut h, "a", "- x\n  - y", 5);
    assert!(h.api.called("ImportTreeMarkdown"));
    assert_eq!(parse_calls(&h), ["from tree"]);
}

#[test]
fn plain_text_inside_an_open_code_fence_is_not_split() {
    let mut h = Harness::with_items(vec![item("code", 1, "```\nlet")]);
    paste(&mut h, "code", "a\nb", 7);
    assert!(!h.api.called("ParseFlatMarkdown"));
    // The save closes the fence that is still open
    assert_eq!(
        h.model.state.get_item_by_id("code").unwrap().body,
        "```\nleta\nb\n```"
    );
}

#[test]
fn raw_text_goes_into_the_item_at_the_cursor_without_a_split() {
    let mut h = harness();
    paste_raw(
        &mut h,
        "a",
        "one
  two
three",
        2,
    );
    assert!(!h.api.called("ParseFlatMarkdown"));
    assert_eq!(
        h.outline(),
        [
            "a:1=abone
  two
three|cd",
            "z:1=body of z"
        ]
    );
    assert_eq!(
        h.focus_cursor(),
        Some(
            2 + "one
  two
three"
                .len()
        )
    );
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "paste");
}

#[test]
fn raw_tree_markdown_and_stored_flat_markdown_are_not_converted() {
    let mut h = harness();
    h.model.clipboard = Some(("flat".into(), "- tree".into()));
    paste_raw(&mut h, "a", "- tree", 2);
    paste_raw(
        &mut h,
        "z",
        "- x
  - y",
        0,
    );
    assert!(!h.api.called("ImportTreeMarkdown"));
    assert!(!h.api.called("ParseFlatMarkdown"));
    assert_eq!(
        h.bodies(),
        [
            "a=ab- tree|cd",
            "z=- x
  - ybody of z"
        ]
    );
}

#[test]
fn a_failed_creation_drops_the_undo_entry() {
    let mut h = harness();
    h.api.fail_next("CreateItem", "offline");
    paste(&mut h, "a", "one\ntwo", 2);
    assert!(!h.model.undo.can_undo());
}

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
