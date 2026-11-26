use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{item, move_calls, typed, update_calls, Harness};
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

// トランスクルージョンのルートに打って、まだ保存していない文は、インデントのあとも行に残り、元の項目に自動保存される（#734）。
#[test]
fn text_typed_into_a_transclusion_root_stays_through_an_indent_and_is_saved_to_its_source() {
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item_with("root", 1, |i| {
            i.transclude_item_id = Some("s".into());
            i.body = "shown".into();
        }),
    ]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "shown")]);
    h.model.state.set_transcluded_trees(trees);
    typed(&mut h, "root", "shown typed");

    h.send(Event::Indent {
        item_id: "root".into(),
    });

    assert_eq!(h.outline(), ["a:1=", "root:2=shown typed"]);
    h.fire_timers();
    assert_eq!(
        update_calls(&h.api),
        [("s".into(), Some("shown typed".into()))]
    );
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

/// A move request an outdent sent.
#[derive(Debug, PartialEq)]
struct OutdentMove {
    item_ids: Vec<String>,
    position: String,
    level_delta: Option<i32>,
    body: Option<String>,
}

fn outdent_moves(h: &Harness) -> Vec<OutdentMove> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::MoveItems {
                item_ids,
                position,
                level_delta,
                body,
                ..
            } => Some(OutdentMove {
                item_ids: item_ids.clone(),
                position: position.to_string(),
                level_delta: *level_delta,
                body: body.clone(),
            }),
            _ => None,
        })
        .collect()
}

// 論理アウトデントは、親の最後の子の後へ1段浅くする移動を1つの要求で送り、後続の兄弟 d は親 p の下に残る。階層の変更は別には送らない。答えが来ると、階層と位置が一緒に変わり、エディタのフォーカスは保たれ、Undo に「outdent」として記録される。
#[test]
fn logical_outdent_moves_the_group_one_level_up_after_the_parents_last_child_in_one_request() {
    let mut h = outdent_harness();
    h.model.config.logical_outdent = true;
    h.send(Event::Outdent {
        item_id: "b".into(),
    });
    assert!(shift_calls(&h).is_empty());
    assert_eq!(
        outdent_moves(&h),
        [OutdentMove {
            item_ids: vec!["b".to_string(), "c".to_string()],
            position: "after:d".to_string(),
            level_delta: Some(-1),
            body: None,
        }]
    );
    assert_eq!(h.outline(), ["p:1=", "a:2=", "d:2=", "b:1=", "c:2="]);
    assert_eq!(h.focused_editor_requests(), [Some("b".to_string())]);
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "outdent");
}

// 入力した直後の論理アウトデントでは、打ったばかりの文は移動の要求に入って届く。保存を別には送らず、その自動保存は取り消される。画面には、サーバーが保存した本文が出る。
#[test]
fn text_typed_just_before_a_logical_outdent_travels_in_its_move() {
    let mut h = outdent_harness();
    h.model.config.logical_outdent = true;
    typed(&mut h, "b", "b typed");
    h.send(Event::Outdent {
        item_id: "b".into(),
    });
    assert_eq!(outdent_moves(&h)[0].body, Some("b typed".to_string()));
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
    assert_eq!(h.outline(), ["p:1=", "a:2=", "d:2=", "b:1=b typed", "c:2="]);
}

// 論理アウトデントが断られると、画面は変わらず、要求に入れた文はもう一度自動保存にかけられる。キーの保留は終わる。
#[test]
fn after_a_refused_logical_outdent_nothing_changes_and_the_typed_text_is_auto_saved_again() {
    let mut h = outdent_harness();
    h.model.config.logical_outdent = true;
    typed(&mut h, "b", "b typed");
    h.api.fail_next("MoveItems", "offline");
    h.send(Event::Outdent {
        item_id: "b".into(),
    });
    assert_eq!(h.outline(), ["p:1=", "a:2=", "b:2=b typed", "c:3=", "d:2="]);
    assert!(h.model.undo.pop_undo().is_none_or(|e| e.label != "outdent"));
    assert!(!h.model.key_relay.is_holding());
    assert!(!h.api.called("GetPageById"));
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("b".into(), Some("b typed".into()))]);
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

// 深い子の後ろにある項目も、同じ段の項目が上にあれば、その最後の子として1段深くできる。
#[test]
fn an_item_after_a_deep_subtree_can_be_indented_under_the_root_above() {
    // * a (1)
    //     * b (2)
    //         * c (3)
    // * d (1)  ← indent
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item("b", 2, ""),
        item("c", 3, ""),
        item("d", 1, ""),
    ]);
    h.send(Event::Indent {
        item_id: "d".into(),
    });
    assert_eq!(h.outline(), ["a:1=", "b:2=", "c:3=", "d:2="]);
}
