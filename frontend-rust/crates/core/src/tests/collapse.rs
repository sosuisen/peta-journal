//! Hiding and showing the children of an item with Ctrl+Up and Ctrl+Down.

use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{item, Harness};

// 子を持つ項目の折りたたみ切り替えで項目が折りたたまれて保存され、Undo に記録される。
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

// 折りたたみの保存が失敗すると画面が元の展開状態に戻される。
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

// 子を持たない項目では折りたたみ切り替えが何もしない。
#[test]
fn toggle_collapsed_does_nothing_for_an_item_without_children() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });
    assert!(h.api.calls.is_empty());
}
