use crate::app::Event;
use crate::effects::{ApiOp, UiOp};
use crate::harness::{create_calls, item, move_calls, Harness};
use crate::testing::item_with;
use std::collections::HashSet;

// タスク状態の切り替えで次の状態が保存され、Undo に記録される。
#[test]
fn cycle_task_saves_the_next_task_state_and_records_it_for_undo() {
    let mut h = Harness::with_items(vec![item("a", 1, "do it")]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    assert!(h.api.called("CycleTask"));
    assert!(h.model.state.get_item_by_id("a").unwrap().is_task());
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "cycle-task");
}

// 日次ジャーナル上の項目がタスクになると、その日の日付が plan プロパティとして保存される。
#[test]
fn an_item_that_becomes_a_task_on_a_daily_journal_is_planned_for_that_day() {
    let mut h = Harness::with_items(vec![item_with("a", 1, |i| {
        i.item_type = "journal".into();
        i.name = "2026-02-10".into();
    })]);
    h.send(Event::TaskCycle {
        item_id: "a".into(),
    });
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertItemProperty { key, value, .. } if key == "plan" && value == "2026-02-10")));
    assert_eq!(
        h.model.state.get_item_by_id("a").unwrap().prop_str("plan"),
        Some("2026-02-10")
    );
}

// ズームの変更で倍率が表示に反映されて保存され、リセットで 1.0 に戻る。
#[test]
fn set_zoom_shows_and_saves_the_factor() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::ZoomStep {
        item_id: "a".into(),
        up: true,
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.1);
    assert!(h.api.calls.iter().any(|c| matches!(c, ApiOp::UpsertItemProperty { key, value, .. } if key == "zoom" && value == "1.1")));
    h.send(Event::ResetZoom {
        item_id: "a".into(),
    });
    assert_eq!(h.model.state.get_item_by_id("a").unwrap().zoom(), 1.0);
}

// 子を持つ項目の折りたたみ切り替えで項目が折りたたまれて保存され、Undo に記録される。
#[test]
fn toggle_collapsed_collapses_an_item_with_children_saves_it_and_records_it_for_undo() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 2, "")]);
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });
    assert!(h.model.state.is_collapsed("a"));
    assert_eq!(h.model.state.visible_items().len(), 1);
    assert!(h.api.calls.iter().any(|c| matches!(
        c,
        ApiOp::UpdateItem {
            collapsed: Some(true),
            ..
        }
    )));
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "collapse");
}

// 折りたたみの保存が失敗すると画面が元の展開状態に戻される。
#[test]
fn toggle_collapsed_puts_the_screen_back_when_the_save_fails() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 2, "")]);
    h.api.fail_next("UpdateItem", "offline");
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });
    assert!(!h.model.state.is_collapsed("a"));
    assert_eq!(h.model.state.visible_items().len(), 2);
}

// 子を持たない項目では折りたたみ切り替えが何もしない。
#[test]
fn toggle_collapsed_does_nothing_for_an_item_without_children() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::CollapseToggle {
        item_id: "a".into(),
    });
    assert!(h.api.calls.is_empty());
}

// トランスクルージョンとしての貼り付けで、項目が貼り付けられた ID に結び付けられてページが再読み込みされる。
#[test]
fn paste_as_transclusion_links_the_item_to_the_pasted_id_and_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.clipboard_text = "2026-01-01-00-00-00-AAAAAAA".into();
    h.send(Event::PasteAsTransclusion {
        item_id: "a".into(),
    });
    assert!(h.api.called("SetTransclusion"));
    assert!(h.api.called("GetPageById"));
}

// 項目 ID でないテキストはトランスクルージョンとしての貼り付けで無視される。
#[test]
fn paste_as_transclusion_ignores_text_that_is_not_an_item_id() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.clipboard_text = "hello world".into();
    h.send(Event::PasteAsTransclusion {
        item_id: "a".into(),
    });
    assert!(!h.api.called("SetTransclusion"));
}

// Stash への送付で Stash 見出しがページ末尾に作られ、部分木がそのまま（タグなしで）その下へ移動される。
#[test]
fn sending_to_the_stash_creates_its_heading_at_the_end_and_moves_the_subtree_under_it() {
    let mut h = Harness::with_items(vec![
        item("a", 1, "todo"),
        item("a1", 2, ""),
        item("z", 1, ""),
    ]);
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert!(
        !h.api.called("UpdateItem"),
        "the item is moved as it is, with no tag"
    );
    assert_eq!(
        create_calls(&h.api),
        [("page1".into(), "after:z".into(), 1, "# Stash".into(), None)]
    );
    assert_eq!(
        move_calls(&h.api),
        [(
            vec!["a".to_string(), "a1".to_string()],
            "page1".to_string(),
            "after:new1".to_string()
        )]
    );
}

// Stash 見出しが既にあれば、Stash への送付で新しい見出しは作られずその直後へ移動される。
#[test]
fn sending_to_the_stash_reuses_an_existing_stash_heading() {
    let mut h = Harness::with_items(vec![item("a", 1, "todo"), item("stash", 1, "# Stash")]);
    h.send(Event::SendToStash {
        item_id: "a".into(),
    });
    assert!(!h.api.called("CreateItem"));
    assert_eq!(move_calls(&h.api)[0].2, "after:stash");
}

// 選択範囲の削除で項目が一つの Undo エントリとして削除され、ページが空にならないよう空の項目が一つ作られる。
#[test]
fn deleting_the_selection_removes_the_items_as_one_undo_entry_and_keeps_one_item_on_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model
        .state
        .set_selected_item_ids(HashSet::from(["a".to_string(), "b".to_string()]));
    h.model.state.set_item_selection_mode(true);
    h.send(Event::SelectionKey(crate::app::SelectionKey::Delete));
    assert_eq!(h.api.calls_named("DeleteItem").len(), 2);
    assert!(!h.model.state.item_selection_mode());
    assert_eq!(
        h.items_ids(),
        ["new1"],
        "an empty item was created so the page is not empty"
    );
    assert_eq!(h.model.undo.pop_undo().unwrap().label, "delete-selection");
}

// 選択範囲のコピーでツリー形式の Markdown が書き出されてクリップボードに渡され、フラット形式がモデルに記憶される。
#[test]
fn copying_the_selection_exports_tree_markdown_and_remembers_the_flat_form() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model
        .state
        .set_selected_item_ids(HashSet::from(["a".to_string()]));
    h.model.state.set_item_selection_mode(true);
    h.send(Event::SelectionKey(crate::app::SelectionKey::Copy));
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
    h.model
        .state
        .set_selected_item_ids(HashSet::from(["a".to_string()]));
    h.model.state.set_item_selection_mode(true);
    h.api.fail_next("ExportTreeMarkdown", "boom");
    h.send(Event::SelectionKey(crate::app::SelectionKey::Copy));
    assert!(h.ui.iter().any(|u| matches!(u, UiOp::CopyFailed)));
    assert!(!h.ui.iter().any(|u| matches!(u, UiOp::CopyMarkdown { .. })));
    assert_eq!(h.model.clipboard, None);
}

fn select_with_transclusion(key: crate::app::SelectionKey) -> Harness {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![item("a", 1, "x"), transclusion]);
    h.model
        .state
        .set_selected_item_ids(HashSet::from(["a".to_string(), "t".to_string()]));
    h.model.state.set_item_selection_mode(true);
    h.send(Event::SelectionKey(key));
    h
}

// トランスクルージョンを含む選択範囲の削除は警告とともに拒否され、元の項目と選択状態が残る。
#[test]
fn deleting_a_selection_with_a_transclusion_is_refused_so_the_source_stays() {
    let h = select_with_transclusion(crate::app::SelectionKey::Delete);
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
    let h = select_with_transclusion(crate::app::SelectionKey::Cut);
    assert!(!h.api.called("DeleteItem"));
    assert!(!h.api.called("ExportTreeMarkdown"));
    assert!(h.ui.iter().any(|op| matches!(op, UiOp::Alert(_))));
}
