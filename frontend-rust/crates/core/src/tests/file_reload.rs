//! Reloading the shown page when its Markdown file changes on disk.

use crate::app::Event;
use crate::harness::{item, Harness};

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
