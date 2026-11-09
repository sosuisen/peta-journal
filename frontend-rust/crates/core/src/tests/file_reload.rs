//! Reloading the shown page when its Markdown file changes on disk.

use crate::app::Event;
use crate::harness::{item, Harness};
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

// このクライアント自身の変更の知らせでは、ページは読み直されない。
#[test]
fn a_change_by_this_client_does_not_reload_the_page() {
    let (mut h, me) = started_with_a_page();

    changed_by(&mut h, &me);

    assert!(!h.api.called("GetPageById"));
}
