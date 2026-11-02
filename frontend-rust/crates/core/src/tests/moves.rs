use crate::app::Event;
use crate::harness::{item, move_calls, Harness};
use crate::model::TranscludedTrees;
use crate::testing::item_with;
use std::collections::HashSet;

fn page_harness() -> Harness {
    // * a (1)
    // * b (1)
    //     * b1 (2)
    // * c (1)
    Harness::with_items(vec![
        item("a", 1, ""),
        item("b", 1, ""),
        item("b1", 2, ""),
        item("c", 1, ""),
    ])
}

// 項目は子と一緒に上の兄弟を越えて移動し、そのエディタのフォーカスは保たれる。移動は undo の対象として記録される。
#[test]
fn moves_the_item_with_its_children_past_the_sibling_above_and_keeps_its_editor() {
    let mut h = page_harness();
    h.send(Event::MoveUp {
        item_id: "b".into(),
    });
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["b".to_string(), "b1".to_string()],
            "page1".to_string(),
            "first".to_string()
        )]
    );
    assert_eq!(h.items_ids(), ["b", "b1", "a", "c"]);
    assert_eq!(h.focused_editor_requests(), [Some("b".to_string())]);
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "move");
    assert!(!h.model.move_in_progress);
}

// 項目は子と一緒に下の兄弟を越えて移動される。
#[test]
fn moves_the_item_past_the_sibling_below() {
    let mut h = page_harness();
    h.send(Event::MoveDown {
        item_id: "b".into(),
    });
    assert_eq!(move_calls(&h.api)[0].2, "after:c");
    assert_eq!(h.items_ids(), ["a", "c", "b", "b1"]);
}

// 先頭の項目の上移動と末尾の項目の下移動では何も起きない。
#[test]
fn does_nothing_at_the_top_or_bottom() {
    let mut h = page_harness();
    h.send(Event::MoveUp {
        item_id: "a".into(),
    });
    h.send(Event::MoveDown {
        item_id: "c".into(),
    });
    assert!(move_calls(&h.api).is_empty());
    assert!(!h.model.move_in_progress);
}

// 連続した移動は一つずつ順に実行される。
#[test]
fn moves_run_one_after_another() {
    let mut h = page_harness();
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    assert_eq!(h.items_ids(), ["b", "b1", "c", "a"]);
    assert_eq!(move_calls(&h.api).len(), 2);
}

// トランスクルージョンの中では、子孫の項目は参照元ツリーの中で移動し、ページが読み直される。
#[test]
fn inside_a_transclusion_a_descendant_moves_within_the_source_tree_and_the_page_reloads() {
    // own: root (1, transcludes s); source: s (1), c1 (2), c2 (2)
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![item("s", 1, ""), item("c1", 2, ""), item("c2", 2, "")],
    );
    h.model.state.set_transcluded_trees(trees);
    h.send(Event::MoveUp {
        item_id: "c2".into(),
    });
    // "first" in the scoped list means right after the source root
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["c2".to_string()],
            "page1".to_string(),
            "after:s".to_string()
        )]
    );
    assert!(h.api.called("GetPageById"));
}

// 選択移動では、選択されたサブツリーが一つのまとまりとして端のルートの兄弟を越えて移動する。
#[test]
fn move_selection_moves_the_selected_subtrees_as_one_group_past_the_sibling_of_the_edge_root() {
    let mut h = page_harness();
    let selected: HashSet<String> = ["b", "b1", "c"].iter().map(|s| s.to_string()).collect();
    h.model.state.set_selected_item_ids(selected);
    h.model.state.set_item_selection_mode(true);
    h.send(Event::SelectionKey(crate::app::SelectionKey::MoveUp));
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["b".to_string(), "b1".to_string(), "c".to_string()],
            "page1".to_string(),
            "first".to_string()
        )]
    );
    assert_eq!(h.items_ids(), ["b", "b1", "c", "a"]);
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "move-selection");
}
