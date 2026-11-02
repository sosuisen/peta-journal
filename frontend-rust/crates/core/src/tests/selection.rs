//! What happens to the selected items: delete, copy, cut and move.

use crate::app::{Event, SelectionKey};
use crate::effects::UiOp;
use crate::harness::{item, move_calls, Harness};
use crate::testing::item_with;

fn select(h: &mut Harness, ids: &[&str]) {
    h.model
        .state
        .set_selected_item_ids(ids.iter().map(|s| s.to_string()).collect());
    h.model.state.set_item_selection_mode(true);
}

// 選択範囲の削除で項目が一つの Undo エントリとして削除され、ページが空にならないよう空の項目が一つ作られる。
#[test]
fn deleting_the_selection_removes_the_items_as_one_undo_entry_and_keeps_one_item_on_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    select(&mut h, &["a", "b"]);
    h.send(Event::SelectionKey(SelectionKey::Delete));
    assert_eq!(h.api.calls_named("DeleteItem").len(), 2);
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
