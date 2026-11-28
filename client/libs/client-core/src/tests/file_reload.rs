//! Reloading the shown page when its Markdown file changes on disk.

use crate::app::Event;
use crate::harness::{create_calls, item, typed, Harness};
use crate::model::PageWithItems;
use crate::ops::file_reload::AskedAt;
use crate::rest::ClientId;

/// The server tells that the Markdown file of the shown page changed on disk.
fn file_changed(h: &mut Harness) {
    h.send(Event::Sse(crate::effects::SseEvent::Message(
        r#"{"type":"pagesUpdated","pageId":"page1","source":"file"}"#.into(),
    )));
}

/// The next reading of the page answers with these items, as the server has them now.
fn server_has(h: &mut Harness, items: Vec<crate::model::Item>) {
    h.api.answer_next(
        "GetPageById",
        Ok(crate::effects::ApiOutput::Page(PageWithItems {
            items,
            ..Default::default()
        })),
    );
}

// 編集中でも、ファイルの変更の知らせでページはすぐに読み直され、ほかの項目はサーバーの版になる。カーソルはほぼいつもどこかの項目にあるので、編集の終わりを待つと変更が出ない（#731）。
#[test]
fn a_file_change_while_editing_shows_the_other_items_as_the_server_has_them() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "old")]);
    h.model.state.add_editing_item("a");
    server_has(
        &mut h,
        vec![item("a", 1, ""), item("b", 1, "from the file")],
    );

    file_changed(&mut h);

    assert_eq!(h.bodies(), ["a=", "b=from the file"]);
}

// 打っていない編集中の項目は、サーバーの版の本文になり、カーソルは本文の長さに収まる。
#[test]
fn an_edited_item_not_typed_into_takes_the_server_body_and_the_cursor_stays_inside_it() {
    let mut h = Harness::with_items(vec![item("a", 1, "0123456789")]);
    h.model.state.add_editing_item("a");
    h.model.state.set_focused_item("a", Some(10));
    server_has(&mut h, vec![item("a", 1, "abc")]);

    file_changed(&mut h);

    assert_eq!(h.bodies(), ["a=abc"]);
    assert_eq!(h.focus_cursor(), Some(3));
    assert_eq!(h.focused_editor_requests(), [Some("a".to_string())]);
}

// 打っていない編集中の項目の本文が変わっても、カーソルが新しい本文の中にあれば、同じ位置に残る。
#[test]
fn the_cursor_inside_the_new_body_stays_where_it_was() {
    let mut h = Harness::with_items(vec![item("a", 1, "0123456789")]);
    h.model.state.add_editing_item("a");
    h.model.state.set_focused_item("a", Some(1));
    server_has(&mut h, vec![item("a", 1, "abc")]);

    file_changed(&mut h);

    assert_eq!(h.focus_cursor(), Some(1));
}

// 打っている編集中の項目は、読み直しでも手元の本文を残す。
#[test]
fn an_edited_item_typed_into_keeps_its_typed_body() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "old")]);
    typed(&mut h, "a", "typed here");
    server_has(
        &mut h,
        vec![item("a", 1, "from the file"), item("b", 1, "from the file")],
    );

    file_changed(&mut h);

    assert_eq!(h.bodies(), ["a=typed here", "b=from the file"]);
}

// 編集を閉じたが保存していない打ちかけの項目も、読み直しで手元の本文を残す。
#[test]
fn an_item_left_with_unsaved_typing_keeps_its_typed_body() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    typed(&mut h, "a", "typed here");
    h.model.state.remove_editing_item("a");
    server_has(&mut h, vec![item("a", 1, "from the file")]);

    file_changed(&mut h);

    assert_eq!(h.bodies(), ["a=typed here"]);
}

// 打っていない編集中の項目がサーバーの版で消えていたら、画面から消え、編集の印が外れ、前の項目にフォーカスが移る。
#[test]
fn an_edited_item_not_typed_into_that_the_server_lost_goes_and_the_focus_moves_up() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "b")]);
    h.model.state.add_editing_item("b");
    h.model.state.set_focused_item("b", Some(1));
    server_has(&mut h, vec![item("a", 1, "a")]);

    file_changed(&mut h);

    assert_eq!(h.bodies(), ["a=a"]);
    assert!(!h.model.state.is_editing("b"));
    assert_eq!(h.model.state.focused_item_id(), Some("a"));
}

// 打っている編集中の項目がサーバーの版で消えていたら、同じidで、手元の並びの前の項目の後に作り直され、編集が続く。
#[test]
fn an_edited_item_typed_into_that_the_server_lost_is_made_again_after_the_item_before_it() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 2, "")]);
    typed(&mut h, "b", "typed here");
    server_has(&mut h, vec![item("a", 1, "a")]);

    file_changed(&mut h);

    assert_eq!(
        create_calls(&h.api),
        [(
            "page1".to_string(),
            "after:a".to_string(),
            2,
            "typed here".to_string(),
            Some("b".to_string())
        )]
    );
    assert_eq!(h.outline(), ["a:1=a", "b:2=typed here"]);
    assert!(h.model.state.is_editing("b"));
}

// 手元の並びで前にあった項目もサーバーの版になければ、打っている項目はページの最初に作り直される。
#[test]
fn a_lost_typed_item_whose_item_before_went_too_is_made_again_first() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "")]);
    typed(&mut h, "b", "typed here");
    server_has(&mut h, vec![item("c", 1, "c")]);

    file_changed(&mut h);

    let positions: Vec<String> = create_calls(&h.api).into_iter().map(|c| c.1).collect();
    assert_eq!(positions, ["first"]);
    assert_eq!(h.bodies(), ["b=typed here", "c=c"]);
}

// 打っている項目も、字下げと並びはサーバーの版になる。残すのは本文だけである。
#[test]
fn a_typed_item_takes_its_level_and_place_from_the_server() {
    let mut h = Harness::with_items(vec![item("a", 1, ""), item("b", 1, "b")]);
    typed(&mut h, "a", "typed here");
    server_has(
        &mut h,
        vec![item("b", 1, "b"), item("a", 2, "from the file")],
    );

    file_changed(&mut h);

    assert_eq!(h.outline(), ["b:1=b", "a:2=typed here"]);
    assert!(create_calls(&h.api).is_empty());
}

// 編集していない項目の本文が読み直しで変わっても、エディタは開かない。フォーカスの印が残っていても、カーソルを奪わない。
#[test]
fn a_changed_body_of_an_item_not_edited_opens_no_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, "a")]);
    h.model.state.set_focused_item("a", Some(1));
    server_has(&mut h, vec![item("a", 1, "from the file")]);

    file_changed(&mut h);

    assert_eq!(h.bodies(), ["a=from the file"]);
    assert!(h.focused_editor_requests().is_empty());
}

// 打っていない編集中の項目が最初の項目で、サーバーの版で消えていたら、編集は終わり、どのエディタも開かない。
#[test]
fn a_lost_first_item_not_typed_into_ends_the_edit_and_opens_no_editor() {
    let mut h = Harness::with_items(vec![item("a", 1, "a"), item("b", 1, "b")]);
    h.model.state.add_editing_item("a");
    h.model.state.set_focused_item("a", Some(1));
    server_has(&mut h, vec![item("b", 1, "b")]);

    file_changed(&mut h);

    assert_eq!(h.bodies(), ["b=b"]);
    assert!(h.model.state.editing_item_ids().is_empty());
    assert!(h.focused_editor_requests().is_empty());
}

// サーバーの拡張が項目を書いたと知らされると、ファイルの変更と同じように、表示中のページが読み直される。
#[test]
fn items_written_by_an_extension_reload_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);

    h.send(Event::Sse(crate::effects::SseEvent::Message(
        r#"{"type":"pagesUpdated","pageId":"page1","source":"extension"}"#.into(),
    )));
    h.fire_timers();

    assert!(h.api.called("GetPageById"));
}

// MCPのツールがページを変えたと知らされると、表示中のページが読み直される。
#[test]
fn a_change_over_mcp_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);

    h.send(Event::Sse(crate::effects::SseEvent::Message(
        r#"{"type":"pagesUpdated","pageId":"page1","source":"mcp"}"#.into(),
    )));
    h.fire_timers();

    assert!(h.api.called("GetPageById"));
}

/// A page shown by a client that has started, and the ID it named itself with.
fn started_with_a_page() -> (Harness, ClientId) {
    let me = ClientId::new();
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::Start {
        client_id: me.clone(),
        opens_new_day: true,
    });
    (h, me)
}

/// The server tells that the client `by` changed the shown page through the API.
fn changed_by(h: &mut Harness, by: &ClientId) {
    h.send(Event::Sse(crate::effects::SseEvent::Message(format!(
        r#"{{"type":"pagesUpdated","pageId":"page1","source":"app","clientId":"{}"}}"#,
        by.as_str()
    ))));
    h.fire_timers();
}

// ほかのクライアントがAPIでページを変えたと知らされると、表示中のページが読み直される。
#[test]
fn a_change_by_another_client_reloads_the_page() {
    let (mut h, _) = started_with_a_page();

    changed_by(&mut h, &ClientId::new());

    assert!(h.api.called("GetPageById"));
}

// 新しいページの項目が入ると、そこにない項目の編集中の印は消える。離れたページの項目に印が残ると、他のクライアントの変更が取り込まれなくなる。
#[test]
fn the_items_of_a_page_drop_the_edit_marks_of_the_items_not_in_it() {
    let (mut h, _) = started_with_a_page();
    h.model.state.add_editing_item("item-of-the-page-left");
    h.model.state.set_items(vec![item("a", 1, "")]);

    changed_by(&mut h, &ClientId::new());

    assert!(h.api.called("GetPageById"));
}

// このクライアント自身の変更の知らせでは、ページは読み直されない。
#[test]
fn a_change_by_this_client_does_not_reload_the_page() {
    let (mut h, me) = started_with_a_page();

    changed_by(&mut h, &me);

    assert!(!h.api.called("GetPageById"));
}

/// The answer to a reading of the page that was asked for at `asked_at`: the page with "a" as
/// the server had it then.
fn page_as_it_was(asked_at: AskedAt) -> Event {
    Event::PageReplaced {
        page_id: "page1".into(),
        loaded: Box::new(PageWithItems {
            items: vec![item("a", 1, "as it was")],
            ..Default::default()
        }),
        asked_at,
    }
}

/// The answer to a page read for its indices, which gives "a" the index "a9"
fn indices_as_they_were(asked_at: AskedAt) -> Event {
    let mut a = item("a", 1, "");
    a.idx = "a9".into();
    Event::IndicesRead {
        page_id: "page1".into(),
        loaded: Box::new(PageWithItems {
            items: vec![a],
            ..Default::default()
        }),
        asked_at,
    }
}

// 並び順の値のための読み直しの答えは、頼んだあとにこちらで変更がなければ、項目の値になる。
#[test]
fn the_indices_read_again_are_taken_when_nothing_changed_here() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    let asked_at = AskedAt::now(&h.model);

    h.send(indices_as_they_were(asked_at));

    assert_eq!(h.model.state.get_item_by_id("a").unwrap().idx, "a9");
}

// 並び順の値のための読み直しの答えは、頼んだあとにこちらで変更があれば捨てる。その変更の答えが、改めて確かめられる。
#[test]
fn the_indices_read_again_are_dropped_when_a_change_here_overtook_them() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    let asked_at = AskedAt::now(&h.model);
    change_here(&mut h);

    h.send(indices_as_they_were(asked_at));

    assert_eq!(h.model.state.get_item_by_id("a").unwrap().idx, "a");
}

/// Changes the body of "a" here, as an undo or an edit does.
fn change_here(h: &mut Harness) {
    let before = item("a", 1, "");
    let after = item("a", 1, "changed here");
    h.model
        .state
        .update_item("a", |i| i.body = after.body.clone());
    h.model.undo.record_update("body", &before, &after, 0);
}

// 読み直しを頼んだあとにこちらで変更がなければ、答えのページが画面に出る。
#[test]
fn the_page_read_again_is_shown_when_nothing_changed_here_since_it_was_asked_for() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    let asked_at = AskedAt::now(&h.model);

    h.send(page_as_it_was(asked_at));

    assert_eq!(h.bodies(), ["a=as it was"]);
    assert!(!h.api.called("GetPageById"));
}

// 読み直しを頼んだあとにこちらで変更があれば、答えのページは画面に出ず、ページをもう一度読む。要求は順に送るので、答えを待つ間に取り消しができる。
#[test]
fn a_page_read_before_a_change_made_here_is_not_shown_and_the_page_is_read_once_more() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    let asked_at = AskedAt::now(&h.model);
    change_here(&mut h);
    h.api.answer_next(
        "GetPageById",
        Ok(crate::effects::ApiOutput::Page(PageWithItems {
            items: vec![item("a", 1, "with the change")],
            ..Default::default()
        })),
    );

    h.send(page_as_it_was(asked_at));

    assert_eq!(h.bodies(), ["a=with the change"]);
    assert_eq!(h.api.calls_named("GetPageById").len(), 1);
}

// 編集中でも、追い越された読み直しは、編集の終わりを待たずにもう一度読む。
#[test]
fn the_reading_that_a_change_overtook_is_done_again_at_once_while_editing() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    let asked_at = AskedAt::now(&h.model);
    change_here(&mut h);

    h.send(page_as_it_was(asked_at));

    assert_eq!(h.api.calls_named("GetPageById").len(), 1);
}
