//! Pasting a copied item reference as a transclusion, and taking the link away again.

use crate::app::Event;
use crate::harness::{item, Harness};
use crate::testing::item_with;

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

// トランスクルージョンの解除で、項目は普通の項目に戻り、ページが読み直される。ない項目では何もしない。
#[test]
fn removing_a_transclusion_turns_the_item_back_into_a_plain_one_and_reloads_the_page() {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![transclusion]);
    h.send(Event::RemoveTransclusion {
        item_id: "t".into(),
    });
    assert!(h.api.called("RemoveTransclusion"));
    assert!(h.api.called("GetPageById"));

    let mut h = Harness::with_items(vec![item("a", 1, "")]);
    h.send(Event::RemoveTransclusion {
        item_id: "gone".into(),
    });
    assert!(!h.api.called("RemoveTransclusion"));
}

// 解除をサーバーが断ると、項目はそのままである。
#[test]
fn a_refused_removal_leaves_the_transclusion() {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let mut h = Harness::with_items(vec![transclusion]);
    h.api.fail_next("RemoveTransclusion", "offline");
    h.send(Event::RemoveTransclusion {
        item_id: "t".into(),
    });
    assert!(!h.api.called("GetPageById"));
    assert!(h
        .model
        .state
        .get_item_by_id("t")
        .unwrap()
        .transclude_item_id
        .is_some());
}

// 元の項目が消えたトランスクルージョンは、壊れたものとして示される。
#[test]
fn a_transclusion_whose_source_is_gone_is_shown_as_broken() {
    let transclusion = item_with("t", 1, |i| i.transclude_item_id = Some("src".into()));
    let h = Harness::with_items(vec![item("a", 1, ""), transclusion]);
    let broken: Vec<bool> = h
        .view()
        .outline
        .rows
        .iter()
        .map(|r| r.is_broken_transclusion)
        .collect();
    assert_eq!(broken, [false, true]);
}
