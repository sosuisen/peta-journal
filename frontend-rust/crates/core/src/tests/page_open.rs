//! Opening a doc page by its URL.

use crate::app::{Event, Route};
use crate::effects::{ApiOutput, UiOp};
use crate::harness::{item, Harness};
use crate::model::{BacklinkEntry, Page, PageWithItems};
use crate::view::ScreenView;

fn doc(dir_path: &str, name: &str) -> Page {
    Page {
        id: format!("{dir_path}{name}"),
        dir_path: dir_path.into(),
        name: name.into(),
        page_type: "doc".into(),
        ..Default::default()
    }
}

fn visit(h: &mut Harness, name: &str, fragment: Option<&str>) {
    h.send(Event::Route {
        route: Route::Page {
            name: name.into(),
            fragment: fragment.map(str::to_string),
        },
        today: "2026-02-10".into(),
    });
}

/// The server answers the next page lookup with this page.
fn served(h: &mut Harness, page: Page, created: bool) {
    h.api.answer_next(
        "GetOrCreateDoc",
        Ok(ApiOutput::Page(PageWithItems {
            page,
            items: vec![],
            created,
            transcluded_trees: None,
        })),
    );
}

/// The server answers the next item load of a page with these items.
fn with_items(h: &mut Harness, page: Page, items: Vec<crate::model::Item>) {
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(PageWithItems {
            page,
            items,
            created: false,
            transcluded_trees: None,
        })),
    );
}

fn titles(h: &Harness) -> Vec<String> {
    h.ui.iter()
        .filter_map(|op| match op {
            UiOp::SetTitle(t) => Some(t.clone()),
            _ => None,
        })
        .collect()
}

fn shown_page(h: &Harness) -> Option<Box<crate::view::PageView>> {
    match h.view().screen {
        ScreenView::Page(p) => p,
        other => panic!("not a page: {other:?}"),
    }
}

// ページを開くと、サーバーからページ・項目・バックリンクが読まれ、タイトルはフォルダ付きの名前になる。サイドバーの今のページもそのページになる。
#[test]
fn a_page_is_loaded_with_its_items_and_backlinks() {
    let mut h = Harness::new();
    served(&mut h, doc("notes/", "Idea"), false);
    with_items(&mut h, doc("notes/", "Idea"), vec![item("i1", 1, "hello")]);
    h.api.answer_next(
        "GetBacklinks",
        Ok(ApiOutput::Backlinks(vec![BacklinkEntry {
            source_page: doc("", "Other"),
            source_item: None,
        }])),
    );
    visit(&mut h, "notes/Idea", None);

    assert_eq!(titles(&h), ["notes/Idea"]);
    assert_eq!(h.items_ids(), ["i1"]);
    assert_eq!(shown_page(&h).unwrap().backlinks.len(), 1);
    assert_eq!(
        h.model.current_page.as_ref().map(|p| p.id.as_str()),
        Some("notes/Idea")
    );
}

// URL がページの正しいパスと違うときは、そのパスに置き換えて開き直す。たとえば `Quarkus` はフォルダ `Quarkus/` の index ページである。
#[test]
fn a_url_that_is_not_the_canonical_path_is_replaced_by_it() {
    let mut h = Harness::new();
    served(&mut h, doc("Quarkus/", "index"), false);
    visit(&mut h, "Quarkus", None);

    let last = h.navigations.last().unwrap();
    assert_eq!(
        (last.url.as_str(), last.replace),
        ("/pages/Quarkus/index", true)
    );
    assert!(titles(&h).is_empty());
    assert!(!h.api.called("GetPageById"));
}

// URL のフラグメントが項目を指すと、その項目が選ばれてスクロールされる。アウトラインにない項目ならスクロールしない。
#[test]
fn the_fragment_selects_and_scrolls_to_its_item() {
    let mut h = Harness::new();
    with_items(
        &mut h,
        doc("", "Idea"),
        vec![item("i1", 1, ""), item("i2", 1, "")],
    );
    visit(&mut h, "Idea", Some("i2"));
    assert!(h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::ScrollToItem { item_id, smooth: false } if item_id == "i2")));
    assert!(h.model.state.item_selection_mode());
    assert!(h.model.state.selected_item_ids().contains("i2"));

    let mut h = Harness::new();
    visit(&mut h, "Idea", Some("gone"));
    assert!(!h
        .ui
        .iter()
        .any(|op| matches!(op, UiOp::ScrollToItem { .. })));
}

// ページを読めなければ画面は空のままである。項目やバックリンクを読めなければ、その部分が空のままである。
#[test]
fn what_cannot_be_loaded_stays_empty() {
    let mut h = Harness::new();
    h.api.fail_next("GetOrCreateDoc", "offline");
    visit(&mut h, "Idea", None);
    assert!(shown_page(&h).is_none());
    assert!(titles(&h).is_empty());

    let mut h = Harness::new();
    h.api.fail_next("GetPageById", "offline");
    h.api.fail_next("GetBacklinks", "offline");
    visit(&mut h, "Idea", None);
    assert!(h.items_ids().is_empty());
    assert!(shown_page(&h).unwrap().backlinks.is_empty());
}

// ブラウザのタイトルは、フォルダの index ページならそのフォルダ、ルートの index なら (root)/ になる。
#[test]
fn an_index_page_is_titled_by_its_folder() {
    let mut h = Harness::new();
    served(&mut h, doc("notes/", "index"), false);
    visit(&mut h, "notes/index", None);
    served(&mut h, doc("", "index"), false);
    visit(&mut h, "index", None);

    assert_eq!(titles(&h), ["notes/", "(root)/"]);
}

// doc ページの見出しの項目は、サイドバーに段の深さとともに並ぶ。
#[test]
fn the_headings_of_a_doc_page_are_listed_in_the_sidebar() {
    let heading = |id: &str, body: &str, level: u8| {
        crate::testing::item_with(id, 1, |i| {
            i.body = body.into();
            i.types = vec!["heading".into()];
            i.ast = format!(
                r#"{{"type":"document","children":[{{"type":"heading","level":{level},"children":[]}}]}}"#
            );
        })
    };
    let mut h = Harness::new();
    with_items(
        &mut h,
        doc("", "Idea"),
        vec![
            heading("h1", "# Top", 1),
            item("p", 1, "text"),
            heading("h2", "## Next", 2),
        ],
    );
    visit(&mut h, "Idea", None);

    let headings: Vec<(String, u8)> = h
        .view()
        .sidebar
        .headings
        .iter()
        .map(|x| (x.text.clone(), x.level))
        .collect();
    assert_eq!(headings, [("Top".into(), 1), ("Next".into(), 2)]);
}
