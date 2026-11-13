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

fn visible(h: &Harness) -> Vec<String> {
    h.model
        .state
        .visible_items()
        .iter()
        .map(|i| i.id.clone())
        .collect()
}

// トランスクルージョンの中の項目を折りたたむと、その子は隠れる。他の項目は見えたままである。
#[test]
fn collapsing_an_item_inside_a_transclusion_hides_its_children() {
    let root = crate::testing::item_with("root", 1, |i| i.transclude_item_id = Some("s".into()));
    let mut h = Harness::with_items(vec![root]);
    let mut trees = crate::model::TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![
            item("s", 1, ""),
            item("s1", 2, ""),
            item("s11", 3, ""),
            item("s2", 2, ""),
        ],
    );
    h.model.state.set_transcluded_trees(trees);
    assert_eq!(visible(&h), ["root", "s1", "s11", "s2"]);

    h.send(Event::CollapseToggle {
        item_id: "s1".into(),
    });
    assert_eq!(visible(&h), ["root", "s1", "s2"]);
}

fn parent_and_child() -> Harness {
    Harness::with_items(vec![item("a", 1, ""), item("b", 2, "")])
}

// 「隠す」キーは、開いている項目の子を隠す。もう隠れている項目では、何もしない。
#[test]
fn the_key_that_hides_the_children_never_shows_them() {
    let mut h = parent_and_child();
    h.send(Event::SetCollapsed {
        item_id: "a".into(),
        collapsed: true,
    });
    assert!(h.model.state.is_collapsed("a"));
    let saves = h.api.calls.len();

    h.send(Event::SetCollapsed {
        item_id: "a".into(),
        collapsed: true,
    });

    assert!(h.model.state.is_collapsed("a"));
    assert_eq!(h.api.calls.len(), saves);
}

// 「表示する」キーは、隠れている子を表示する。もう開いている項目では、何もしない。
#[test]
fn the_key_that_shows_the_children_never_hides_them() {
    let mut h = parent_and_child();
    h.send(Event::SetCollapsed {
        item_id: "a".into(),
        collapsed: false,
    });
    assert!(!h.model.state.is_collapsed("a"));
    assert!(h.api.calls.is_empty());
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });

    h.send(Event::SetCollapsed {
        item_id: "a".into(),
        collapsed: false,
    });

    assert!(!h.model.state.is_collapsed("a"));
    assert_eq!(h.model.state.visible_items().len(), 2);
}
