use crate::app::Event;
use crate::effects::ApiOp;
use crate::harness::{item, move_calls, Harness};
use crate::model::{ItemPosition, TranscludedTrees};
use crate::testing::item_with;

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

/// A page whose only item "root" transcludes "s", which shows its children "c1" and "c2".
fn transclusion_harness() -> Harness {
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert(
        "s".into(),
        vec![item("s", 1, ""), item("c1", 2, ""), item("c2", 2, "")],
    );
    h.model.state.set_transcluded_trees(trees);
    h
}

// トランスクルージョンの中では、子孫の項目は参照元ツリーの中で移動し、ページが読み直される。
#[test]
fn inside_a_transclusion_a_descendant_moves_within_the_source_tree_and_the_page_reloads() {
    let mut h = transclusion_harness();
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

/// The body each move request carried, in the order they went.
fn carried_bodies(h: &Harness) -> Vec<Option<String>> {
    h.api
        .calls
        .iter()
        .filter_map(|op| match op {
            ApiOp::MoveItems { body, .. } => Some(body.clone()),
            _ => None,
        })
        .collect()
}

fn typed(h: &mut Harness, item_id: &str, body: &str) {
    h.model.state.add_editing_item(item_id);
    let now = h.now_ms;
    h.send(Event::BodyChanged {
        item_id: item_id.into(),
        body: body.into(),
        now_ms: now,
    });
}

// 入力した直後に動かすと、打ったばかりの文は移動の要求に入って届く。保存を別には送らず、その自動保存は取り消される。
#[test]
fn text_typed_just_before_a_move_travels_in_the_move_itself() {
    let mut h = page_harness();
    typed(&mut h, "a", "typed");
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    assert_eq!(carried_bodies(&h), [Some("typed".to_string())]);
    assert!(crate::harness::update_calls(&h.api).is_empty());
    h.fire_timers();
    assert!(crate::harness::update_calls(&h.api).is_empty());
    assert_eq!(h.items_ids(), ["b", "b1", "a", "c"]);
    assert_eq!(h.bodies()[2], "a=typed");
}

// 保存済みの項目を動かす要求は、本文を持たない。
#[test]
fn a_move_of_an_item_with_no_unsaved_text_carries_no_body() {
    let mut h = page_harness();
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    assert_eq!(carried_bodies(&h), [None]);
}

// サーバーが移動を断ると、画面は変わらず、要求に入っていた未保存の入力は、もう一度自動保存にかけられる。キーの保留は終わり、次の移動を始められる。
#[test]
fn after_a_failed_move_nothing_moves_and_the_typed_text_is_auto_saved_again() {
    let mut h = page_harness();
    typed(&mut h, "a", "typed");
    h.api.fail_next("MoveItems", "offline");
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    assert_eq!(h.items_ids(), ["a", "b", "b1", "c"]);
    assert!(h.model.undo.pop_undo().is_none_or(|e| e.label != "move"));
    assert!(!h.model.key_relay.is_holding());
    assert!(!h.model.move_in_progress);
    assert!(crate::harness::update_calls(&h.api).is_empty());
    h.fire_timers();
    assert_eq!(
        crate::harness::update_calls(&h.api),
        [("a".into(), Some("typed".into()))]
    );
}

// 答えが届いたときに動かした項目がもう画面に無ければ、何も表示しない。待っている間に別のページへ移った場合である。キーの保留は終わり、次の移動を始められる。
#[test]
fn the_answer_to_a_move_of_items_no_longer_shown_changes_nothing() {
    let mut h = Harness::with_items(vec![item("x", 1, "other page")]);
    h.model.key_relay.hold_until_focus();
    h.model.move_in_progress = true;
    h.send(Event::Moved {
        moved: vec![item("b", 1, ""), item("b1", 2, "")],
        position: ItemPosition::First,
        edited: Some("b".into()),
        label: "move",
    });
    assert_eq!(h.items_ids(), ["x"]);
    assert!(h.focused_editor_requests().is_empty());
    assert!(h.model.undo.pop_undo().is_none());
    assert!(!h.model.key_relay.is_holding());
    assert!(!h.model.move_in_progress);
}

// 動かした項目が新しい場所に描き直されるとき、そのエディタはフォーカスを失う。それでも編集は終わらず、動かした項目で続く。
#[test]
fn the_edit_goes_on_in_the_moved_item_though_its_editor_blurs_when_it_is_drawn_again() {
    let mut h = page_harness();
    h.model.state.add_editing_item("b");
    h.blurs_when_drawn = Some("b".into());
    h.send(Event::MoveUp {
        item_id: "b".into(),
    });
    assert!(h.model.state.is_editing("b"));
    assert_eq!(h.focused_editor_requests(), [Some("b".to_string())]);
    assert_eq!(h.model.edits.edit_handovers, 0, "the handover is over");
}

// 答えを待つ間に押されたキーは、動かした項目のエディタに渡される。キーの保留は、そのフォーカスで終わる。
// The fake server answers at once, so the wait is started by hand.
#[test]
fn keys_pressed_while_a_move_is_on_its_way_are_played_into_the_moved_item() {
    let mut h = page_harness();
    h.model.key_relay.hold_until_focus();
    h.send(Event::KeyHeld(crate::model::DeferredKey {
        key: "x".into(),
        code: "KeyX".into(),
        shift_key: false,
        ctrl_key: false,
        alt_key: false,
        meta_key: false,
    }));
    h.send(Event::MoveUp {
        item_id: "b".into(),
    });
    let replayed: Vec<Vec<String>> =
        h.ui.iter()
            .filter_map(|u| match u {
                crate::effects::UiOp::FocusEditor { replay_keys, .. } => {
                    Some(replay_keys.iter().map(|k| k.key.clone()).collect())
                }
                _ => None,
            })
            .collect();
    assert_eq!(replayed, [vec!["x".to_string()]]);
    assert!(!h.model.key_relay.is_holding());
}

// トランスクルージョンが見せている項目に打って、まだ保存していない文も、移動の要求に入って届く。保存を別には送らない。ページを読み直したあと、その項目の編集が続く。
#[test]
fn text_typed_into_a_shown_item_travels_in_its_move_inside_the_transclusion() {
    let mut h = transclusion_harness();
    typed(&mut h, "c2", "typed");
    h.send(Event::MoveUp {
        item_id: "c2".into(),
    });
    assert_eq!(carried_bodies(&h), [Some("typed".to_string())]);
    h.fire_timers();
    assert!(crate::harness::update_calls(&h.api).is_empty());
    assert_eq!(h.api.calls_named("GetPageById").len(), 1);
    assert_eq!(h.focused_editor_requests(), [Some("c2".to_string())]);
    assert!(!h.model.key_relay.is_holding());
    assert!(!h.model.move_in_progress);
    assert_eq!(h.model.edits.edit_handovers, 0, "the handover is over");
}

// 読み直すのは、トランスクルージョンのあるページである。ジャーナルは何日ものページを見せるので、画面のページとは限らない。
#[test]
fn the_page_read_again_after_a_move_is_the_page_of_the_transclusion() {
    let mut h = transclusion_harness();
    h.model
        .state
        .update_item("root", |i| i.page_id = "day2".into());
    h.send(Event::MoveUp {
        item_id: "c2".into(),
    });
    assert_eq!(
        h.api.calls_named("GetPageById"),
        [&ApiOp::GetPageById {
            page_id: "day2".into()
        }]
    );
}

// トランスクルージョンに打って、まだ保存していない文も、そのトランスクルージョンを動かす要求に入って届く。トランスクルージョンは元の項目の本文を見せ、打った文字は保存されるまで自分で持つ。
#[test]
fn text_typed_into_a_transclusion_travels_in_the_move_of_the_transclusion() {
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item_with("root", 1, |i| i.transclude_item_id = Some("s".into())),
    ]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "shown")]);
    h.model.state.set_transcluded_trees(trees);
    typed(&mut h, "root", "shown typed");
    h.send(Event::MoveUp {
        item_id: "root".into(),
    });
    assert_eq!(carried_bodies(&h), [Some("shown typed".to_string())]);
    assert_eq!(h.items_ids(), ["root", "a"]);
    h.fire_timers();
    assert!(crate::harness::update_calls(&h.api).is_empty());
}

// トランスクルージョンの中の移動の答えが届いたときに、その項目がもう画面に無ければ、ページを読み直さず、フォーカスも移さない。キーの保留は終わり、次の移動を始められる。
#[test]
fn the_answer_to_a_move_inside_a_transclusion_no_longer_shown_reads_and_focuses_nothing() {
    let mut h = Harness::with_items(vec![item("x", 1, "other page")]);
    h.model.key_relay.hold_until_focus();
    h.model.move_in_progress = true;
    h.send(Event::MovedInTransclusion {
        item_id: "c2".into(),
    });
    assert!(h.api.calls.is_empty());
    assert!(h.focused_editor_requests().is_empty());
    assert!(!h.model.key_relay.is_holding());
    assert!(!h.model.move_in_progress);
    assert_eq!(h.model.edits.edit_handovers, 0);
}

// 移動の答えを待つ間に押された移動は、その答えのあとに始まる。移動先は、項目が動いたあとの並びから決まる。
#[test]
fn a_move_pressed_while_another_is_on_its_way_starts_when_that_one_is_through() {
    let mut h = page_harness();
    h.model.move_in_progress = true;
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    assert!(move_calls(&h.api).is_empty());

    h.send(Event::MoveDone { handover: false });
    assert_eq!(h.items_ids(), ["b", "b1", "a", "c"]);
}

// 保存済みの項目の移動が失敗しても、何も保存されない。
#[test]
fn a_failed_move_saves_nothing_when_the_item_had_no_unsaved_text() {
    let mut h = page_harness();
    h.api.fail_next("MoveItems", "offline");
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    h.fire_timers();
    assert!(crate::harness::update_calls(&h.api).is_empty());
    assert_eq!(h.items_ids(), ["a", "b", "b1", "c"]);
}

// トランスクルージョンが見せている項目は、その中だけで動く。最初の項目は上へ動かず、最後の項目は下へ動かない。
#[test]
fn an_item_a_transclusion_shows_does_not_move_out_of_it() {
    let mut h = transclusion_harness();
    h.send(Event::MoveUp {
        item_id: "c1".into(),
    });
    h.send(Event::MoveDown {
        item_id: "c2".into(),
    });
    assert!(h.api.calls.is_empty());
    assert!(!h.model.key_relay.is_holding());
}

// トランスクルージョンの中で下へ動かすと、参照元ツリーの次の兄弟の後ろへ移動する。
#[test]
fn inside_a_transclusion_a_descendant_moves_below_its_next_sibling() {
    let mut h = transclusion_harness();
    h.send(Event::MoveDown {
        item_id: "c1".into(),
    });
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["c1".to_string()],
            "page1".to_string(),
            "after:c2".to_string()
        )]
    );
}
