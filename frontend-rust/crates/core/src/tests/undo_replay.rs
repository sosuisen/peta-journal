use crate::app::Event;
use crate::harness::{create_calls, item, update_calls, Harness};
use crate::outline::CursorState;

// undo は古い本文を戻して保存し、その項目を redo スタックに移す。redo はそれを元に戻す。
#[test]
fn undo_puts_the_old_body_back_saves_it_and_moves_the_entry_to_the_redo_stack() {
    let mut h = Harness::with_items(vec![item("a", 1, "new")]);
    let before = item("a", 1, "old");
    let after = item("a", 1, "new");
    h.model.undo.begin("body", None);
    h.model.undo.push_update(&before, &after);
    h.model.undo.commit(None, 0);

    h.send(Event::Undo);
    assert_eq!(h.bodies(), ["a=old"]);
    assert_eq!(update_calls(&h.api), [("a".into(), Some("old".into()))]);
    assert!(h.model.undo.can_redo());
    assert!(!h.model.undo.can_undo());

    h.send(Event::Redo);
    assert_eq!(h.bodies(), ["a=new"]);
    assert!(h.model.undo.can_undo());
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
    assert!(h.api.called("DeleteItem"));
    // The recorded cursor is used
    assert_eq!(h.model.state.focused_item_id(), Some("a"));
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

// 削除の undo は項目を元の位置に作り直す。
#[test]
fn undo_of_a_delete_recreates_the_item_where_it_was() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("c", 1, "")]);
    let deleted = {
        let mut b = item("b", 1, "gone");
        b.idx = "b".into();
        b
    };
    h.model.undo.begin("delete-selection", None);
    h.model.undo.push_delete(&deleted);
    h.model.undo.commit(None, 0);
    h.send(Event::Undo);
    // Shown by its index between a and c, then replaced by the server's item
    assert_eq!(h.items_ids(), ["a", "new1", "c"]);
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
