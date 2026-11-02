//! What happens to the selected items: delete, copy, cut and move.

use crate::app::{Event, SelectionKey};
use crate::effects::UiOp;
use crate::harness::{delete_calls, item, move_calls, Harness};
use crate::testing::item_with;

fn select(h: &mut Harness, ids: &[&str]) {
    h.model
        .state
        .set_selected_item_ids(ids.iter().map(|s| s.to_string()).collect());
    h.model.state.set_item_selection_mode(true);
}

// 選択範囲の削除で項目が一つの Undo エントリとして削除され、ページが空にならないよう空の項目が一つ作られる。
// サーバーには、選んだ項目を1つのグループとして1回で送る。
#[test]
fn deleting_the_selection_removes_the_items_as_one_undo_entry_and_keeps_one_item_on_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    select(&mut h, &["a", "b"]);
    h.send(Event::SelectionKey(SelectionKey::Delete));
    assert_eq!(
        delete_calls(&h.api),
        [vec!["a".to_string(), "b".to_string()]]
    );
    assert!(!h.model.state.item_selection_mode());
    assert_eq!(
        h.items_ids(),
        ["new1"],
        "an empty item was created so the page is not empty"
    );
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "delete-selection");
}

// 選択移動では、選択されたサブツリーが一つのまとまりとして端のルートの兄弟を越えて移動する。
#[test]
fn move_selection_moves_the_selected_subtrees_as_one_group_past_the_sibling_of_the_edge_root() {
    // * a (1)
    // * b (1)
    //     * b1 (2)
    // * c (1)
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item("b", 1, ""),
        item("b1", 2, ""),
        item("c", 1, ""),
    ]);
    select(&mut h, &["b", "b1", "c"]);
    h.send(Event::SelectionKey(SelectionKey::MoveUp));
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

// 選択範囲のコピーでツリー形式の Markdown が書き出されてクリップボードに渡され、フラット形式がモデルに記憶される。
#[test]
fn copying_the_selection_exports_tree_markdown_and_remembers_the_flat_form() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    select(&mut h, &["a"]);
    h.send(Event::SelectionKey(SelectionKey::Copy));
    assert!(h.api.called("ExportTreeMarkdown"));
    assert!(h
        .ui
        .iter()
        .any(|u| matches!(u, UiOp::CopyMarkdown { markdown } if markdown == "- tree")));
    assert_eq!(h.model.clipboard, Some(("flat".into(), "- tree".into())));
}

// 書き出しに失敗したコピーはシェルに失敗が伝えられ、クリップボードへの書き込みは行われない。
#[test]
fn a_copy_whose_export_fails_tells_the_shell_so_it_can_give_up_the_clipboard_write() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    select(&mut h, &["a"]);
    h.api.fail_next("ExportTreeMarkdown", "boom");
    h.send(Event::SelectionKey(SelectionKey::Copy));
    assert!(h.ui.iter().any(|u| matches!(u, UiOp::CopyFailed)));
    assert!(!h.ui.iter().any(|u| matches!(u, UiOp::CopyMarkdown { .. })));
    assert_eq!(h.model.clipboard, None);
}

fn select_with_transclusion(key: SelectionKey) -> Harness {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![item("a", 1, "x"), transclusion]);
    select(&mut h, &["a", "t"]);
    h.send(Event::SelectionKey(key));
    h
}

// トランスクルージョンを含む選択範囲の削除は警告とともに拒否され、元の項目と選択状態が残る。
#[test]
fn deleting_a_selection_with_a_transclusion_is_refused_so_the_source_stays() {
    let h = select_with_transclusion(SelectionKey::Delete);
    assert!(!h.api.called("DeleteItem"));
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::Alert(m) if m.contains("transclusion"))));
    assert!(h.model.state.item_selection_mode(), "the selection stays");
    assert_eq!(h.items_ids(), ["a", "t"]);
}

// トランスクルージョンを含む選択範囲の切り取りも同様に拒否される。
#[test]
fn cutting_a_selection_with_a_transclusion_is_refused_too() {
    let h = select_with_transclusion(SelectionKey::Cut);
    assert!(!h.api.called("DeleteItem"));
    assert!(!h.api.called("ExportTreeMarkdown"));
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}

fn selected(h: &Harness) -> Vec<String> {
    let mut ids: Vec<String> = h.model.state.selected_item_ids().iter().cloned().collect();
    ids.sort();
    ids
}

fn key(h: &mut Harness, key: SelectionKey) {
    h.send(Event::SelectionKey(key));
}

// エディタの外へドラッグすると、その項目と子が選ばれて編集は終わる。マウスが入った項目まで選択が広がり、ボタンを離すと選択はそのまま残る。
#[test]
fn dragging_out_of_an_editor_selects_the_item_with_its_children_and_follows_the_mouse() {
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item("a1", 2, ""),
        item("b", 1, ""),
        item("c", 1, ""),
    ]);
    h.model.state.add_editing_item("a");
    h.send(Event::DragBeyondBounds {
        item_id: "a".into(),
    });
    assert_eq!(selected(&h), ["a", "a1"]);
    assert!(!h.model.state.is_editing("a"));
    assert!(h.model.selection.is_dragging());

    h.send(Event::ItemMouseEnter {
        item_id: "c".into(),
    });
    assert_eq!(selected(&h), ["a", "a1", "b", "c"]);

    h.send(Event::SelectionClickOutside);
    assert!(
        h.model.state.item_selection_mode(),
        "a click during the drag is the drag itself"
    );

    h.send(Event::SelectionMouseUp);
    assert!(!h.model.selection.is_dragging());
    assert_eq!(selected(&h), ["a", "a1", "b", "c"]);
}

// ドラッグしていないときにマウスが項目に入っても、何も選ばれない。
#[test]
fn the_mouse_entering_an_item_without_a_drag_selects_nothing() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::ItemMouseEnter {
        item_id: "b".into(),
    });
    assert!(selected(&h).is_empty());
    assert!(!h.model.state.item_selection_mode());
}

// Shift+矢印はエディタの項目だけを選んで編集を終える。Shift+↓/↑で選択は下へ上へ伸び縮みし、Escape で終わる。
#[test]
fn shift_arrows_grow_and_shrink_the_selection_from_the_editor_and_escape_ends_it() {
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item("b", 1, ""),
        item("c", 1, ""),
        item("d", 1, ""),
    ]);
    h.model.state.add_editing_item("b");
    h.send(Event::ShiftSelect {
        item_id: "b".into(),
    });
    assert_eq!(selected(&h), ["b"]);
    assert!(!h.model.state.is_editing("b"));

    key(&mut h, SelectionKey::ExtendDown);
    key(&mut h, SelectionKey::ExtendDown);
    assert_eq!(selected(&h), ["b", "c", "d"]);
    key(&mut h, SelectionKey::ExtendDown);
    assert_eq!(selected(&h), ["b", "c", "d"], "the end of the outline");

    key(&mut h, SelectionKey::ExtendUp);
    key(&mut h, SelectionKey::ExtendUp);
    assert_eq!(selected(&h), ["b"]);
    key(&mut h, SelectionKey::ExtendUp);
    assert_eq!(selected(&h), ["a", "b"]);
    key(&mut h, SelectionKey::ExtendUp);
    assert_eq!(selected(&h), ["a", "b"], "the start of the outline");

    key(&mut h, SelectionKey::Escape);
    assert!(!h.model.state.item_selection_mode());
    assert!(selected(&h).is_empty());
}

// 選択の外をクリックすると選択は終わる。選択していなければキーは何もしない。
#[test]
fn a_click_outside_ends_the_selection_and_keys_do_nothing_without_one() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    select(&mut h, &["a"]);
    h.send(Event::SelectionClickOutside);
    assert!(!h.model.state.item_selection_mode());

    key(&mut h, SelectionKey::ExtendDown);
    key(&mut h, SelectionKey::Delete);
    assert!(selected(&h).is_empty());
    assert!(!h.api.called("DeleteItem"));
}

// 選択中の Tab と Shift+Tab は、選ばれた項目をまとめて一段下げ、一段上げる。先頭の項目は下げられない。
#[test]
fn tab_indents_and_outdents_the_selected_items_together() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, ""), item("c", 1, "")]);
    select(&mut h, &["b", "c"]);
    key(&mut h, SelectionKey::Indent);
    let levels =
        |h: &Harness| -> Vec<i32> { h.model.state.items().iter().map(|i| i.level).collect() };
    assert_eq!(levels(&h), [1, 2, 2]);
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "indent-selection");

    key(&mut h, SelectionKey::Outdent);
    assert_eq!(levels(&h), [1, 1, 1]);

    select(&mut h, &["a"]);
    key(&mut h, SelectionKey::Indent);
    assert_eq!(
        levels(&h),
        [1, 1, 1],
        "the first item has nothing to go under"
    );
}

// 選択中の Alt+↓ は、選ばれた項目をまとめて下の兄弟の後ろへ動かす。
#[test]
fn moving_the_selection_down_puts_it_after_the_next_sibling() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, ""), item("c", 1, "")]);
    select(&mut h, &["a", "b"]);
    key(&mut h, SelectionKey::MoveDown);
    assert_eq!(h.items_ids(), ["c", "a", "b"]);
}

// 選択中の Ctrl+Z と Ctrl+Y は、エディタの中と同じように undo と redo をする。
#[test]
fn undo_and_redo_work_while_items_are_selected() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, ""), item("c", 1, "")]);
    select(&mut h, &["b", "c"]);
    key(&mut h, SelectionKey::Indent);
    let levels =
        |h: &Harness| -> Vec<i32> { h.model.state.items().iter().map(|i| i.level).collect() };
    assert_eq!(levels(&h), [1, 2, 2]);

    select(&mut h, &["a"]);
    key(&mut h, SelectionKey::Undo);
    assert_eq!(levels(&h), [1, 1, 1]);
    key(&mut h, SelectionKey::Redo);
    assert_eq!(levels(&h), [1, 2, 2]);
}

// 選択範囲の切り取りは、コピーした後に削除する。
#[test]
fn cutting_the_selection_copies_it_and_then_deletes_it() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    select(&mut h, &["a"]);
    key(&mut h, SelectionKey::Cut);
    assert!(h.api.called("ExportTreeMarkdown"));
    assert_eq!(delete_calls(&h.api), [vec!["a".to_string()]]);
    assert_eq!(h.items_ids(), ["b"]);
}

// サーバーが選択範囲の削除を断ると、ページはサーバーの持つ形に読み直される。
#[test]
fn a_selection_delete_the_server_refuses_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    select(&mut h, &["a"]);
    h.api.fail_next("DeleteItems", "refused");
    key(&mut h, SelectionKey::Delete);
    assert!(h.api.called("GetPageById"));
}

// 子を持つ項目まで選択を伸ばすと、その子も入る。伸ばし続けると、入っている子は飛ばして次へ進む。
#[test]
fn extending_over_a_parent_takes_its_children_and_skips_them_on_the_next_step() {
    // * a (1)
    // * b (1)
    //     * b1 (2)
    //     * b2 (2)
    // * c (1)
    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item("b", 1, ""),
        item("b1", 2, ""),
        item("b2", 2, ""),
        item("c", 1, ""),
    ]);
    h.model.state.add_editing_item("a");
    h.send(Event::ShiftSelect {
        item_id: "a".into(),
    });
    key(&mut h, SelectionKey::ExtendDown);
    assert_eq!(selected(&h), ["a", "b", "b1", "b2"]);
    key(&mut h, SelectionKey::ExtendDown);
    assert_eq!(selected(&h), ["a", "b", "b1", "b2", "c"]);

    let mut h = Harness::with_items(vec![
        item("a", 1, ""),
        item("b", 1, ""),
        item("b1", 2, ""),
        item("b2", 2, ""),
        item("c", 1, ""),
    ]);
    h.model.state.add_editing_item("c");
    h.send(Event::ShiftSelect {
        item_id: "c".into(),
    });
    key(&mut h, SelectionKey::ExtendUp);
    assert_eq!(selected(&h), ["b", "b1", "b2", "c"]);
    key(&mut h, SelectionKey::ExtendUp);
    assert_eq!(selected(&h), ["a", "b", "b1", "b2", "c"]);
}
