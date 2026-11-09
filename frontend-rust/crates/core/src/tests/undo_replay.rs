use crate::app::Event;
use crate::effects::ApiOutput;
use crate::harness::{batch_calls, create_calls, delete_calls, item, update_calls, Harness};
use crate::model::{BatchOp, Item, ItemPosition, TaskState, TranscludedTrees};
use crate::outline::CursorState;
use crate::testing::item_with;
use serde_json::json;

/// The update of a batch that writes a body, a level and a collapsed state, and no task state
fn update_op(id: &str, level: i32, body: &str) -> BatchOp {
    BatchOp::Update {
        id: id.into(),
        body: Some(body.into()),
        level: Some(level),
        collapsed: Some(false),
        task: None,
    }
}

// undo は古い本文を戻し、1つの要求でサーバーに送り、その項目を redo スタックに移す。redo はそれを元に戻す。
#[test]
fn undo_puts_the_old_body_back_sends_it_in_one_request_and_moves_the_entry_to_the_redo_stack() {
    let mut h = Harness::with_items(vec![item("a", 1, "new")]);
    let before = item("a", 1, "old");
    let after = item("a", 1, "new");
    h.model.undo.begin("body", None);
    h.model.undo.push_update(&before, &after);
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);
    assert_eq!(h.bodies(), ["a=old"]);
    assert_eq!(batch_calls(&h.api), [vec![update_op("a", 1, "old")]]);
    assert!(h.model.undo.can_redo());
    assert!(!h.model.undo.can_undo());

    h.send(Event::Redo);
    assert_eq!(h.bodies(), ["a=new"]);
    assert_eq!(batch_calls(&h.api)[1], [update_op("a", 1, "new")]);
    assert!(h.model.undo.can_undo());
}

fn shown_ast(h: &Harness, item_id: &str) -> String {
    let rows = h.view().outline.rows;
    let row = rows.iter().find(|r| r.item.id == item_id).unwrap();
    row.item.ast.clone()
}

// 入力を redo した後に別の項目へ移動しても、redo した本文の内容で表示される。
#[test]
fn the_view_shows_the_redone_text_after_moving_away() {
    let mut h = Harness::with_items(vec![item("a", 1, "old"), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        cursor: 3,
    });
    h.send(Event::BodyChanged {
        item_id: "a".into(),
        body: "new".into(),
        now_ms: 0,
    });
    h.send(Event::Undo);
    h.send(Event::Redo);
    h.send(Event::NavigateDown {
        item_id: "a".into(),
        cursor: 0,
    });
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(shown_ast(&h, "a"), "ast:new");
}

// 本文を変えない操作の redo では、サーバーの応答が項目の種類を上書きしない。
#[test]
fn the_redo_of_a_task_cycle_keeps_the_item_a_task() {
    let mut h = Harness::with_items(vec![item("a", 1, "x")]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    h.send(Event::Undo);
    h.send(Event::Redo);
    assert!(h.model.state.get_item_by_id("a").unwrap().is_task());
}

/// A task that is done, with its times and its plan.
fn done_task(id: &str) -> Item {
    item_with(id, 1, |i| {
        i.body = "x".into();
        i.types = vec!["task".into()];
        i.properties = [
            ("begin".to_string(), json!("b")),
            ("end".to_string(), json!("e")),
            ("plan".to_string(), json!("p")),
        ]
        .into();
    })
}

fn done_state() -> TaskState {
    TaskState {
        task: true,
        begin: Some("b".into()),
        end: Some("e".into()),
        plan: Some("p".into()),
    }
}

/// The task states the batches sent, as `(item, state)`
fn task_states_sent(h: &Harness) -> Vec<(String, TaskState)> {
    let ops = batch_calls(&h.api).into_iter().flatten();
    ops.filter_map(|op| match op {
        BatchOp::Update {
            id,
            task: Some(state),
            ..
        } => Some((id, state)),
        _ => None,
    })
    .collect()
}

// タスク状態の切り替えの undo は、元の時刻と予定日ごと、元の状態を、項目の更新と同じ操作でサーバーに送る。redo は切り替え後の状態を送る。
#[test]
fn undo_and_redo_of_a_task_cycle_send_the_task_state_to_the_server() {
    let mut h = Harness::with_items(vec![done_task("a")]);
    // A done task cycles on to a normal item
    h.api
        .answer_next("CycleTask", Ok(ApiOutput::Item(item("a", 1, "x"))));
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });

    h.send(Event::Undo);
    assert_eq!(
        batch_calls(&h.api),
        [[BatchOp::Update {
            id: "a".into(),
            body: Some("x".into()),
            level: Some(1),
            collapsed: Some(false),
            task: Some(done_state()),
        }]]
    );

    h.send(Event::Redo);
    assert_eq!(task_states_sent(&h).len(), 2);
    assert_eq!(task_states_sent(&h)[1].1, TaskState::default());
}

// undo の後は、サーバーが答えたタスク状態が表示される。
#[test]
fn undo_of_a_task_cycle_shows_the_task_state_the_server_answers() {
    let mut h = Harness::with_items(vec![done_task("a")]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    h.send(Event::Undo);
    let shown = h.model.state.get_item_by_id("a").unwrap();
    assert_eq!(shown.task_state(), done_state());
}

// トランスクルージョンのルートでのタスク状態の undo は、元の項目のタスク状態を戻す。ルートの更新と同じ要求で送る。
#[test]
fn undo_of_a_task_cycle_on_a_transclusion_root_goes_to_the_source_item() {
    let root = item_with("root", 1, |i| i.transclude_item_id = Some("s".into()));
    let mut h = Harness::with_items(vec![root]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![done_task("s")]);
    h.model.state.set_transcluded_trees(trees);
    h.send(Event::TaskCycle {
        item_id: "root".into(),
    });

    h.send(Event::Undo);

    assert_eq!(batch_calls(&h.api).len(), 1);
    assert_eq!(task_states_sent(&h), [("s".to_string(), done_state())]);
    let rows = h.view().outline.rows;
    let shown = rows.iter().find(|r| r.item.id == "root").unwrap();
    assert_eq!(shown.item.task_state(), done_state());
}

// タスク状態が変わらない操作の undo では、タスク状態を送らない。
#[test]
fn undo_of_a_body_edit_does_not_send_the_task_state() {
    let mut h = Harness::with_items(vec![done_task("a")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        cursor: 1,
    });
    h.send(Event::BodyChanged {
        item_id: "a".into(),
        body: "y".into(),
        now_ms: 0,
    });
    h.send(Event::Undo);
    assert_eq!(batch_calls(&h.api).len(), 1);
    assert!(task_states_sent(&h).is_empty());
}

// undo の後も更新された項目の編集は続き、カーソルはその本文の末尾に置かれる。
#[test]
fn keeps_editing_the_updated_item_with_the_cursor_at_the_end_of_its_body() {
    let mut h = Harness::with_items(vec![item("a", 1, "new")]);
    h.model.undo.begin("body", None);
    h.model
        .undo
        .push_update(&item("a", 1, "old"), &item("a", 1, "new"));
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);
    assert!(h.model.state.is_editing("a"));
    assert_eq!(h.focus_cursor(), Some(3));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}

// 作成の undo は、項目をここでもサーバーでも削除する。記録されたカーソルが使われる。
#[test]
fn undo_of_a_create_removes_the_item_here_and_on_the_server() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("n", 1, "created")]);
    h.model.undo.begin("enter", Some(CursorState::new("a", 0)));
    h.model.undo.push_create(&item("n", 1, "created"));
    h.model.undo.commit(Some(CursorState::new("n", 0)), 0);
    h.send(Event::Undo);
    assert_eq!(h.items_ids(), ["a"]);
    assert_eq!(delete_calls(&h.api), [vec!["n".to_string()]]);
    // The recorded cursor is used
    assert_eq!(h.model.state.focused_item_id(), Some("a"));
}

// 親と子を作った操作の undo は、両方を1つのグループとしてサーバーに送る。親だけを先に消すとサーバーが断るためである。
#[test]
fn undo_of_a_create_of_a_parent_and_its_child_deletes_them_as_one_group() {
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item("p", 1, "parent"),
        item("c", 2, "child"),
    ]);
    h.model.undo.begin("paste", None);
    h.model.undo.push_create(&item("p", 1, "parent"));
    h.model.undo.push_create(&item("c", 2, "child"));
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);
    assert_eq!(h.items_ids(), ["a"]);
    // The order inside the group does not matter to the server
    let mut sent = delete_calls(&h.api);
    sent.iter_mut().for_each(|ids| ids.sort());
    assert_eq!(sent, [vec!["c".to_string(), "p".to_string()]]);
}

// undo の削除をサーバーが断ると、ページはサーバーの持つ形に読み直される。
#[test]
fn an_undo_delete_the_server_refuses_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("n", 1, "created")]);
    h.model.undo.begin("enter", None);
    h.model.undo.push_create(&item("n", 1, "created"));
    h.model.undo.commit(None, 0);
    h.api.fail_next("DeleteItems", "refused");
    h.send(Event::Undo);
    assert!(h.api.called("GetPageById"));
}

// 作成の redo は項目をその index の位置に戻し、サーバーが与えた id を取る。
#[test]
fn redo_of_a_create_puts_the_item_back_by_its_index_and_takes_the_id_the_server_gives() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("n", 1, "created")]);
    h.model.undo.begin("enter", None);
    h.model.undo.push_create(&item("n", 1, "created"));
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);
    h.send(Event::Redo);
    assert_eq!(create_calls(&h.api)[0].3, "created");
    assert_eq!(h.items_ids(), ["a", "new1"]);
}

// 削除の undo は、項目を元のIDのまま、元の位置に作り直す。サーバーが決めた並び順の値を受け取る。
#[test]
fn undo_of_a_delete_makes_the_item_again_with_its_id_where_it_was() {
    // The index of an item of these tests is its id: b goes back between a and c
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("c", 1, "")]);
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&item("b", 1, "gone"));
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert_eq!(
        batch_calls(&h.api),
        [[BatchOp::Create {
            id: "b".into(),
            page_id: "page1".into(),
            position: ItemPosition::after("a"),
            level: 1,
            body: "gone".into(),
        }]]
    );
    assert!(create_calls(&h.api).is_empty());
    assert_eq!(h.items_ids(), ["a", "b", "c"]);
    assert_eq!(h.model.state.get_item_by_id("b").unwrap().idx, "created1");
}

// undo スタックが空なら何も起きず、保留中の編集もそのままである。
#[test]
fn does_nothing_on_an_empty_stack() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.edits.dirty_item_ids.insert("a".into());
    h.send(Event::Undo);
    assert!(h.api.calls.is_empty());
    assert!(
        h.model.edits.dirty_item_ids.contains("a"),
        "the pending edit is left alone"
    );
}

// 折りたたみの変更の undo は、折りたたみ集合も切り替える。
#[test]
fn a_collapse_change_also_flips_the_collapsed_set() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 2, "")]);
    let mut after = item("a", 1, "");
    after.collapsed = true;
    h.model.undo.begin("collapse", None);
    h.model.undo.push_update(&item("a", 1, ""), &after);
    h.model.undo.commit(None, 0);
    h.model.state.toggle_collapsed("a");
    assert_eq!(h.model.state.visible_items().len(), 1);
    h.send(Event::Undo);
    assert!(!h.model.state.is_collapsed("a"));
    assert_eq!(h.model.state.visible_items().len(), 2);
}

// 入力した直後の undo は入力を戻して保存する。その入力の自動保存は捨てられ、戻した本文が上書きされることはない。
#[test]
fn an_undo_right_after_typing_drops_the_pending_save_of_the_typed_text() {
    let mut h = Harness::with_items(vec![item("a", 1, "old")]);
    h.model.state.add_editing_item("a");
    h.model.state.set_focused_item("a", None);
    h.send(Event::BodyChanged {
        item_id: "a".into(),
        body: "typed".into(),
        now_ms: h.now_ms,
    });

    h.send(Event::Undo);
    h.fire_timers();
    assert_eq!(h.bodies(), ["a=old"]);
    assert_eq!(batch_calls(&h.api), [vec![update_op("a", 1, "old")]]);
    assert!(update_calls(&h.api).is_empty());
}
