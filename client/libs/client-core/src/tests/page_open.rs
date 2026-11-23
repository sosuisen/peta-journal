//! Opening a doc page by its URL.

use crate::app::{Event, Route};
use crate::effects::{ApiOutput, UiOp};
use crate::harness::{item, Harness};
use crate::model::{BacklinkEntry, DocPage, Page, PageWithItems};
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
        Ok(ApiOutput::DocPage(DocPage {
            page,
            items: vec![],
            created,
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

// ページのウィンドウのタイトルは、フォルダ付きの名前である。次のページが届かないときは、前のタイトルのままである。
#[test]
fn the_window_title_of_a_page_is_its_name_with_its_folder() {
    let mut h = Harness::new();
    assert_eq!(h.view().title, "");
    served(&mut h, doc("notes/", "Idea"), false);
    visit(&mut h, "notes/Idea", None);
    assert_eq!(h.view().title, "notes/Idea");

    h.api.fail_next("GetOrCreateDoc", "the server is away");
    visit(&mut h, "notes/Other", None);
    assert_eq!(h.view().title, "notes/Idea");
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
    let heading = |id: &str, body: &str| {
        crate::testing::item_with(id, 1, |i| {
            i.body = body.into();
            i.types = vec!["heading".into()];
        })
    };
    let mut h = Harness::new();
    with_items(
        &mut h,
        doc("", "Idea"),
        vec![
            heading("h1", "# Top"),
            item("p", 1, "text"),
            heading("h2", "## Next"),
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

// 開いたことで作られたページは最近のページに出るので、最近のページが読み直される。既にあったページでは読み直さない。
#[test]
fn a_page_made_by_opening_it_refreshes_the_recent_pages() {
    for created in [true, false] {
        let mut h = Harness::new();
        served(&mut h, doc("notes/", "Idea"), created);

        visit(&mut h, "notes/Idea", None);

        assert_eq!(
            h.api.called("GetRecentPages"),
            created,
            "created: {created}"
        );
    }
}

// 別のページを開いたあとに届いた、前のページの答え（ページ、項目、バックリンク）は、表示されない。
#[test]
fn the_answers_for_a_page_the_screen_has_left_are_not_shown() {
    let mut h = Harness::new();
    served(&mut h, doc("", "Now"), false);
    with_items(&mut h, doc("", "Now"), vec![item("now1", 1, "now")]);
    visit(&mut h, "Now", None);

    h.send(Event::PageLoaded {
        page: doc("", "Old"),
        created: false,
    });
    h.send(Event::PageItemsLoaded {
        page_id: "Old".into(),
        loaded: Box::new(PageWithItems {
            page: doc("", "Old"),
            items: vec![item("old1", 1, "old")],
            created: false,
            transcluded_trees: None,
        }),
    });
    h.send(Event::BacklinksLoaded {
        page_id: "Old".into(),
        entries: vec![BacklinkEntry {
            source_page: doc("", "Other"),
            source_item: None,
        }],
    });

    assert_eq!(titles(&h), ["Now"]);
    assert_eq!(h.items_ids(), ["now1"]);
    assert!(shown_page(&h).unwrap().backlinks.is_empty());
    assert_eq!(
        h.model.current_page.as_ref().map(|p| p.id.as_str()),
        Some("Now")
    );
}

// フォルダの中のページは、根からそのフォルダまでの道を持つ。道の1つずつが、名前と、そこまでのパスと、そのフォルダの index ページのパスを持つ。
#[test]
fn a_page_in_a_folder_shows_the_way_to_its_folder() {
    let mut h = Harness::new();
    served(&mut h, doc("a/b/", "notes"), false);

    visit(&mut h, "a/b/notes", None);

    let ScreenView::Page(Some(page)) = h.view().screen else {
        panic!("the page is not shown");
    };
    let way: Vec<(&str, &str, &str)> = page
        .dir_segments
        .iter()
        .map(|segment| {
            (
                segment.name.as_str(),
                segment.path.as_str(),
                segment.index_path.as_str(),
            )
        })
        .collect();
    assert_eq!(way, [("a", "a", "a/index"), ("b", "a/b", "a/b/index")]);
}
