//! Taking the outline again when another PC or site writes the atlas.

use super::*;

fn page_changed(h: &mut Harness, page_id: &str, source: &str) {
    h.send(Event::Sse(crate::effects::SseEvent::Message(format!(
        r#"{{"type":"pagesUpdated","pageId":"{page_id}","source":"{source}"}}"#
    ))));
}

// 表示中のアトラスは、サーバーがそのアウトラインを書き換えると、アウトラインを取得し直す。
#[test]
fn the_shown_atlas_takes_its_outline_again_when_the_server_writes_it() {
    // No item is shown yet, as on an atlas whose directory had no pages
    let mut h = atlas_of(vec![], vec![]);
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(crate::model::PageWithItems {
            page: doc("a/b/", ATLAS_NAME),
            items: vec![atlas_item(
                "l",
                1,
                "[[x/Target]]",
                &[("atlas_bounds", "1,2,0,40")],
            )],
            ..Default::default()
        })),
    );
    page_changed(&mut h, ATLAS_ID, "server");

    assert_eq!(h.view().atlas_links.len(), 1);
}

// このクライアント自身の変更や他のページの変更では、アウトラインは取得されない。
#[test]
fn the_outline_is_not_fetched_for_a_change_this_client_made_or_on_another_page() {
    let mut h = atlas_with(&[("one", None)]);
    h.model.client_id = "me".into();
    h.send(Event::Sse(crate::effects::SseEvent::Message(format!(
        r#"{{"type":"pagesUpdated","pageId":"{ATLAS_ID}","source":"app","clientId":"me"}}"#
    ))));
    page_changed(&mut h, "a/other_atlas", "server");

    assert!(!h.api.called("GetPageById"));
}
