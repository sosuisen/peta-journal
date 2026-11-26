//! What happens to the selected items: delete, copy, cut and move.

use crate::app::{Event, SelectionKey};
use crate::effects::{ApiOutput, UiOp};
use crate::harness::{batch_calls, delete_calls, item, move_calls, op_name, Harness};
use crate::model::TranscludedTrees;
use crate::testing::item_with;

fn select(h: &mut Harness, ids: &[&str]) {
    h.model
        .state
        .set_selected_item_ids(ids.iter().map(|s| s.to_string()).collect());
    h.model.state.set_item_selection_mode(true);
}

// トランスクルージョンが見せる元のページの項目を削除すると、元のページで削除され、画面からも消える（#736）。
#[test]
fn a_deleted_item_that_a_transclusion_shows_is_gone_from_the_screen() {
    let root = item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into());
        i.body = "shown".into();
    });
    let mut h = Harness::with_items(vec![root]);
    let source = |id: &str, level: i32| {
        item_with(id, level, |i| {
            i.page_id = "src".into();
            i.body = id.into();
        })
    };
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![source("s", 1), source("c", 2)]);
    h.model.state.set_transcluded_trees(trees);

    h.send(Event::DeleteItem {
        item_id: "c".into(),
    });

    assert_eq!(delete_calls(&h.api), [vec!["c".to_string()]]);
    let shown: Vec<_> = h
        .model
        .state
        .visible_items()
        .iter()
        .map(|i| i.id.clone())
        .collect();
    assert_eq!(shown, ["root"]);
}

// 選択範囲の削除で項目が一つの Undo エントリとして削除され、ページが空にならないようサーバーが作った空の項目が表示される。
// サーバーには、選んだ項目を1つのグループとして1回で送る。空の項目を作る要求は送らない。
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
        "the empty item the server made is shown so the page is not empty"
    );
    assert!(!h.api.called("CreateItem"));
    assert_eq!(h.model.state.focused_item_id(), Some("new1"));
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "delete-selection");
}

// ページの項目をすべて削除した直後は、サーバーが作った空の項目が編集中になる。そのまま打てる。
#[test]
fn the_empty_item_left_after_deleting_every_item_is_edited() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    select(&mut h, &["a", "b"]);

    h.send(Event::SelectionKey(SelectionKey::Delete));

    let rows = h.view().outline.rows;
    assert_eq!(rows.len(), 1);
    assert!(rows[0].is_editing);
}

/// "a" and "b" selected, with "a" transcluded by an item on another page
fn select_with_a_transcluded_item(confirm: bool) -> Harness {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.confirm = confirm;
    h.api
        .answer_next("TranscludedItems", Ok(ApiOutput::ItemIds(vec!["a".into()])));
    select(&mut h, &["a", "b"]);
    h
}

fn destructive_confirms(h: &Harness) -> Vec<String> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::ConfirmDestructive(m) => Some(m.clone()),
            _ => None,
        })
        .collect()
}

// 他の項目からトランスクルードされている項目を含む選択の削除は、確認でOKのときだけ行われる。
#[test]
fn deleting_a_selection_with_a_transcluded_item_asks_first_and_deletes_on_ok() {
    let mut h = select_with_a_transcluded_item(true);
    key(&mut h, SelectionKey::Delete);

    assert_eq!(
        destructive_confirms(&h),
        ["1 item to delete is transcluded by another item. Delete anyway? Each transclusion keeps a plain copy."]
    );
    assert_eq!(
        delete_calls(&h.api),
        [vec!["a".to_string(), "b".to_string()]]
    );
    assert!(!h.model.state.item_selection_mode());
}

// 確認でキャンセルすると、何も消えず、選択状態も残る。
#[test]
fn a_cancel_to_the_confirmation_deletes_nothing_and_keeps_the_selection() {
    let mut h = select_with_a_transcluded_item(false);
    key(&mut h, SelectionKey::Delete);

    assert_eq!(destructive_confirms(&h).len(), 1);
    assert!(delete_calls(&h.api).is_empty());
    assert_eq!(h.items_ids(), ["a", "b"]);
    assert!(h.model.state.item_selection_mode(), "the selection stays");
}

// 切り取りも、確認でキャンセルすると何も消えない。
#[test]
fn a_cut_of_a_transcluded_item_is_asked_about_too() {
    let mut h = select_with_a_transcluded_item(false);
    key(&mut h, SelectionKey::Cut);

    assert_eq!(destructive_confirms(&h).len(), 1);
    assert!(delete_calls(&h.api).is_empty());
    assert_eq!(h.items_ids(), ["a", "b"]);
}

// 複数の項目がトランスクルードされていれば、確認はその数を伝える。
#[test]
fn the_confirmation_tells_how_many_items_are_transcluded() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.api.answer_next(
        "TranscludedItems",
        Ok(ApiOutput::ItemIds(vec!["a".into(), "b".into()])),
    );
    select(&mut h, &["a", "b"]);
    key(&mut h, SelectionKey::Delete);

    assert!(destructive_confirms(&h)[0]
        .starts_with("2 items to delete are transcluded by other items."));
}

// トランスクルードされている項目がなければ、確認なしに削除される。
#[test]
fn a_selection_no_other_item_transcludes_is_deleted_without_a_question() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.confirm = false;
    select(&mut h, &["a"]);
    key(&mut h, SelectionKey::Delete);

    assert!(h.api.called("TranscludedItems"));
    assert!(destructive_confirms(&h).is_empty());
    assert_eq!(delete_calls(&h.api), [vec!["a".to_string()]]);
}

// サーバーに尋ねるのに失敗したら、確認なしに削除する。
#[test]
fn a_failed_question_to_the_server_deletes_without_a_confirmation() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.confirm = false;
    h.api.fail_next("TranscludedItems", "offline");
    select(&mut h, &["a"]);
    key(&mut h, SelectionKey::Delete);

    assert!(destructive_confirms(&h).is_empty());
    assert_eq!(delete_calls(&h.api), [vec!["a".to_string()]]);
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

// 選択範囲の移動をサーバーが断ると、画面は変わらない。そのあとの移動は、いつもどおり始まる。
#[test]
fn a_move_of_the_selection_the_server_refuses_changes_nothing_and_the_next_move_starts() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, ""), item("c", 1, "")]);
    select(&mut h, &["b"]);
    h.api.fail_next("MoveItems", "offline");
    h.send(Event::SelectionKey(SelectionKey::MoveUp));
    assert_eq!(h.items_ids(), ["a", "b", "c"]);
    assert!(h.model.undo.pop_undo().is_none());

    h.send(Event::SelectionKey(SelectionKey::MoveDown));
    assert_eq!(h.items_ids(), ["a", "c", "b"]);
}

// 選択範囲のコピーでツリー形式の Markdown が書き出され、コアが作ったそのASTとともにクリップボードに渡され、フラット形式がモデルに記憶される。
#[test]
fn copying_the_selection_hands_over_tree_markdown_with_its_ast_and_remembers_the_flat_form() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    select(&mut h, &["a"]);
    h.send(Event::SelectionKey(SelectionKey::Copy));
    assert!(h.api.called("ExportTreeMarkdown"));
    assert!(h.ui.iter().any(|u| matches!(
        u,
        UiOp::CopyMarkdown { markdown, ast }
            if markdown == "- tree" && *ast == crate::asts::parse("- tree").to_json()
    )));
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

/// Own items "a" and "t", where "t" transcludes "src" on another page. The tree of "src"
/// shows its child "c" under "t". The selection holds "a", "t" and "c", as a range does.
fn select_with_transclusion(key: SelectionKey) -> Harness {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![item("a", 1, "x"), transclusion]);
    let mut trees = TranscludedTrees::new();
    trees.insert("src".into(), vec![item("src", 1, "y"), item("c", 2, "z")]);
    h.model.state.set_transcluded_trees(trees);
    select(&mut h, &["a", "t", "c"]);
    h.send(Event::SelectionKey(key));
    h
}

fn position(h: &Harness, name: &str) -> usize {
    h.api
        .calls
        .iter()
        .position(|op| op_name(op) == name)
        .unwrap_or_else(|| panic!("no {name} call"))
}

// 選択にトランスクルージョンがあれば、解除の要求は送らず、トランスクルージョン自身を1回の削除で送る。
// 元の項目とその子は、元のページの項目なので削除しない。
#[test]
fn deleting_a_selected_transclusion_sends_it_in_the_one_delete_and_keeps_the_source() {
    let h = select_with_transclusion(SelectionKey::Delete);
    assert!(!h.api.called("RemoveTransclusion"));
    assert_eq!(
        delete_calls(&h.api),
        [vec!["a".to_string(), "t".to_string()]]
    );
    assert!(!h.model.state.item_selection_mode());
    assert_eq!(h.items_ids(), ["new1"]);
}

// 切り取りも同じで、コピーの後に、トランスクルージョン自身を1回の削除で送る。
#[test]
fn cutting_a_selected_transclusion_sends_it_in_the_one_delete_and_keeps_the_source() {
    let h = select_with_transclusion(SelectionKey::Cut);
    assert!(position(&h, "ExportTreeMarkdown") < position(&h, "DeleteItems"));
    assert!(!h.api.called("RemoveTransclusion"));
    assert_eq!(
        delete_calls(&h.api),
        [vec!["a".to_string(), "t".to_string()]]
    );
}

// 確認でキャンセルすると、トランスクルージョンは解除されない。
#[test]
fn a_cancel_leaves_the_selected_transclusion_as_it_is() {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![item("a", 1, "x"), transclusion]);
    h.confirm = false;
    h.api
        .answer_next("TranscludedItems", Ok(ApiOutput::ItemIds(vec!["a".into()])));
    select(&mut h, &["a", "t"]);
    h.send(Event::SelectionKey(SelectionKey::Delete));
    assert!(!h.api.called("RemoveTransclusion"));
    assert!(!h.api.called("DeleteItems"));
}

// 折りたたまれた親を消すと、隠れた子も一緒にサーバーへ送られる。
#[test]
fn deleting_a_collapsed_parent_sends_its_hidden_children_too() {
    let mut h = Harness::with_items(vec![
        item_with("p", 1, |i| i.collapsed = true),
        item("q", 2, "hidden"),
        item("r", 1, ""),
    ]);
    select(&mut h, &["p", "q"]);
    h.send(Event::SelectionKey(SelectionKey::Delete));
    assert_eq!(
        delete_calls(&h.api),
        [vec!["p".to_string(), "q".to_string()]]
    );
    assert_eq!(h.items_ids(), ["r"]);
}

// トランスクルージョンの自分の子は、トランスクルージョンと一緒に削除される。
#[test]
fn a_selected_transclusion_goes_with_its_own_children() {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![transclusion, item("own", 2, ""), item("z", 1, "")]);
    let mut trees = TranscludedTrees::new();
    trees.insert("src".into(), vec![item("src", 1, "y"), item("c", 2, "z")]);
    h.model.state.set_transcluded_trees(trees);
    select(&mut h, &["t", "c", "own"]);
    h.send(Event::SelectionKey(SelectionKey::Delete));
    assert_eq!(
        delete_calls(&h.api),
        [vec!["t".to_string(), "own".to_string()]]
    );
    assert_eq!(h.items_ids(), ["z"]);
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

// 選択範囲を削除した直後は、選択もエディタもないが、Ctrl+Zで削除が元に戻り、Ctrl+Yでまた消える。
#[test]
fn undo_and_redo_right_after_deleting_the_selection_bring_it_back_and_take_it_again() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    select(&mut h, &["a"]);
    key(&mut h, SelectionKey::Delete);
    assert_eq!(h.items_ids(), ["b"]);
    assert!(!h.model.state.item_selection_mode());

    key(&mut h, SelectionKey::Undo);
    assert_eq!(h.items_ids(), ["a", "b"]);
    key(&mut h, SelectionKey::Redo);
    assert_eq!(h.items_ids(), ["b"]);
}

// 選択もなく、項目を編集しているときは、選択のキーのCtrl+Zは何もしない。エディタが自分でUndoを送る。
#[test]
fn a_selection_undo_while_an_item_is_edited_does_nothing() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    select(&mut h, &["a"]);
    key(&mut h, SelectionKey::Delete);
    h.model.state.add_editing_item("b");

    key(&mut h, SelectionKey::Undo);

    assert_eq!(h.items_ids(), ["b"]);
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

// サーバーが選択範囲の削除を断ると、何も消えず、選択もUndoも変わらない。画面は変えていないので、ページは読み直さない。
#[test]
fn a_selection_delete_the_server_refuses_changes_nothing() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    select(&mut h, &["a"]);
    h.api.fail_next("DeleteItems", "refused");
    key(&mut h, SelectionKey::Delete);
    assert_eq!(h.items_ids(), ["a", "b"]);
    assert!(h.model.state.item_selection_mode(), "the selection stays");
    assert!(h.model.undo.pop_undo().is_none());
    assert!(!h.api.called("GetPageById"));
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

// 1つの項目の削除は、選択を経ずに、子ごと1回でサーバーへ送られる。
#[test]
fn deleting_an_item_sends_it_with_its_children_without_selecting_them() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "親"),
        item("b", 2, "子"),
        item("c", 1, ""),
    ]);

    h.send(Event::DeleteItem {
        item_id: "a".into(),
    });

    assert!(h.api.called("TranscludedItems"));
    assert_eq!(
        delete_calls(&h.api),
        [vec!["a".to_string(), "b".to_string()]]
    );
    assert_eq!(h.items_ids(), ["c"]);
    assert!(!h.model.state.item_selection_mode());
}

// 編集中の項目を消すと、その編集も終わる。
#[test]
fn deleting_the_edited_item_ends_its_edit() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.model.state.add_editing_item("a");

    h.send(Event::DeleteItem {
        item_id: "a".into(),
    });

    assert_eq!(h.items_ids(), ["b"]);
    assert!(!h.model.state.is_editing("a"));
}

// サーバーが1つの項目の削除を断ると、何も消えず、編集は続き、選択にもならない。
#[test]
fn a_delete_of_an_item_the_server_refuses_changes_nothing() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.model.state.add_editing_item("a");
    h.api.fail_next("DeleteItems", "refused");

    h.send(Event::DeleteItem {
        item_id: "a".into(),
    });

    assert_eq!(h.items_ids(), ["a", "b"]);
    assert!(h.model.state.is_editing("a"));
    assert!(!h.model.state.item_selection_mode());
    assert!(h.model.undo.pop_undo().is_none());
}

// サーバーが削除を断ると、その理由が出る。理由がなければ既定の文が出る。
#[test]
fn a_refused_delete_shows_the_reason_or_a_default_message() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.api.fail_next("DeleteItems", "disk full");
    h.send(Event::DeleteItem {
        item_id: "a".into(),
    });
    select(&mut h, &["b"]);
    h.api.fail_next("DeleteItems", "");
    key(&mut h, SelectionKey::Delete);

    assert_eq!(h.alerts(), ["disk full", "Failed to delete items"]);
}

// 他の項目からトランスクルードされている項目の削除を断ると、何も消えず、編集は続き、選択にもならない。
#[test]
fn a_no_to_deleting_a_transcluded_item_changes_nothing() {
    let mut h = Harness::with_items(vec![item("a", 1, "x"), item("b", 1, "y")]);
    h.confirm = false;
    h.api
        .answer_next("TranscludedItems", Ok(ApiOutput::ItemIds(vec!["a".into()])));
    h.model.state.add_editing_item("a");

    h.send(Event::DeleteItem {
        item_id: "a".into(),
    });

    assert!(delete_calls(&h.api).is_empty());
    assert!(h.model.state.is_editing("a"));
    assert!(!h.model.state.item_selection_mode());
}

// もうない項目の削除は、何も送らない。ほかのクライアントが先に消したときに起きる。
#[test]
fn deleting_an_item_that_is_gone_sends_nothing() {
    let mut h = Harness::with_items(vec![item("a", 1, "x")]);

    h.send(Event::DeleteItem {
        item_id: "gone".into(),
    });

    assert!(!h.api.called("TranscludedItems"));
    assert!(delete_calls(&h.api).is_empty());
    assert_eq!(h.items_ids(), ["a"]);
}

/// Items `a` to `d`, with `b` and `c` selected.
fn selection_harness() -> Harness {
    let mut h = Harness::with_items(vec![
        item("a", 1, "a"),
        item("b", 1, "b"),
        item("c", 1, "c"),
        item("d", 1, "d"),
    ]);
    select(&mut h, &["b", "c"]);
    h
}

/// True when `ids` are selected, and no editor is open or asked for.
fn still_selected(h: &Harness, ids: &[&str]) -> bool {
    let selected: std::collections::HashSet<String> = ids.iter().map(|s| s.to_string()).collect();
    h.model.state.item_selection_mode()
        && *h.model.state.selected_item_ids() == selected
        && h.model.state.editing_item_ids().is_empty()
        && h.focused_editor_requests().is_empty()
}

// 選択した項目の移動を元に戻しても、選択はそのまま残り、エディタは開かない。続けて選んだまま操作できる（#739）。
#[test]
fn the_undo_of_a_move_of_the_selection_keeps_the_selection_and_opens_no_editor() {
    let mut h = selection_harness();
    h.send(Event::SelectionKey(SelectionKey::MoveUp));

    h.send(Event::SelectionKey(SelectionKey::Undo));

    assert!(h.api.called("ApplyBatch"), "the move is undone");
    assert!(still_selected(&h, &["b", "c"]));
}

// 選択した項目の移動をやり直しても、選択はそのまま残り、エディタは開かない（#739）。
#[test]
fn the_redo_of_a_move_of_the_selection_keeps_the_selection_and_opens_no_editor() {
    let mut h = selection_harness();
    h.send(Event::SelectionKey(SelectionKey::MoveUp));
    h.send(Event::SelectionKey(SelectionKey::Undo));

    h.send(Event::SelectionKey(SelectionKey::Redo));

    assert_eq!(batch_calls(&h.api).len(), 2, "the move is done again");
    assert!(still_selected(&h, &["b", "c"]));
}

// 選択した項目のインデントとアウトデントを元に戻しても、選択はそのまま残り、エディタは開かない（#739）。
#[test]
fn the_undo_of_an_indent_or_outdent_of_the_selection_keeps_the_selection_and_opens_no_editor() {
    for key in [SelectionKey::Indent, SelectionKey::Outdent] {
        let mut h = selection_harness();
        if key == SelectionKey::Outdent {
            h.send(Event::SelectionKey(SelectionKey::Indent));
        }
        h.send(Event::SelectionKey(key));

        h.send(Event::SelectionKey(SelectionKey::Undo));

        assert!(still_selected(&h, &["b", "c"]), "{key:?}");
    }
}
