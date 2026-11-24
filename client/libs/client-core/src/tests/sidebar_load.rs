//! Whether the lists of the sidebar are loading, loaded or failed (ADR 085).

use crate::app::{Event, Route};
use crate::effects::{ApiOutput, SseEvent};
use crate::harness::Harness;
use crate::model::{DocPage, Item, Page, PageExt, PageWithItems};
use crate::rest::ClientId;
use crate::view::LoadState;

fn doc(dir_path: &str, name: &str) -> Page {
    Page {
        id: format!("{dir_path}{name}"),
        dir_path: dir_path.into(),
        name: name.into(),
        page_type: "doc".into(),
        ..Page::default()
    }
}

fn start(h: &mut Harness) {
    h.send(Event::Start {
        client_id: ClientId::new(),
    });
}

/// The labels of the recent pages, and their state.
fn recent(h: &Harness) -> (Vec<String>, LoadState) {
    let view = h.view().recent_pages;
    let labels = view.pages.iter().map(|p| p.label.clone()).collect();
    (labels, view.state)
}

// 最近のページは、最初の答えが届くまで、空の一覧で読み込み中である。
#[test]
fn the_recent_pages_are_loading_and_empty_until_the_first_answer() {
    let mut h = Harness::new();
    h.hold("GetRecentPages");

    start(&mut h);

    assert_eq!(recent(&h), (vec![], LoadState::Loading));
}

// 最近のページは、答えが届くと、その一覧で読み込み済みになる。
#[test]
fn the_recent_pages_are_loaded_with_the_answer() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetRecentPages",
        Ok(ApiOutput::Pages(vec![doc("", "Idea")])),
    );

    start(&mut h);

    assert_eq!(recent(&h), (vec!["Idea".to_string()], LoadState::Loaded));
}

// 最近のページの最初の読み込みに失敗すると、空の一覧で失敗になる。
#[test]
fn the_recent_pages_fail_empty_when_the_first_load_fails() {
    let mut h = Harness::new();
    h.api.fail_next("GetRecentPages", "offline");

    start(&mut h);

    assert_eq!(recent(&h), (vec![], LoadState::Failed));
}

// 最近のページの読み直しの間と、その失敗のあとは、前の一覧で読み込み済みのままである。
#[test]
fn the_recent_pages_keep_their_list_while_and_after_a_reload_fails() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetRecentPages",
        Ok(ApiOutput::Pages(vec![doc("", "Idea")])),
    );
    start(&mut h);
    h.hold("GetRecentPages");

    h.send(Event::Sse(SseEvent::Open));
    let during = recent(&h);
    h.api.fail_next("GetRecentPages", "offline");
    h.release_held();

    let kept = (vec!["Idea".to_string()], LoadState::Loaded);
    assert_eq!(during, kept);
    assert_eq!(recent(&h), kept);
}

/// Opens a page by its path, which the server finds as this page.
fn open(h: &mut Harness, page: Page) {
    let name = page.canonical_path();
    h.api.answer_next(
        "GetOrCreateDoc",
        Ok(ApiOutput::DocPage(DocPage {
            page,
            items: vec![],
            created: false,
        })),
    );
    h.send(Event::Route {
        route: Route::Page {
            name,
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
}

/// The server lists these pages in the folders asked for next: once for the sidebar, and
/// once for the demotion of an index page.
fn children(h: &mut Harness, pages: Vec<Page>) {
    for _ in 0..2 {
        h.api
            .answer_next("GetChildPages", Ok(ApiOutput::Pages(pages.clone())));
    }
}

/// The labels of the pages of the folder, and their state.
fn folder(h: &Harness) -> (Vec<String>, LoadState) {
    let view = h.view().index_pages;
    let labels = view.pages.iter().map(|p| p.label.clone()).collect();
    (labels, view.state)
}

fn in_folder(dir_path: &str, name: &str) -> Page {
    let mut page = doc(dir_path, name);
    page.parent_id = format!("{dir_path}index");
    page
}

// フォルダのページは、文書のページを開いてから答えが届くまで、空の一覧で読み込み中である。
#[test]
fn the_folder_pages_are_loading_and_empty_until_the_answer() {
    let mut h = Harness::new();
    h.hold("GetChildPages");
    open(&mut h, doc("notes/", "index"));
    assert_eq!(folder(&h), (vec![], LoadState::Loading));

    children(&mut h, vec![doc("notes/", "Idea")]);
    h.release_held();

    assert_eq!(folder(&h), (vec!["Idea".to_string()], LoadState::Loaded));
}

// 別のフォルダのページに移ると、フォルダのページは、空の一覧で読み込み中になる。
#[test]
fn the_folder_pages_are_loading_and_empty_in_another_folder() {
    let mut h = Harness::new();
    children(&mut h, vec![doc("notes/", "Idea")]);
    open(&mut h, doc("notes/", "index"));
    h.hold("GetChildPages");

    open(&mut h, doc("other/", "index"));

    assert_eq!(folder(&h), (vec![], LoadState::Loading));
    assert_eq!(h.view().index_pages.header, "");
}

// 同じフォルダのページに移っても、フォルダのページは、前の一覧で読み込み済みのままである。
#[test]
fn the_folder_pages_stay_in_the_same_folder() {
    let mut h = Harness::new();
    children(&mut h, vec![doc("notes/", "Idea")]);
    open(&mut h, doc("notes/", "index"));
    h.hold("GetChildPages");

    open(&mut h, in_folder("notes/", "Idea"));

    assert_eq!(folder(&h), (vec!["Idea".to_string()], LoadState::Loaded));
}

// フォルダのページの読み込みに失敗すると、空の一覧で失敗になる。前のフォルダの一覧は残らない。
#[test]
fn the_folder_pages_fail_empty_when_their_load_fails() {
    let mut h = Harness::new();
    children(&mut h, vec![doc("old/", "Kept")]);
    open(&mut h, doc("old/", "index"));
    h.api.fail_next("GetChildPages", "offline");
    h.api.fail_next("GetChildPages", "offline");

    open(&mut h, doc("notes/", "index"));

    assert_eq!(folder(&h), (vec![], LoadState::Failed));
}

// 親のページを読めないときも、フォルダのページは空の一覧で失敗になる。
#[test]
fn the_folder_pages_fail_when_the_parent_cannot_be_read() {
    let mut h = Harness::new();
    h.api.fail_next("GetPageById", "offline");
    h.api.fail_next("GetPageById", "offline");

    open(&mut h, in_folder("notes/", "Idea"));

    assert_eq!(folder(&h), (vec![], LoadState::Failed));
}

// 同じフォルダの読み直しに失敗しても、前の一覧で読み込み済みのままである。
#[test]
fn the_folder_pages_stay_when_a_reload_of_the_same_folder_fails() {
    let mut h = Harness::new();
    children(&mut h, vec![doc("notes/", "Idea")]);
    open(&mut h, doc("notes/", "index"));
    h.api.fail_next("GetChildPages", "offline");

    open(&mut h, in_folder("notes/", "Idea"));

    assert_eq!(folder(&h), (vec!["Idea".to_string()], LoadState::Loaded));
}

// 文書でないページでは、フォルダのページは、空の一覧で読み込み済みである。親のない文書のページも同じである。
#[test]
fn the_folder_pages_are_loaded_and_empty_on_a_page_without_a_folder() {
    let mut h = Harness::new();
    open(&mut h, doc("", "Idea"));
    assert_eq!(folder(&h), (vec![], LoadState::Loaded));

    h.send(Event::Route {
        route: Route::Page {
            name: "today".into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });

    assert_eq!(folder(&h), (vec![], LoadState::Loaded));
}

/// A heading item with this text in Markdown.
fn heading(id: &str, body: &str) -> Item {
    crate::testing::item_with(id, 1, |i| {
        i.body = body.into();
        i.types = vec!["heading".into()];
    })
}

/// The server answers the next item load of a page with these items.
fn items_of(h: &mut Harness, page: Page, items: Vec<Item>) {
    h.api.answer_next(
        "GetPageById",
        Ok(ApiOutput::Page(PageWithItems {
            page,
            items,
            transcluded_trees: Default::default(),
        })),
    );
}

/// The texts of the headings, and their state.
fn headings(h: &Harness) -> (Vec<String>, LoadState) {
    let view = h.view().headings;
    let texts = view.headings.iter().map(|x| x.text.clone()).collect();
    (texts, view.state)
}

// 目次は、ページの項目が届くまで、空の一覧で読み込み中である。前のページの目次は出ない。
#[test]
fn the_headings_are_loading_and_empty_until_the_items_arrive() {
    let mut h = Harness::new();
    items_of(&mut h, doc("", "Old"), vec![heading("h1", "# Old")]);
    open(&mut h, doc("", "Old"));
    assert_eq!(headings(&h), (vec!["Old".to_string()], LoadState::Loaded));
    h.hold("GetPageById");

    open(&mut h, doc("", "New"));

    assert_eq!(headings(&h), (vec![], LoadState::Loading));
}

// 目次は、ページの項目が届くと、見出しの一覧で読み込み済みになる。
#[test]
fn the_headings_are_loaded_with_the_items() {
    let mut h = Harness::new();
    h.hold("GetPageById");
    open(&mut h, doc("", "New"));

    items_of(&mut h, doc("", "New"), vec![heading("h1", "# Top")]);
    h.release_held();

    assert_eq!(headings(&h), (vec!["Top".to_string()], LoadState::Loaded));
}

// ページの読み込みに失敗すると、目次は空の一覧で失敗になる。ページそのものを読めないときも同じである。
#[test]
fn the_headings_fail_empty_when_the_page_cannot_be_read() {
    let mut h = Harness::new();
    h.api.fail_next("GetPageById", "offline");
    open(&mut h, doc("", "New"));
    assert_eq!(headings(&h), (vec![], LoadState::Failed));

    let mut h = Harness::new();
    h.api.fail_next("GetOrCreateDoc", "offline");
    open(&mut h, doc("", "New"));
    assert_eq!(headings(&h), (vec![], LoadState::Failed));
}

// 日誌では、目次は、日誌が届くと空の一覧で読み込み済みになる。
#[test]
fn the_headings_of_a_journal_are_loaded_and_empty_when_it_arrives() {
    let mut h = Harness::new();
    items_of(&mut h, doc("", "Old"), vec![heading("h1", "# Old")]);
    open(&mut h, doc("", "Old"));
    h.hold("GetOrCreateJournal");

    h.send(Event::Route {
        route: Route::Page {
            name: "today".into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
    let during = headings(&h);
    h.release_held();

    assert_eq!(during, (vec![], LoadState::Loading));
    assert_eq!(headings(&h), (vec![], LoadState::Loaded));
}
