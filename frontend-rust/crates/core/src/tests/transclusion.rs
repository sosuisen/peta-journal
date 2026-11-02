//! Pasting a copied item reference as a transclusion.

use crate::app::Event;
use crate::harness::{item, Harness};

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
