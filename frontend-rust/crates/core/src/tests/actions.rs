use crate::app::Event;
use crate::effects::{ApiOp, UiOp};
use crate::harness::{create_calls, item, move_calls, update_calls, Harness};
use crate::testing::item_with;
use std::collections::HashSet;

#[test]
fn cycle_task_saves_the_next_task_state_and_records_it_for_undo() {
    let mut h = Harness::with_items(vec![item("a", 1, "do it")]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    assert!(h.api.called("CycleTask"));
    assert!(h.model.state.get_item_by_id("a").unwrap().is_task());
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "cycle-task");
}

#[test]
fn an_item_that_becomes_a_task_on_a_daily_journal_is_planned_for_that_day() {
    let mut h = Harness::with_items(vec![item_with("a", 1, |i| {
        i.item_type = "journal".into();
        i.name = "2026-02-10".into();
    })]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertItemProperty { key, value, .. } if key == "plan" && value == "2026-02-10")));
    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().prop_str("plan"),
        Some("2026-02-10")
    );
}

#[test]
fn set_zoom_shows_and_saves_the_factor() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::ZoomStep {
        item_id: "a".into(),
        up: true,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.1);
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertItemProperty { key, value, .. } if key == "zoom" && value == "1.1")));
    h.send(Event::ResetZoom {
        item_id: "a".into(),
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.0);
}

#[test]
fn toggle_collapsed_collapses_an_item_with_children_saves_it_and_records_it_for_undo() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 2, "")]);
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });
    assert!(h.model.state.is_collapsed("a"));
    assert_eq!(h.model.state.visible_items().len(), 1);
    assert!(h.api.calls.iter().any(|c| matches!(
        c,
        ApiOp::UpdateItem {
            collapsed: Some(true),
            ..
        }
    )));
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "collapse");
}

#[test]
fn toggle_collapsed_puts_the_screen_back_when_the_save_fails() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 2, "")]);
    h.api.fail_next("UpdateItem", "offline");
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });
    assert!(!h.model.state.is_collapsed("a"));
    assert_eq!(h.model.state.visible_items().len(), 2);
}

#[test]
fn toggle_collapsed_does_nothing_for_an_item_without_children() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });
    assert!(h.api.calls.is_empty());
}

#[test]
fn paste_as_transclusion_links_the_item_to_the_pasted_id_and_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.clipboard_text = "2026-01-01-00-00-00-AAAAAAA".into();
    h.send(Event::PasteAsTransclusion {
        item_id: "a".into(),
    });
    assert!(h.api.called("SetTransclusion"));
    assert!(h.api.called("GetPageById"));
}

#[test]
fn paste_as_transclusion_ignores_text_that_is_not_an_item_id() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.clipboard_text = "hello world".into();
    h.send(Event::PasteAsTransclusion {
        item_id: "a".into(),
    });
    assert!(!h.api.called("SetTransclusion"));
}

#[test]
fn archive_tags_the_item_creates_item_archive_at_the_end_and_moves_the_subtree_under_it() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "todo"),
        item("a1", 2, ""),
        item("z", 1, ""),
    ]);
    h.send(Event::SendToArchive {
        item_id: "a".into(),
    });
    assert!(update_calls(&h.api).contains(&("a".to_string(), Some("todo #archived".to_string()))));
    assert_eq!(
        create_calls(&h.api),
        [(
            "page1".into(),
            "after:z".into(),
            1,
            "#item-archive".into(),
            None
        )]
    );
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["a".to_string(), "a1".to_string()],
            "page1".to_string(),
            "after:new1".to_string()
        )]
    );
}

#[test]
fn archive_reuses_an_existing_item_archive_and_does_not_tag_twice() {
    let mut h = Harness::with_items(vec![
        item_with("a", 1, |i| {
            i.body = "todo #archived".into();
            i.properties
                .insert("tag".into(), serde_json::json!(["archived"]));
        }),
        item_with("arch", 1, |i| {
            i.body = "#item-archive".into();
            i.properties
                .insert("tag".into(), serde_json::json!(["item-archive"]));
        }),
    ]);
    h.send(Event::SendToArchive {
        item_id: "a".into(),
    });
    assert!(!h.api.called("UpdateItem"));
    assert!(!h.api.called("CreateItem"));
    assert_eq!(move_calls(&h.api)[0].2, "after:arch");
}

#[test]
fn deleting_the_selection_removes_the_items_as_one_undo_entry_and_keeps_one_item_on_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model
        .state
        .set_selected_item_ids(HashSet::from(["a".to_string(), "b".to_string()]));
    h.model.state.set_item_selection_mode(true);
    h.send(Event::SelectionKey(crate::app::SelectionKey::Delete));
    assert_eq!(h.api.calls_named("DeleteItem").len(), 2);
    assert!(!h.model.state.item_selection_mode());
    assert_eq!(
        h.items_ids(),
        ["new1"],
        "an empty item was created so the page is not empty"
    );
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "delete-selection");
}

#[test]
fn copying_the_selection_exports_tree_markdown_and_remembers_the_flat_form() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model
        .state
        .set_selected_item_ids(HashSet::from(["a".to_string()]));
    h.model.state.set_item_selection_mode(true);
    h.send(Event::SelectionKey(crate::app::SelectionKey::Copy));
    assert!(h.api.called("ExportTreeMarkdown"));
    assert!(h
        .ui
        .iter()
        .any(|u| matches!(u, UiOp::CopyMarkdown { markdown } if markdown == "- tree")));
    assert_eq!(h.model.clipboard, Some(("flat".into(), "- tree".into())));
}
