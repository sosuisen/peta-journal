use crate::app::Event;
use crate::harness::{item, typed, update_calls, Harness};
use crate::model::TranscludedTrees;
use crate::testing::item_with;

// 入力したテキストはすぐに表示され、自動保存は最後の入力から一定時間後に一度だけ行われる。
#[test]
fn typing_shows_the_text_at_once_and_schedules_the_auto_save() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hel");
    typed(&mut h, "a", "hello");
    assert_eq!(h.bodies(), ["a=hello"]);
    assert!(
        update_calls(&h.api).is_empty(),
        "nothing is sent before the pause"
    );
    assert_eq!(
        h.pending_timer_ids().len(),
        1,
        "the second keystroke replaced the first timer"
    );
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.model.edits.pending_saves.is_empty());
}

// トランスクルージョンのルート項目への入力は、元の項目に対する保存として予約される。
#[test]
fn a_transclusion_root_schedules_the_save_for_its_source_item() {
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "src")]);
    h.model.state.set_transcluded_trees(trees);
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "typed");
    h.fire_timers();
    assert_eq!(update_calls(&h.api), [("s".into(), Some("typed".into()))]);
}

// 編集終了で入力したテキストが一度だけ保存され、自動保存のタイマーが取り消されてエディタが閉じられる。
#[test]
fn finish_edit_saves_typed_text_once_cancelling_the_auto_save_and_closes_the_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "hello");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some("hello".into()))]);
    assert!(h.timers.is_empty(), "the auto-save timer was cancelled");
    assert!(!h.model.state.is_editing("a"));
    h.fire_timers();
    assert_eq!(update_calls(&h.api).len(), 1, "the text is not sent twice");
}

// 変更のない項目の編集終了では保存されずにエディタが閉じられる。
#[test]
fn finish_edit_closes_the_editor_of_a_clean_item_without_a_save() {
    let mut h = Harness::with_items(vec![item("a", 1, "x")]);
    h.model.state.add_editing_item("a");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(h.api.calls.is_empty());
    assert!(!h.model.state.is_editing("a"));
}

// 編集終了で開いたままのコードフェンスが閉じられて保存される。その後に何も入力されていなくても同様である。
#[test]
fn finish_edit_closes_an_open_code_fence_even_when_nothing_was_typed_since() {
    let mut h = Harness::with_items(vec![item("a", 1, "```js")]);
    h.model.state.add_editing_item("a");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(
        update_calls(&h.api),
        [("a".into(), Some("```js\n```".into()))]
    );
}

// 引き継ぎがその項目を保存している間は、編集終了が無視される。
#[test]
fn finish_edit_is_ignored_while_a_handover_is_saving_the_item_itself() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    h.model.edits.edit_handovers = 1;
    typed(&mut h, "a", "hello");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(update_calls(&h.api).is_empty());
    assert!(h.model.state.is_editing("a"));
}

// 別の項目をクリックすると変更中の項目が保存され、クリックされた項目のエディタが開かれる。
#[test]
fn clicking_another_item_saves_the_dirty_one_and_opens_the_clicked_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.send(Event::EditRequested {
        item_id: "a".into(),
        cursor: 0,
    });
    typed(&mut h, "a", "typed");
    h.send(Event::EditRequested {
        item_id: "b".into(),
        cursor: 0,
    });
    assert_eq!(update_calls(&h.api), [("a".into(), Some("typed".into()))]);
    assert!(!h.model.state.is_editing("a"));
    assert!(h.model.state.is_editing("b"));
    assert_eq!(h.model.edits.edit_handovers, 0);
}

// 矢印キーでの移動で項目が保存され、カーソル桁を保ったままエディタが移動先の項目に移される。
#[test]
fn arrow_navigation_saves_and_moves_the_editor_with_the_cursor_column() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model.state.add_editing_item("a");
    typed(&mut h, "a", "x");
    h.send(Event::NavigateDown {
        item_id: "a".into(),
        cursor: 3,
    });
    assert!(h.model.state.is_editing("b"));
    assert!(!h.model.state.is_editing("a"));
    assert_eq!(h.focus_cursor(), Some(3));
    assert_eq!(update_calls(&h.api), [("a".into(), Some("x".into()))]);
}

// 上矢印は、カーソル桁を保ったままエディタを上の項目に移す。先頭の項目では何も起きない。
#[test]
fn arrow_up_moves_the_editor_to_the_item_above_and_stops_at_the_top() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "")]);
    h.model.state.add_editing_item("b");
    h.send(Event::NavigateUp {
        item_id: "b".into(),
        cursor: 2,
    });
    assert!(h.model.state.is_editing("a"));
    assert!(!h.model.state.is_editing("b"));
    assert_eq!(h.focus_cursor(), Some(2));

    h.send(Event::NavigateUp {
        item_id: "a".into(),
        cursor: 2,
    });
    assert!(h.model.state.is_editing("a"));
}

// 編集を終えると、開いていたページ名のポップアップも閉じる。
#[test]
fn finish_edit_closes_the_wiki_popup() {
    let mut h = Harness::with_items(vec![item("a", 1, "[[To]]")]);
    h.model.state.add_editing_item("a");
    h.model.wiki.item_id = Some("a".into());
    h.model.wiki.suggestions = vec!["Todo".into()];
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert_eq!(h.view().outline.wiki.item_id, None);
}

// トランスクルージョンのルートの保存で、サーバーが返した構文木と種類は、表示しているルートにも写される。
#[test]
fn saving_a_transclusion_root_copies_the_parsed_result_to_the_shown_root() {
    let mut h = Harness::with_items(vec![item_with("root", 1, |i| {
        i.transclude_item_id = Some("s".into())
    })]);
    let mut trees = TranscludedTrees::new();
    trees.insert("s".into(), vec![item("s", 1, "src")]);
    h.model.state.set_transcluded_trees(trees);
    h.model.state.add_editing_item("root");
    typed(&mut h, "root", "# typed");
    h.api.answer_next(
        "UpdateItem",
        Ok(crate::effects::ApiOutput::Item(item_with("s", 1, |i| {
            i.body = "# typed".into();
            i.ast = "parsed".into();
            i.types = vec!["heading".into()];
        }))),
    );
    h.send(Event::EditFinished {
        item_id: "root".into(),
    });
    let root = h.model.state.get_item_by_id("root").unwrap();
    assert_eq!(root.ast, "parsed");
    assert_eq!(root.types, ["heading"]);
}
