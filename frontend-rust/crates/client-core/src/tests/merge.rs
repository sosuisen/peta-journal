use crate::app::Event;
use crate::effects::{ApiOp, ApiOutput, UiOp};
use crate::harness::{item, merge_calls, op_name, typed, update_calls, Harness};
use crate::ops::merge::JoinKey;
use crate::outline::UndoOp;
use crate::testing::item_with;

fn harness() -> Harness {
    Harness::with_items(vec![
        item("a", 1, "hello"),
        item("b", 1, " world"),
        item("c", 1, "!"),
    ])
}

// 行頭での Backspace でテキストが上の項目に連結され、その項目は削除されて上の項目の編集が続く。
// サーバーには結合を1回だけ送る。本文の更新と削除を別々には送らない。
#[test]
fn backspace_appends_the_text_to_the_item_above_deletes_the_item_and_keeps_editing_above() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert_eq!(h.bodies(), ["a=hello world", "c=!"]);
    assert_eq!(merge_calls(&h.api), [("b".to_string(), "a".to_string())]);
    assert!(!h.api.called("UpdateItem"));
    assert!(!h.api.called("DeleteItem"));
    assert!(h.model.state.is_editing("a"));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}

// 行頭での Backspace は結合位置にカーソルを戻す一つの Undo エントリとして記録される。
#[test]
fn backspace_records_one_undo_entry_that_puts_the_cursor_at_the_join() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "backspace");
    assert!(matches!(
        entry.ops.as_slice(),
        [UndoOp::Update { .. }, UndoOp::Delete { .. }]
    ));
    assert_eq!(entry.cursor_after.unwrap().position, "hello".len());
    assert!(h.model.undo.pop_undo().is_none());
}

/// The saves and merges that went to the server, in the order they went.
fn saves_and_merges(h: &Harness) -> Vec<&'static str> {
    h.api
        .calls
        .iter()
        .map(op_name)
        .filter(|name| matches!(*name, "UpdateItem" | "MergeItems"))
        .collect()
}

// 自動保存の前に行頭で Backspace を押すと、打ったばかりの文は結合の要求に入って届く。保存を別には送らず、その自動保存は取り消される。保存済みの上の項目の本文は送らない。
#[test]
fn backspace_sends_the_text_just_typed_in_the_merge_itself() {
    let mut h = harness();
    typed(&mut h, "b", " world?");
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert_eq!(saves_and_merges(&h), ["MergeItems"]);
    assert_eq!(
        h.api.calls_named("MergeItems"),
        [&ApiOp::MergeItems {
            item_id: "b".into(),
            into_id: "a".into(),
            body: Some(" world?".into()),
            into_body: None,
        }]
    );
    assert_eq!(h.bodies(), ["a=hello world?", "c=!"]);
    h.fire_timers();
    assert!(
        update_calls(&h.api).is_empty(),
        "the text is not sent twice"
    );
}

/// The one merge request that went to the server
fn merge_request(h: &Harness) -> ApiOp {
    let merges = h.api.calls_named("MergeItems");
    assert_eq!(merges.len(), 1, "{merges:?}");
    merges[0].clone()
}

// 行頭での Backspace の前に上の項目へ打って、まだ保存していない文も、結合の要求に入って届く。
#[test]
fn backspace_sends_the_text_just_typed_in_the_item_above_in_the_merge() {
    let mut h = harness();
    typed(&mut h, "a", "hello?");
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert_eq!(saves_and_merges(&h), ["MergeItems"]);
    assert_eq!(
        merge_request(&h),
        ApiOp::MergeItems {
            item_id: "b".into(),
            into_id: "a".into(),
            body: None,
            into_body: Some("hello?".into()),
        }
    );
    h.fire_timers();
    assert!(
        update_calls(&h.api).is_empty(),
        "the text is not sent twice"
    );
}

// 自動保存の前に行末で Delete を押すと、打ったばかりの文は結合の要求に入って届く。
// 結合の後に自動保存が届いて、取り込んだ文字を上書きすることもない。
#[test]
fn delete_sends_the_text_just_typed_in_the_merge_and_saves_nothing_after_it() {
    let mut h = harness();
    typed(&mut h, "a", "hello?");
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    assert_eq!(saves_and_merges(&h), ["MergeItems"]);
    assert_eq!(
        merge_request(&h),
        ApiOp::MergeItems {
            item_id: "b".into(),
            into_id: "a".into(),
            body: None,
            into_body: Some("hello?".into()),
        }
    );
    assert_eq!(h.bodies(), ["a=hello? world", "c=!"]);
    h.fire_timers();
    assert!(
        update_calls(&h.api).is_empty(),
        "the merged text is not overwritten"
    );
}

// 行末での Delete で取り込む下の項目に打って、まだ保存していない文も、結合の要求に入って届く。
#[test]
fn delete_sends_the_text_just_typed_in_the_item_below_in_the_merge() {
    let mut h = harness();
    typed(&mut h, "b", " world?");
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    assert_eq!(saves_and_merges(&h), ["MergeItems"]);
    assert_eq!(
        merge_request(&h),
        ApiOp::MergeItems {
            item_id: "b".into(),
            into_id: "a".into(),
            body: Some(" world?".into()),
            into_body: None,
        }
    );
}

// サーバーが結合を断ると、画面は変わらず、元の項目の編集が続く。ページは読み直さない。キーの保留は終わる。
#[test]
fn a_merge_the_server_refuses_changes_nothing_on_screen_and_stops_holding_keys() {
    let mut h = harness();
    h.model.state.add_editing_item("b");
    h.api
        .fail_next("MergeItems", "A transclusion cannot be merged");
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert_eq!(h.bodies(), ["a=hello", "b= world", "c=!"]);
    assert!(h.model.state.is_editing("b"));
    assert!(!h.api.called("GetPageById"));
    assert!(!h.model.key_relay.is_holding());
    assert!(h.model.undo.pop_undo().is_none());
}

// 結合に失敗すると、要求に入っていた未保存の入力は、もう一度自動保存にかけられる。消える側の項目のものも、受け取る側の項目のものも保存される。
#[test]
fn after_a_failed_merge_the_text_typed_and_not_saved_yet_is_auto_saved_again() {
    let mut h = harness();
    typed(&mut h, "a", "hello?");
    typed(&mut h, "b", " world?");
    h.api.fail_next("MergeItems", "offline");
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert!(update_calls(&h.api).is_empty());
    h.fire_timers();
    let mut saved = update_calls(&h.api);
    saved.sort();
    assert_eq!(
        saved,
        [
            ("a".to_string(), Some("hello?".to_string())),
            ("b".to_string(), Some(" world?".to_string()))
        ]
    );
}

// 保存済みの本文での結合が失敗しても、何も保存されない。
#[test]
fn a_failed_merge_saves_nothing_when_no_item_had_unsaved_text() {
    let mut h = harness();
    h.api.fail_next("MergeItems", "offline");
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    h.fire_timers();
    assert!(update_calls(&h.api).is_empty());
}

// 答えが届くと、つないだ項目が編集され、カーソルは2つの文のつなぎ目に置かれる。キーの保留は、そのフォーカスで終わる。Backspace でも Delete でも同じである。
#[test]
fn the_joined_item_is_edited_with_the_cursor_where_the_texts_meet() {
    for key in [
        Event::BackspaceAtStart {
            item_id: "b".into(),
        },
        Event::DeleteAtEnd {
            item_id: "a".into(),
        },
    ] {
        let mut h = harness();
        h.send(key);
        assert!(h.model.state.is_editing("a"));
        assert!(!h.model.state.is_editing("b"));
        assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
        assert_eq!(focus(&h), ("a".into(), Some("hello".len())));
        assert!(!h.model.key_relay.is_holding());
    }
}

// つないだ項目は、サーバーが保存した本文で表示される。サーバーは、閉じていないコードフェンスを閉じる。
#[test]
fn the_joined_item_shows_the_body_the_server_stored() {
    let mut h = harness();
    h.api
        .answer_next("MergeItems", Ok(joined("a", "hello world\n```")));
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    assert_eq!(h.bodies(), ["a=hello world\n```", "c=!"]);
}

// 答えが届いたときに2つの項目がもう画面に無ければ、何も表示しない。待っている間に別のページへ移った場合である。キーの保留は終わる。
#[test]
fn the_answer_to_a_merge_of_items_no_longer_shown_changes_nothing() {
    let mut h = Harness::with_items(vec![item("x", 1, "other page")]);
    h.model.key_relay.hold_until_focus();
    let ApiOutput::Item(merged) = joined("a", "hello world") else {
        unreachable!()
    };
    h.send(Event::Merged {
        key: JoinKey::Backspace,
        item_id: "b".into(),
        merged: Box::new(merged),
    });
    assert_eq!(h.bodies(), ["x=other page"]);
    assert!(h.focused_editor_requests().is_empty());
    assert!(!h.model.key_relay.is_holding());
    assert!(h.model.undo.pop_undo().is_none());
}

// 確認を待つ間にページが変わって、もうつなぐ相手がなければ、要求は送らない。キーの保留は終わる。
#[test]
fn a_join_that_is_gone_when_the_user_has_answered_sends_nothing() {
    let mut h = Harness::with_items(vec![item("x", 1, "alone")]);
    h.model.key_relay.hold_until_focus();
    h.send(Event::MergeAnswered {
        key: JoinKey::Delete,
        item_id: "x".into(),
        agreed: true,
    });
    assert!(h.api.calls.is_empty());
    assert!(!h.model.key_relay.is_holding());
}

/// The item as a fake server returns it after joining: parsed from the joined text.
fn joined(id: &str, body: &str) -> ApiOutput {
    ApiOutput::Item(item_with(id, 1, |i| {
        i.body = body.into();
        i.ast = format!("ast:{body}");
        i.types = vec!["heading".into()];
        i.properties = [("tags".to_string(), ["joined"].into())].into();
    }))
}

/// What the view of an item renders and styles itself from.
fn shown_parse(
    h: &Harness,
    item_id: &str,
) -> (String, Vec<String>, Option<crate::model::PropertyValue>) {
    let rows = h.view().outline.rows;
    let row = rows.iter().find(|r| r.item.id == item_id).unwrap();
    (
        row.item.ast.clone(),
        row.item.types.clone(),
        row.item.properties.get("tags").cloned(),
    )
}

// 行頭での Backspace の直後に下の項目へ移動しても、上の項目は結合後の内容で表示される。
#[test]
fn the_item_above_shows_the_joined_text_after_backspace_and_moving_away() {
    let mut h = harness();
    h.api
        .answer_next("MergeItems", Ok(joined("a", "hello world")));
    h.send(Event::BackspaceAtStart {
        item_id: "b".into(),
    });
    h.send(Event::NavigateDown {
        item_id: "a".into(),
        cursor: 0,
    });
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(
        shown_parse(&h, "a"),
        (
            "ast:hello world".to_string(),
            vec!["heading".to_string()],
            Some(["joined"].into())
        )
    );
}

// 行末での Delete の直後に下の項目へ移動しても、取り込んだ項目は結合後の内容で表示される。
#[test]
fn the_item_shows_the_joined_text_after_delete_and_moving_away() {
    let mut h = harness();
    h.api
        .answer_next("MergeItems", Ok(joined("a", "hello world")));
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    h.send(Event::NavigateDown {
        item_id: "a".into(),
        cursor: 0,
    });
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(shown_parse(&h, "a").0, "ast:hello world");
}

// 先頭の項目では行頭での Backspace が何もしない。
#[test]
fn backspace_does_nothing_for_the_first_item() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart {
        item_id: "a".into(),
    });
    assert_eq!(h.bodies().len(), 3);
    assert!(h.api.calls.is_empty());
}

// トランスクルージョン項目は行頭での Backspace で結合されて消えることがない。
#[test]
fn backspace_does_not_merge_a_transclusion_item_away() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "a"),
        item_with("t", 1, |i| i.transclude_item_id = Some("src".into())),
    ]);
    h.send(Event::BackspaceAtStart {
        item_id: "t".into(),
    });
    assert_eq!(h.items_ids(), ["a", "t"]);
}

// ジャーナルの日の先頭項目は行頭での Backspace で上の日に結合されない。
#[test]
fn backspace_does_not_merge_the_first_item_of_a_journal_day_into_the_day_above() {
    let day = |id: &str, page: &str, name: &str| {
        item_with(id, 1, |i| {
            i.page_id = page.into();
            i.name = name.into();
            i.item_type = "journal".into();
            i.body = id.into();
        })
    };
    let mut h = Harness::with_items(vec![
        day("x", "p1", "2026-02-10"),
        day("y", "p2", "2026-02-09"),
    ]);
    h.send(Event::BackspaceAtStart {
        item_id: "y".into(),
    });
    assert_eq!(h.items_ids(), ["x", "y"]);
}

// 行末での Delete で下の項目が連結されて削除され、結合位置にカーソルを戻す Undo が記録される。
// サーバーには、下の項目をこの項目に結合する1回の呼び出しだけを送る。
#[test]
fn delete_appends_the_item_below_and_deletes_it() {
    let mut h = harness();
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    assert_eq!(h.bodies(), ["a=hello world", "c=!"]);
    assert_eq!(merge_calls(&h.api), [("b".to_string(), "a".to_string())]);
    assert!(!h.api.called("UpdateItem"));
    assert!(!h.api.called("DeleteItem"));
    let entry = h.model.undo.pop_undo().unwrap();
    assert_eq!(entry.label, "delete");
    assert_eq!(entry.cursor_after.unwrap().position, "hello".len());
}

// 最後の項目では行末での Delete が何もしない。
#[test]
fn delete_does_nothing_for_the_last_item() {
    let mut h = harness();
    h.send(Event::DeleteAtEnd {
        item_id: "c".into(),
    });
    assert_eq!(h.bodies().len(), 3);
}

// 下の項目がトランスクルージョンなら、行末での Delete はそれを取り込まない。
#[test]
fn delete_does_not_merge_a_transclusion_item_below_away() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "a"),
        item_with("t", 1, |i| i.transclude_item_id = Some("src".into())),
    ]);
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });
    assert_eq!(h.items_ids(), ["a", "t"]);
    assert!(h.api.calls.is_empty());
}

// トランスクルージョンは、文を受け取る側にもならない。その下の項目の行頭での Backspace も、トランスクルージョンの行末での Delete も、サーバーに何も送らない。サーバーは、どちらの結合も断る。
#[test]
fn nothing_is_joined_into_a_transclusion_and_nothing_is_sent() {
    let keys = [
        (
            "Backspace",
            Event::BackspaceAtStart {
                item_id: "b".into(),
            },
        ),
        (
            "Delete",
            Event::DeleteAtEnd {
                item_id: "t".into(),
            },
        ),
    ];
    for (name, key) in keys {
        let mut h = Harness::with_items(vec![
            item_with("t", 1, |i| i.transclude_item_id = Some("src".into())),
            item("b", 1, "b"),
        ]);
        h.send(key);
        assert_eq!(h.items_ids(), ["t", "b"]);
        assert!(h.api.calls.is_empty(), "{name}: {:?}", h.api.calls);
        assert!(!h.model.key_relay.is_holding());
    }
}

// ジャーナルの日の最後の項目は、行末での Delete で下の日の項目を取り込まない。
#[test]
fn delete_does_not_merge_the_first_item_of_the_journal_day_below() {
    let day = |id: &str, page: &str, name: &str| {
        item_with(id, 1, |i| {
            i.page_id = page.into();
            i.name = name.into();
            i.item_type = "journal".into();
            i.body = id.into();
        })
    };
    let mut h = Harness::with_items(vec![
        day("x", "p1", "2026-02-10"),
        day("y", "p2", "2026-02-09"),
    ]);
    h.send(Event::DeleteAtEnd {
        item_id: "x".into(),
    });
    assert_eq!(h.items_ids(), ["x", "y"]);
    assert!(h.api.calls.is_empty());
}

/// A1, A2, A3 in this order, with A2 transcluded by an item on another page.
fn with_a2_transcluded(confirm: bool) -> Harness {
    let mut h = Harness::with_items(vec![
        item("a1", 1, "one"),
        item("a2", 1, "two"),
        item("a3", 1, "three"),
    ]);
    h.confirm = confirm;
    h.api.answer_next(
        "TranscludedItems",
        Ok(ApiOutput::ItemIds(vec!["a2".into()])),
    );
    h
}

fn asked(h: &Harness) -> bool {
    h.ui.iter()
        .any(|op| matches!(op, UiOp::ConfirmDestructive(m) if m.contains("transcluded")))
}

fn focus(h: &Harness) -> (String, Option<usize>) {
    let request = h.model.state.focus_request().expect("a focus request");
    (request.item_id.clone(), request.cursor)
}

// 他の項目からトランスクルードされている A2 の先頭で Backspace を押すと、確認の後に A1 へ結合される。
#[test]
fn backspace_at_the_start_of_a_transcluded_item_joins_it_after_an_ok() {
    let mut h = with_a2_transcluded(true);
    h.send(Event::BackspaceAtStart {
        item_id: "a2".into(),
    });

    assert!(asked(&h));
    assert_eq!(h.bodies(), ["a1=onetwo", "a3=three"]);
    assert_eq!(merge_calls(&h.api), [("a2".into(), "a1".into())]);
}

// 確認でキャンセルすると A2 は残り、カーソルは A2 の先頭に戻る。キーの保留は終わる。
#[test]
fn a_cancel_to_backspace_keeps_the_transcluded_item_and_the_cursor_at_its_start() {
    let mut h = with_a2_transcluded(false);
    h.send(Event::BackspaceAtStart {
        item_id: "a2".into(),
    });

    assert!(asked(&h));
    assert_eq!(h.bodies(), ["a1=one", "a2=two", "a3=three"]);
    assert!(merge_calls(&h.api).is_empty());
    assert_eq!(focus(&h), ("a2".into(), Some(0)));
    assert!(!h.model.key_relay.is_holding());
}

// A1 の末尾で Delete を押すと、トランスクルードされている A2 を取り込む前に確認する。キャンセルなら A1 の末尾に戻る。
#[test]
fn delete_at_the_end_asks_before_taking_in_a_transcluded_item_below() {
    let mut h = with_a2_transcluded(false);
    h.send(Event::DeleteAtEnd {
        item_id: "a1".into(),
    });

    assert!(asked(&h));
    assert_eq!(h.bodies(), ["a1=one", "a2=two", "a3=three"]);
    assert!(merge_calls(&h.api).is_empty());
    assert_eq!(focus(&h), ("a1".into(), Some(3)));
}

// 確認で OK なら、A1 の末尾での Delete は A2 を取り込む。
#[test]
fn delete_at_the_end_takes_in_a_transcluded_item_below_after_an_ok() {
    let mut h = with_a2_transcluded(true);
    h.send(Event::DeleteAtEnd {
        item_id: "a1".into(),
    });

    assert!(asked(&h));
    assert_eq!(h.bodies(), ["a1=onetwo", "a3=three"]);
    assert_eq!(merge_calls(&h.api), [("a2".into(), "a1".into())]);
}

// 消える項目がトランスクルードされていなければ、確認なしに結合される。
#[test]
fn a_join_of_an_item_nothing_transcludes_asks_nothing() {
    let mut h = harness();
    h.confirm = false;
    h.send(Event::DeleteAtEnd {
        item_id: "a".into(),
    });

    assert!(h.api.called("TranscludedItems"));
    assert!(!asked(&h));
    assert_eq!(h.bodies(), ["a=hello world", "c=!"]);
}

// 結合しない場面では、サーバーに尋ねもしない。
#[test]
fn no_question_goes_to_the_server_where_nothing_is_joined() {
    let mut h = harness();
    h.send(Event::BackspaceAtStart {
        item_id: "a".into(),
    });
    h.send(Event::DeleteAtEnd {
        item_id: "c".into(),
    });

    assert!(!h.api.called("TranscludedItems"));
}
