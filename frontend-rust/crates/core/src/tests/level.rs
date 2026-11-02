use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{item, move_calls, Harness};
use crate::model::TranscludedTrees;
use crate::testing::item_with;

fn shift_calls(h: &Harness) -> Vec<(Vec<String>, i32)> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::ShiftItemLevels { item_ids, delta } => Some((item_ids.clone(), *delta)),
            _ => None,
        })
        .collect()
}

// インデントで項目が子ごと1段深くなり、一つの相対シフトとして保存されて Undo に記録される。
#[test]
fn indent_moves_the_item_with_its_children_one_level_deeper_and_saves_one_relative_shift() {
    // * a (1)
    // * b (1)  ← indent
    //     * c (2)
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, ""), item("c", 2, "")]);
    h.send(Event::Indent {
        item_id: "b".into(),
    });
    assert_eq!(h.outline(), ["a:1=", "b:2=", "c:3="]);
    assert_eq!(
        shift_calls(&h),
        [(vec!["b".to_string(), "c".to_string()], 1)]
    );
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "indent");
}

// 先頭の項目ではインデントが何もしない。
#[test]
fn indent_does_nothing_for_the_first_item() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::Indent {
        item_id: "a".into(),
    });
    assert_eq!(h.outline(), ["a:1=", "b:1="]);
    assert!(h.api.calls.is_empty());
}

// インデントの保存が失敗するとページが再読み込みされる。
#[test]
fn indent_reloads_the_pages_when_the_save_fails() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.api.fail_next("ShiftItemLevels", "offline");
    h.send(Event::Indent {
        item_id: "b".into(),
    });
    assert!(h.api.called("GetPageById"));
}

fn outdent_harness() -> Harness {
    // * p (1)
    //     * a (2)
    //     * b (2)  ← outdent
    //         * c (3)
    //     * d (2)
    Harness::with_items(vec![
        item("p", 1, ""),
        item("a", 2, ""),
        item("b", 2, ""),
        item("c", 3, ""),
        item("d", 2, ""),
    ])
}

// その場でのアウトデントでグループが1段浅くなり、位置は変わらない。
#[test]
fn outdent_in_place_the_group_goes_up_one_level_and_keeps_its_place() {
    let mut h = outdent_harness();
    h.send(Event::Outdent {
        item_id: "b".into(),
    });
    assert_eq!(h.outline(), ["p:1=", "a:2=", "b:1=", "c:2=", "d:2="]);
    assert_eq!(
        shift_calls(&h),
        [(vec!["b".to_string(), "c".to_string()], -1)]
    );
    assert!(move_calls(&h.api).is_empty());
}

// 論理アウトデントでグループは親の最後の子の後へ移動され、後続の兄弟 d は親 p の下に残る。
#[test]
fn logical_outdent_moves_the_group_after_the_parents_last_child_so_d_stays_under_p() {
    let mut h = outdent_harness();
    h.model.config.logical_outdent = true;
    h.send(Event::Outdent {
        item_id: "b".into(),
    });
    assert_eq!(
        shift_calls(&h),
        [(vec!["b".to_string(), "c".to_string()], -1)]
    );
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["b".to_string(), "c".to_string()],
            "page1".to_string(),
            "after:d".to_string()
        )]
    );
    let ids = h.items_ids();
    assert_eq!(ids, ["p", "a", "d", "b", "c"]);
}

// 一人っ子の論理アウトデントでは移動が不要である。
#[test]
fn logical_outdent_an_only_child_needs_no_move() {
    let mut h = Harness::with_items(vec![item("p", 1, ""), item("x", 2, ""), item("b", 3, "")]);
    h.model.config.logical_outdent = true;
    h.send(Event::Outdent {
        item_id: "b".into(),
    });
    assert!(move_calls(&h.api).is_empty());
    assert_eq!(h.outline(), ["p:1=", "x:2=", "b:2="]);
}

// 最上位レベルではアウトデントが何もしない。
#[test]
fn outdent_does_nothing_at_the_top_level() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::Outdent {
        item_id: "a".into(),
    });
    assert!(h.api.calls.is_empty());
}

fn transclusion_harness() -> Harness {
    // own: root (2, transcludes s); source: s (1), c1 (2), c2 (3)
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item_with("root", 2, |i| i.transclude_item_id = Some("s".into())),
    ]);
    let mut trees = TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![item("s", 1, ""), item("c1", 2, ""), item("c2", 3, "")],
    );
    h.model.state.set_transcluded_trees(trees);
    h
}

// トランスクルードされた子孫はルートと同じレベルまでアウトデントできない。
#[test]
fn a_transcluded_descendant_cannot_be_outdented_onto_the_roots_level() {
    let mut h = transclusion_harness();
    // c1 is shown at level 3, right under the root at level 2
    h.send(Event::Outdent {
        item_id: "c1".into(),
    });
    assert!(h.api.calls.is_empty());
}

// より深い子孫はアウトデントされ、ツリーのレベルオフセットを差し引いたレベルで保存される。
#[test]
fn a_deeper_descendant_is_outdented_and_stored_with_the_trees_level_offset() {
    let mut h = transclusion_harness();
    // c2 is shown at level 4; the tree's offset is 1
    h.send(Event::Outdent {
        item_id: "c2".into(),
    });
    assert_eq!(shift_calls(&h), [(vec!["c2".to_string()], -1)]);
    assert_eq!(h.model.state.get_item_by_id("c2").unwrap().level, 2);
}
