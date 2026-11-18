//! Reloading the shown page when its Markdown file changes on disk.

use crate::app::Event;
use crate::harness::{item, Harness};
use crate::model::PageWithItems;
use crate::ops::file_reload::AskedAt;
use crate::rest::ClientId;

// ファイルの変更通知は編集の終了を待ち、自動保存の猶予の後にページが再読み込みされる。
#[test]
fn a_file_change_waits_for_the_edit_to_end_and_then_reloads_the_page() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    h.send(Event::Sse(crate::effects::SseEvent::Message(
        r#"{"type":"pagesUpdated","pageId":"page1","source":"file"}"#.into(),
    )));
    assert!(!h.api.called("GetPageById"), "not while editing");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    assert!(
        !h.api.called("GetPageById"),
        "the auto-save gets a moment first"
    );
    h.fire_timers();
    assert!(h.api.called("GetPageById"));
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

// 編集中なら、追い越された読み直しのやり直しは、編集の終わりを待つ。
#[test]
fn the_reading_that_a_change_overtook_waits_for_the_end_of_the_edit() {
    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.model.state.add_editing_item("a");
    let asked_at = AskedAt::now(&h.model);
    change_here(&mut h);

    h.send(page_as_it_was(asked_at));

    assert_eq!(h.bodies(), ["a=changed here"]);
    assert!(!h.api.called("GetPageById"), "not while editing");
    h.send(Event::EditFinished {
        item_id: "a".into(),
    });
    h.fire_timers();
    assert!(h.api.called("GetPageById"));
}
