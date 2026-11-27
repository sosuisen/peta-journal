use crate::app::{EditPlace, Event};
use crate::effects::ApiOutput;
use crate::harness::{batch_calls, create_calls, delete_calls, item, update_calls, Harness};
use crate::model::ItemExt;
use std::collections::BTreeMap;

use crate::model::{
    BatchOpInput, Batched, Item, ItemPosition, PropertyValue, TaskState, TranscludedTrees,
};
use crate::outline::CursorState;
use crate::testing::item_with;

/// The update of a batch that writes a body, a level and a collapsed state, and no task state
fn update_op(id: &str, level: i32, body: &str) -> BatchOpInput {
    BatchOpInput::Update {
        id: id.into(),
        body: Some(body.into()),
        level: Some(level),
        collapsed: Some(false),
        task: None,
        properties: Default::default(),
        dropped_properties: Vec::new(),
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

// 本文の保存の答えが Undo のあとに届いても、項目は Undo で戻した本文と、その型のままである。
#[test]
fn an_answer_to_a_body_save_that_arrives_after_the_undo_leaves_the_undone_body_and_its_types() {
    let mut h = Harness::with_items(vec![item("a", 1, "- [ ] new")]);
    let before = item("a", 1, "old");
    let after = item("a", 1, "- [ ] new");
    h.model.undo.begin("body", None);
    h.model.undo.push_update(&before, &after);
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);

    let answer = item_with("a", 1, |i| {
        i.body = "- [ ] new".into();
        i.types = vec!["task".into()];
    });
    h.send(Event::BodySaved {
        sent: "- [ ] new".into(),
        item: Box::new(answer),
    });

    assert_eq!(h.bodies(), ["a=old"]);
    assert!(h.model.cloned_item("a").unwrap().types.is_empty());
}

fn shown(h: &Harness, item_id: &str) -> crate::RichText {
    let rows = h.view().outline.rows;
    let row = rows.iter().find(|r| r.item.id == item_id).unwrap();
    row.body_model.clone()
}

// 入力を redo した後に別の項目へ移動しても、redo した本文の内容で表示される。
#[test]
fn the_view_shows_the_redone_text_after_moving_away() {
    let mut h = Harness::with_items(vec![item("a", 1, "old"), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        place: EditPlace::Markdown(3),
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
    assert_eq!(shown(&h, "a"), crate::testing::shown("new"));
}

/// An undo entry that changed the body of `a` from "old" to "new", on a page that shows "new"
fn harness_with_a_body_edit() -> Harness {
    let mut h = Harness::with_items(vec![item("a", 1, "new"), item("b", 1, "")]);
    h.model.undo.begin("body", None);
    h.model
        .undo
        .push_update(&item("a", 1, "old"), &item("a", 1, "new"));
    h.model.undo.commit(None, 0);
    h
}

/// What the server answers for an item whose body it wrote: with the types it found in it
fn answered(id: &str, level: i32, body: &str) -> Item {
    item_with(id, level, |i| {
        i.body = body.into();
        i.types = vec![format!("found in {body}")];
    })
}

// undo と redo を続けたあとに、undo の応答が遅れて届いても、redo した本文の内容で表示されたままである。
#[test]
fn a_late_answer_to_an_undo_does_not_show_its_parse_over_the_redone_text() {
    let mut h = harness_with_a_body_edit();
    h.send(Event::Undo);
    h.send(Event::Redo);

    // The answer to the undo, the first sync, comes after the answer to the redo
    h.send(Event::UndoSynced {
        sync: 1,
        items: vec![answered("a", 1, "old")],
        reparsed: vec!["a".into()],
        recreated: vec![],
        created: Vec::new(),
    });

    assert!(h.model.state.get_item_by_id("a").unwrap().types.is_empty());
}

// 別々の項目の undo を続けたときは、先の undo の応答が後から届いても、その項目に反映される。後の undo は、その項目を書いていないためである。
#[test]
fn a_late_answer_is_still_shown_for_an_item_no_later_sync_wrote() {
    let mut h = Harness::with_items(vec![item("a", 1, "new a"), item("b", 1, "new b")]);
    for id in ["a", "b"] {
        h.model.undo.begin("enter", None);
        h.model.undo.push_update(
            &item(id, 1, &format!("old {id}")),
            &item(id, 1, &format!("new {id}")),
        );
        h.model.undo.commit(None, 0);
    }
    h.send(Event::Undo);
    h.send(Event::Undo);

    // The answer to the first undo, which wrote b, comes after the second undo wrote a
    let late = answered("b", 1, "old b");
    h.send(Event::UndoSynced {
        sync: 1,
        items: vec![late],
        reparsed: vec!["b".into()],
        recreated: vec![],
        created: Vec::new(),
    });

    assert_eq!(
        h.model.state.get_item_by_id("b").unwrap().types,
        ["found in old b"]
    );
}

// 字下げの undo と redo を続けたあとに、undo の応答が遅れて届いても、項目は redo した階層のままである。
#[test]
fn a_late_answer_to_an_undo_does_not_put_the_item_back_at_the_old_level() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 2, "")]);
    h.model.undo.begin("indent", None);
    h.model
        .undo
        .push_update(&item("b", 1, ""), &item("b", 2, ""));
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);
    h.send(Event::Redo);

    h.send(Event::UndoSynced {
        sync: 1,
        items: vec![answered("b", 1, "")],
        reparsed: vec![],
        recreated: vec![],
        created: Vec::new(),
    });

    assert_eq!(h.outline(), ["a:1=", "b:2="]);
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
            ("begin".to_string(), "b".into()),
            ("end".to_string(), "e".into()),
            ("plan".to_string(), "p".into()),
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
        BatchOpInput::Update {
            id,
            task: Some(state),
            ..
        } => Some((id, state)),
        _ => None,
    })
    .collect()
}

// 切り替えの答えが届く前に、もう一度切り替えても、undo は、先の答えが書いた時刻のある状態を送る。2 つめのキーを押したとき、画面はまだ先の答えを持たない。
#[test]
fn undo_of_a_cycle_pressed_before_the_last_answer_sends_what_that_answer_wrote() {
    // A running task: the key that ends it and the key that makes it a normal item again were
    // both pressed before the server answered the first of them
    let mut h = Harness::with_items(vec![item_with("a", 1, |i| {
        i.body = "x".into();
        i.types = vec!["task".into()];
        i.properties = [
            ("begin".to_string(), "b".into()),
            ("plan".to_string(), "p".into()),
        ]
        .into();
    })]);
    h.send(Event::TaskCycled {
        item_id: "a".into(),
        target_id: "a".into(),
        updated: Box::new(done_task("a")),
    });
    h.send(Event::TaskCycled {
        item_id: "a".into(),
        target_id: "a".into(),
        updated: Box::new(item("a", 1, "x")),
    });
    h.send(Event::Undo);
    assert_eq!(task_states_sent(&h), [("a".to_string(), done_state())]);
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
        [[BatchOpInput::Update {
            id: "a".into(),
            body: Some("x".into()),
            level: Some(1),
            collapsed: Some(false),
            task: Some(done_state()),
            properties: Default::default(),
            dropped_properties: Vec::new(),
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
        place: EditPlace::Markdown(1),
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

// 削除の undo は、トランスクルージョンが同じページの項目を見せていても、項目をこのページの項目として戻す（#736）。
#[test]
fn undo_of_a_delete_puts_the_item_back_on_this_page_when_a_transclusion_shows_this_page_too() {
    let root = item_with("root", 1, |i| {
        i.transclude_item_id = Some("a".into());
        i.body = "a".into();
    });
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "b"), root]);
    let mut trees = TranscludedTrees::new();
    trees.insert("a".into(), vec![item_with("a", 1, |i| i.body = "a".into())]);
    h.model.state.set_transcluded_trees(trees);
    h.send(Event::DeleteItem {
        item_id: "b".into(),
    });
    assert_eq!(h.items_ids(), ["a", "root"]);

    h.send(Event::Undo);

    assert_eq!(h.items_ids(), ["a", "b", "root"]);
}

// 作成の undo は、項目をここで消し、削除を同じ1つの要求でサーバーに送る。記録されたカーソルが使われる。
#[test]
fn undo_of_a_create_removes_the_item_here_and_on_the_server() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("n", 1, "created")]);
    h.model.undo.begin("enter", Some(CursorState::new("a", 0)));
    h.model.undo.push_create(&item("n", 1, "created"));
    h.model.undo.commit(Some(CursorState::new("n", 0)), 0);
    h.send(Event::Undo);
    assert_eq!(h.items_ids(), ["a"]);
    assert_eq!(
        batch_calls(&h.api),
        [[BatchOpInput::Delete { id: "n".into() }]]
    );
    assert!(delete_calls(&h.api).is_empty());
    // The recorded cursor is used
    assert_eq!(h.model.state.focused_item_id(), Some("a"));
}

// undo でページの項目がなくなると、サーバーが同じ要求で作った空の項目が表示され、フォーカスされる。空の項目を作る要求は送らない。
#[test]
fn undo_that_empties_the_page_shows_the_empty_item_the_server_made() {
    let mut h = Harness::with_items(vec![item("n", 1, "created")]);
    h.model.undo.begin("enter", None);
    h.model.undo.push_create(&item("n", 1, "created"));
    h.model.undo.commit(None, 0);
    let empty = item_with("e", 1, |i| {
        i.page_id = h.model.shown_page_id().expect("a page is shown");
    });
    h.api.answer_next(
        "ApplyBatch",
        Ok(ApiOutput::Batched(Box::new(Batched {
            items: Vec::new(),
            created: vec![empty],
        }))),
    );

    h.send(Event::Undo);

    assert_eq!(
        batch_calls(&h.api),
        [[BatchOpInput::Delete { id: "n".into() }]]
    );
    assert!(!h.api.called("CreateItem"));
    assert_eq!(h.items_ids(), ["e"]);
    assert_eq!(h.model.state.focused_item_id(), Some("e"));
}

// 親と子を作った操作の undo は、両方の削除を1つの要求でサーバーに送る。親だけを先に消すとサーバーが断るためである。
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
    // The order of the deletes does not matter to the server: it deletes them as one group
    let mut sent = batch_calls(&h.api);
    sent.iter_mut()
        .for_each(|ops| ops.sort_by_key(|op| format!("{op:?}")));
    assert_eq!(
        sent,
        [[
            BatchOpInput::Delete { id: "c".into() },
            BatchOpInput::Delete { id: "p".into() },
        ]]
    );
}

// undo の要求をサーバーが断ると、ページはサーバーの持つ形に読み直される。
#[test]
fn an_undo_the_server_refuses_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("n", 1, "created")]);
    h.model.undo.begin("enter", None);
    h.model.undo.push_create(&item("n", 1, "created"));
    h.model.undo.commit(None, 0);
    h.api.fail_next("ApplyBatch", "refused");
    h.send(Event::Undo);
    assert!(h.api.called("GetPageById"));
    assert_eq!(h.items_ids(), ["reloaded"]);
}

// 作成の redo は、項目を元のIDのまま、その index の位置に作り直す。
#[test]
fn redo_of_a_create_makes_the_item_again_with_its_id_where_its_index_puts_it() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("n", 1, "created")]);
    h.model.undo.begin("enter", None);
    h.model.undo.push_create(&item("n", 1, "created"));
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);
    h.send(Event::Redo);
    assert_eq!(
        batch_calls(&h.api)[1],
        [BatchOpInput::Create {
            id: "n".into(),
            page_id: "page1".into(),
            position: ItemPosition::after("a"),
            level: 1,
            body: "created".into(),
            transclude_item_id: None,
            properties: BTreeMap::new(),
        }]
    );
    assert_eq!(h.items_ids(), ["a", "n"]);
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
        [[BatchOpInput::Create {
            id: "b".into(),
            page_id: "page1".into(),
            position: ItemPosition::after("a"),
            level: 1,
            body: "gone".into(),
            transclude_item_id: None,
            properties: BTreeMap::new(),
        }]]
    );
    assert!(create_calls(&h.api).is_empty());
    assert_eq!(h.items_ids(), ["a", "b", "c"]);
    assert_eq!(h.model.state.get_item_by_id("b").unwrap().idx, "created1");
}

// 削除の undo は、作り直した項目の折りたたみとタスクの状態を、同じ要求の更新で戻す。
#[test]
fn undo_of_a_delete_puts_back_the_collapsed_state_and_the_task_state_in_the_same_request() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("c", 1, "")]);
    let gone = item_with("b", 1, |i| {
        i.body = "gone".into();
        i.collapsed = true;
        i.types = vec!["task".into()];
    });
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&gone);
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert_eq!(
        batch_calls(&h.api),
        [[
            create_op("b", "a", 1, "gone"),
            BatchOpInput::Update {
                id: "b".into(),
                body: None,
                level: None,
                collapsed: Some(true),
                task: Some(TaskState {
                    task: true,
                    begin: None,
                    end: None,
                    plan: None,
                }),
                properties: Default::default(),
                dropped_properties: Vec::new(),
            },
        ]]
    );
}

// 削除の undo は、タスクでない項目でも、折りたたんであれば、折りたたみを更新で戻す。
#[test]
fn undo_of_a_delete_puts_back_the_collapsed_state_of_an_item_that_is_no_task() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("c", 1, "")]);
    let gone = item_with("b", 1, |i| {
        i.body = "gone".into();
        i.collapsed = true;
    });
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&gone);
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert_eq!(
        batch_calls(&h.api),
        [[
            create_op("b", "a", 1, "gone"),
            BatchOpInput::Update {
                id: "b".into(),
                body: None,
                level: None,
                collapsed: Some(true),
                task: None,
                properties: Default::default(),
                dropped_properties: Vec::new(),
            },
        ]]
    );
}

// 削除の undo は、作り直す項目のトランスクルージョンとプロパティも、作成の操作で送る。
#[test]
fn undo_of_a_delete_sends_the_transclusion_and_the_properties_with_the_create() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("c", 1, "")]);
    let gone = item_with("b", 1, |i| {
        i.body = "source text".into();
        i.transclude_item_id = Some("s".into());
        i.properties = BTreeMap::from([
            ("status".into(), PropertyValue::Text("done".into())),
            (
                "owners_list".into(),
                PropertyValue::List(vec!["ann".into(), "bob".into()]),
            ),
        ]);
    });
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&gone);
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    let BatchOpInput::Create {
        transclude_item_id,
        properties,
        ..
    } = &batch_calls(&h.api)[0][0]
    else {
        panic!("not a create: {:?}", batch_calls(&h.api));
    };
    assert_eq!(transclude_item_id.as_deref(), Some("s"));
    assert_eq!(properties, &gone.properties);
}

fn create_op(id: &str, after: &str, level: i32, body: &str) -> BatchOpInput {
    BatchOpInput::Create {
        id: id.into(),
        page_id: "page1".into(),
        position: ItemPosition::after(after),
        level,
        body: body.into(),
        transclude_item_id: None,
        properties: BTreeMap::new(),
    }
}

// Undoで作り直した項目の答えの並び順の値が、上と下の項目の値の間にないときは、ページの並び順の値を読み直す。サーバーが周りの項目の値を振り直したためである。
#[test]
fn an_item_made_again_out_of_place_reads_the_indices_again() {
    // The fake gives the item made again the index "created1", which sorts after "c"
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("c", 1, "")]);
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&item("b", 1, "gone"));
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert!(h.api.called("GetPageById"));
}

// Undoで作り直した項目の答えの並び順の値が、上と下の項目の値の間にあるときは、読み直さない。
#[test]
fn an_item_made_again_in_place_does_not_read_again() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("z", 1, "")]);
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&item("b", 1, "gone"));
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert!(!h.api.called("GetPageById"));
}

// 削除の redo は、項目をここでもう一度消し、削除をサーバーに送る。
#[test]
fn redo_of_a_delete_removes_the_item_again_here_and_on_the_server() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("c", 1, "")]);
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&item("b", 1, "gone"));
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);

    h.send(Event::Redo);

    assert_eq!(h.items_ids(), ["a", "c"]);
    assert_eq!(
        batch_calls(&h.api)[1],
        [BatchOpInput::Delete { id: "b".into() }]
    );
}

// いくつかの項目の削除の undo は、上の項目から順に作り直す。下の項目は、先に作り直した上の項目を位置の基準にするためである。
#[test]
fn undo_of_a_delete_of_several_items_makes_them_again_from_the_top_down() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&item("b", 1, "parent"));
    h.model.undo.push_delete(&item("c", 2, "child"));
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert_eq!(h.items_ids(), ["a", "b", "c"]);
    assert_eq!(
        batch_calls(&h.api),
        [[
            create_op("b", "a", 1, "parent"),
            create_op("c", "b", 2, "child"),
        ]]
    );
}

// 結合の undo は、受けた項目の更新を先に、消えた項目の作り直しを後に送る。作り直す項目の階層は、上の項目の更新後の階層で確かめられるためである。
#[test]
fn undo_of_a_merge_sends_the_update_before_the_item_made_again() {
    let mut h = Harness::with_items(vec![item("a", 1, "ab")]);
    h.model.undo.begin("backspace", None);
    h.model
        .undo
        .push_update(&item("a", 1, "a"), &item("a", 1, "ab"));
    h.model.undo.push_delete(&item("b", 1, "b"));
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert_eq!(h.bodies(), ["a=a", "b=b"]);
    assert_eq!(
        batch_calls(&h.api),
        [[update_op("a", 1, "a"), create_op("b", "a", 1, "b")]]
    );
}

/// An item with the index that sorts it among the others of its page
fn at(id: &str, idx: &str, level: i32) -> Item {
    item_with(id, level, |i| i.idx = idx.into())
}

fn move_op(ids: &[&str], position: ItemPosition) -> BatchOpInput {
    BatchOpInput::Move {
        ids: ids.iter().map(|id| id.to_string()).collect(),
        page_id: "page1".into(),
        position,
    }
}

// 移動の undo は、項目を元の位置に戻し、その移動をサーバーに送る。redo は、もう一度動かす。
#[test]
fn undo_of_a_move_puts_the_item_back_where_it_was_and_redo_moves_it_again() {
    // a went from above b to below it
    let mut h = Harness::with_items(vec![at("b", "b", 1), at("a", "c", 1)]);
    h.model
        .undo
        .record_updates("move", [(&at("a", "a", 1), &at("a", "c", 1))], 0);

    h.send(Event::Undo);
    assert_eq!(h.items_ids(), ["a", "b"]);
    assert_eq!(
        batch_calls(&h.api),
        [[update_op("a", 1, ""), move_op(&["a"], ItemPosition::First)]]
    );

    h.send(Event::Redo);
    assert_eq!(h.items_ids(), ["b", "a"]);
    assert_eq!(
        batch_calls(&h.api)[1],
        [
            update_op("a", 1, ""),
            move_op(&["a"], ItemPosition::after("b"))
        ]
    );
}

// 子を持つ項目の移動の undo は、まとまりを1つの移動として、上の項目から順にサーバーに送る。親だけを先に動かすと、子が別の親の下に入るためである。
#[test]
fn undo_of_a_move_of_a_parent_sends_the_group_as_one_move() {
    // a and its child a1 went from above b to below it
    let mut h = Harness::with_items(vec![at("b", "b", 1), at("a", "c", 1), at("a1", "d", 2)]);
    h.model.undo.record_updates(
        "move",
        [
            (&at("a", "a", 1), &at("a", "c", 1)),
            (&at("a1", "a5", 2), &at("a1", "d", 2)),
        ],
        0,
    );

    h.send(Event::Undo);

    assert_eq!(h.items_ids(), ["a", "a1", "b"]);
    let sent = batch_calls(&h.api).remove(0);
    assert_eq!(
        sent.last(),
        Some(&move_op(&["a", "a1"], ItemPosition::First))
    );
    assert_eq!(sent.len(), 3, "two updates and one move: {sent:?}");
}

// トランスクルージョンの中での移動の undo は、項目を参照元ツリーの中で前にあった項目の後ろへ戻す移動を送り、ページを読み直す。このページは元のページの項目の並びを持たないので、並びはサーバーから読む。
#[test]
fn undo_of_a_move_inside_a_transclusion_moves_the_item_back_and_reads_the_page_again() {
    let root = item_with("root", 1, |i| i.transclude_item_id = Some("s".into()));
    let mut h = Harness::with_items(vec![root]);
    let mut trees = TranscludedTrees::new();
    // c2 went from below c1 to above it
    let tree = vec![at("s", "s", 1), at("c2", "s1", 2), at("c1", "s2", 2)];
    trees.insert("s".into(), tree);
    h.model.state.set_transcluded_trees(trees);
    h.model
        .undo
        .record_updates("move", [(&at("c2", "s3", 2), &at("c2", "s1", 2))], 0);

    h.send(Event::Undo);

    assert_eq!(
        batch_calls(&h.api),
        [[
            update_op("c2", 2, ""),
            move_op(&["c2"], ItemPosition::after("c1"))
        ]]
    );
    assert!(h.api.called("GetPageById"));
}

// 論理アウトデントの undo は、階層と位置の両方を戻す。階層の更新を先に、移動を後に送る。
#[test]
fn undo_of_a_logical_outdent_puts_the_level_and_the_place_back() {
    // x was the first child of p; it went below the last child, one level up
    let mut h = Harness::with_items(vec![at("p", "p", 1), at("c1", "p2", 2), at("x", "p3", 1)]);
    h.model
        .undo
        .record_updates("outdent", [(&at("x", "p1", 2), &at("x", "p3", 1))], 0);

    h.send(Event::Undo);

    assert_eq!(h.outline(), ["p:1=", "x:2=", "c1:2="]);
    assert_eq!(
        batch_calls(&h.api),
        [[
            update_op("x", 2, ""),
            move_op(&["x"], ItemPosition::after("p"))
        ]]
    );
}

// 移動の undo では、動いた項目のエディタが開いたままである。行が置き直されるとエディタはフォーカスを失うが、それで編集は終わらない。
#[test]
fn undo_of_a_move_keeps_the_editor_of_the_moved_item_open() {
    // a went from the top to below b
    let mut h = Harness::with_items(vec![at("b", "b", 1), at("a", "c", 1)]);
    h.model
        .undo
        .record_updates("move", [(&at("a", "a", 1), &at("a", "c", 1))], 0);
    h.model.state.add_editing_item("a");
    h.blurs_when_drawn = Some("a".into());

    h.send(Event::Undo);

    assert_eq!(h.items_ids(), ["a", "b"]);
    assert!(h.model.state.is_editing("a"));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
    assert_eq!(h.model.edits.edit_handovers, 0, "the handover is over");
}

// 移動と作り直しがある undo は、上の項目から順に送る。作り直す項目の位置の基準が、先に動いているためである。
#[test]
fn an_undo_that_moves_and_makes_items_again_sends_them_from_the_top_down() {
    // y went from the top to below k, and x, which was below y, was deleted
    let mut h = Harness::with_items(vec![at("k", "b", 1), at("y", "c", 1)]);
    h.model.undo.begin("move", None);
    h.model.undo.push_update(&at("y", "a", 1), &at("y", "c", 1));
    h.model.undo.push_delete(&at("x", "a5", 1));
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);

    assert_eq!(h.items_ids(), ["y", "x", "k"]);
    assert_eq!(
        batch_calls(&h.api),
        [[
            update_op("y", 1, ""),
            move_op(&["y"], ItemPosition::First),
            create_op("x", "y", 1, ""),
        ]]
    );
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
    h.model.state.set_collapsed("a", true);
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

fn enter(h: &mut Harness, item_id: &str, cursor: usize) {
    let now = h.now_ms;
    h.send(Event::EnterPressed {
        item_id: item_id.into(),
        cursor,
        now_ms: now,
    });
}

// Enter で作った項目を undo で消すと、編集はカーソルが戻る項目だけになる。消えた項目に編集中の印が残ると、他のクライアントの変更が取り込まれなくなる。
#[test]
fn undoing_an_enter_leaves_only_the_returned_item_being_edited() {
    let mut h = Harness::with_items(vec![item("a", 1, "abc")]);
    h.model.state.add_editing_item("a");
    enter(&mut h, "a", 3);
    assert!(h.model.state.is_editing("new1"));

    h.send(Event::Undo);

    assert!(h.model.state.is_editing("a"));
    assert!(!h.model.state.is_editing("new1"));
}

// undo した Enter を redo すると、編集は作り直した項目だけになる。
#[test]
fn redoing_an_enter_leaves_only_the_new_item_being_edited() {
    let mut h = Harness::with_items(vec![item("a", 1, "abc")]);
    h.model.state.add_editing_item("a");
    enter(&mut h, "a", 3);
    h.send(Event::Undo);

    h.send(Event::Redo);

    assert!(h.model.state.is_editing("new1"));
    assert!(!h.model.state.is_editing("a"));
}

/// The ids the moves of the batches sent to the server move.
fn moved_ids(h: &Harness) -> Vec<Vec<String>> {
    batch_calls(&h.api)
        .into_iter()
        .flatten()
        .filter_map(|op| match op {
            BatchOpInput::Move { ids, .. } => Some(ids),
            _ => None,
        })
        .collect()
}

/// Items a and b, after an edit of the body of b that the undo history holds.
fn harness_after_an_edit_of_b() -> Harness {
    let b = |body: &str| item_with("b", 1, |i| (i.idx, i.body) = ("b".into(), body.into()));
    let mut h = Harness::with_items(vec![at("a", "a", 1), b("b2")]);
    h.model.undo.record_update("body", &b("b1"), &b("b2"), 0);
    h
}

// 移動の答えが届く前のCtrl+Zは、答えが届いてから、その移動を戻す。前の操作は戻さない。
#[test]
fn an_undo_pressed_before_the_answer_of_a_move_undoes_that_move_when_the_answer_is_there() {
    let mut h = harness_after_an_edit_of_b();
    h.hold("MoveItems");
    h.api
        .answer_next("MoveItems", Ok(ApiOutput::Items(vec![at("a", "c", 1)])));
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });

    h.send(Event::Undo);
    assert!(batch_calls(&h.api).is_empty());

    h.release_held();
    assert_eq!(h.items_ids(), ["a", "b"]);
    assert_eq!(moved_ids(&h), [["a".to_string()]]);
    assert_eq!(h.bodies(), ["a=", "b=b2"]);
}

// タスクの切り替えの答えが届く前のCtrl+Zは、答えが届いてから、その切り替えを戻す。前の操作は戻さない。
#[test]
fn an_undo_pressed_before_the_answer_of_a_task_cycle_undoes_that_cycle_when_the_answer_is_there() {
    let mut h = harness_after_an_edit_of_b();
    h.hold("CycleTask");
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });

    h.send(Event::Undo);
    assert!(batch_calls(&h.api).is_empty());

    h.release_held();
    assert!(!h.model.state.get_item_by_id("a").unwrap().is_task());
    assert_eq!(h.bodies(), ["a=", "b=b2"]);
    assert_eq!(
        task_states_sent(&h),
        [("a".to_string(), TaskState::default())]
    );
}

// サーバーが移動を断ると、答えを待っていたCtrl+Zは何も戻さない。戻すはずだった移動は、起きていないためである。
#[test]
fn an_undo_that_waited_for_a_refused_move_undoes_nothing() {
    let mut h = harness_after_an_edit_of_b();
    h.hold("MoveItems");
    h.api.fail_next("MoveItems", "refused");
    h.send(Event::MoveDown {
        item_id: "a".into(),
    });
    h.send(Event::Undo);

    h.release_held();
    assert!(batch_calls(&h.api).is_empty());
    assert_eq!(h.bodies(), ["a=", "b=b2"]);
}

// サーバーがタスクの切り替えを断ると、答えを待っていたCtrl+Zは何も戻さない。そのあとのCtrl+Zは、前の操作を戻す。
#[test]
fn an_undo_that_waited_for_a_refused_task_cycle_undoes_nothing() {
    let mut h = harness_after_an_edit_of_b();
    h.hold("CycleTask");
    h.api.fail_next("CycleTask", "refused");
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    h.send(Event::Undo);

    h.release_held();
    assert!(batch_calls(&h.api).is_empty());

    h.send(Event::Undo);
    assert_eq!(h.bodies(), ["a=", "b=b1"]);
}
