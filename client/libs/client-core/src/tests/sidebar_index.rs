//! The sidebar's list of the pages in the folder of the shown page, and the recent pages.

use crate::app::{Event, Route};
use crate::effects::{ApiOp, ApiOutput};
use crate::harness::Harness;
use crate::model::PageExt;
use crate::model::{DocPage, Page, PageWithItems};

fn page(dir_path: &str, name: &str, page_type: &str) -> Page {
    Page {
        id: format!("{dir_path}{name}"),
        dir_path: dir_path.into(),
        name: name.into(),
        page_type: page_type.into(),
        ..Page::default()
    }
}

fn doc(dir_path: &str, name: &str) -> Page {
    page(dir_path, name, "doc")
}

/// The server answers the next page lookup with this page.
fn served(h: &mut Harness, page: Page) {
    h.api.answer_next(
        "GetOrCreateDoc",
        Ok(ApiOutput::DocPage(DocPage {
            page,
            items: vec![],
            created: false,
        })),
    );
}

/// The server lists these pages as the children of the next folder asked for.
fn children(h: &mut Harness, pages: Vec<Page>) {
    h.api
        .answer_next("GetChildPages", Ok(ApiOutput::Pages(pages)));
}

fn open(h: &mut Harness, page: Page) {
    let name = page.canonical_path();
    served(h, page);
    h.send(Event::Route {
        route: Route::Page {
            name,
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
}

fn listed(h: &Harness) -> Vec<String> {
    h.view()
        .index_pages
        .pages
        .iter()
        .map(|p| p.label.clone())
        .collect()
}

fn asked_for(h: &Harness) -> Vec<String> {
    h.api
        .calls
        .iter()
        .filter_map(|c| match c {
            ApiOp::GetChildPages { parent_id } => Some(parent_id.clone()),
            _ => None,
        })
        .collect()
}

// フォルダの index ページを開くと、そのフォルダの子ページが並ぶ。サブフォルダはフォルダとして先に並び、地図のページは出ない。
#[test]
fn an_index_page_lists_the_pages_of_its_folder() {
    let mut h = Harness::new();
    let pages = vec![
        doc("notes/", "Idea"),
        doc("notes/sub/", "index"),
        doc("notes/", crate::screens::page::ATLAS_NAME),
    ];
    // The page asks once more whether it can be demoted
    children(&mut h, pages.clone());
    children(&mut h, pages);
    open(&mut h, doc("notes/", "index"));

    assert_eq!(asked_for(&h), ["notes/index", "notes/index"]);
    assert_eq!(h.view().index_pages.header, "notes/");
    assert_eq!(h.view().index_pages.parent_path.as_deref(), Some("index"));
    assert_eq!(listed(&h), ["sub/", "Idea"]);
    assert!(h.view().index_pages.pages[0].is_dir);
}

fn can_demote(h: &Harness) -> bool {
    match h.view().screen {
        crate::view::ScreenView::Page(Some(p)) => p.can_demote,
        other => panic!("not a page: {other:?}"),
    }
}

// 前に開いたindexページについての「降格できるか」の答えは、別のindexページへ移ったあとに届いても、今のページの降格のメニューを変えない。
#[test]
fn the_answer_whether_a_page_left_behind_can_be_demoted_does_not_change_the_shown_page() {
    let mut h = Harness::new();
    // Each index page asks for its children for the sidebar and for the demotion
    children(&mut h, vec![]);
    children(&mut h, vec![]);
    open(&mut h, doc("empty/", "index"));
    children(&mut h, vec![doc("full/", "Idea")]);
    children(&mut h, vec![doc("full/", "Idea")]);
    open(&mut h, doc("full/", "index"));
    assert!(!can_demote(&h));

    h.send(Event::CanDemoteLoaded {
        page_id: "empty/index".into(),
        can_demote: true,
    });

    assert!(!can_demote(&h));
}

// ルートの index ページでは、見出しは (root)/ で、doc ページだけが並ぶ。親フォルダはない。
#[test]
fn the_root_index_lists_only_the_doc_pages() {
    let mut h = Harness::new();
    children(
        &mut h,
        vec![doc("", "Idea"), page("", "2026-02-10", "journal")],
    );
    open(&mut h, doc("", "index"));

    assert_eq!(h.view().index_pages.header, "(root)/");
    assert_eq!(h.view().index_pages.parent_path, None);
    assert_eq!(listed(&h), ["Idea"]);
}

// フォルダの中のページを開くと、その親フォルダの index を引いて、兄弟のページが並ぶ。
#[test]
fn a_page_in_a_folder_lists_its_siblings() {
    let mut h = Harness::new();
    // The page's own items are loaded by id first, then the parent
    for _ in 0..2 {
        h.api.answer_next(
            "GetPageById",
            Ok(ApiOutput::Page(PageWithItems {
                page: doc("notes/", "index"),
                items: vec![],
                transcluded_trees: Default::default(),
            })),
        );
    }
    children(&mut h, vec![doc("notes/", "Idea"), doc("notes/", "Other")]);
    let mut page = doc("notes/", "Idea");
    page.parent_id = "notes/index".into();
    open(&mut h, page);

    assert_eq!(asked_for(&h), ["notes/index"]);
    assert_eq!(h.view().index_pages.header, "notes/");
    assert_eq!(listed(&h), ["Idea", "Other"]);
}

// 親のないページでは一覧は空である。ジャーナルを開くと、前の一覧は消える。
#[test]
fn a_page_without_a_parent_lists_nothing_and_a_journal_clears_the_list() {
    let mut h = Harness::new();
    open(&mut h, doc("", "Idea"));
    assert!(!h.api.called("GetChildPages"));
    assert!(listed(&h).is_empty());

    let mut h = Harness::new();
    h.model.sidebar.index_header = "notes/".into();
    h.model.sidebar.index_pages = vec![doc("notes/", "Idea")];
    h.send(Event::Route {
        route: Route::Page {
            name: "today".into(),
            fragment: None,
        },
        today: "2026-02-10".into(),
    });
    assert_eq!(h.view().index_pages.header, "");
    assert!(listed(&h).is_empty());
}

// 親のページを読めなければ、フォルダのページは問い合わせない。
#[test]
fn a_parent_that_cannot_be_read_asks_for_no_pages() {
    let mut h = Harness::new();
    h.api.fail_next("GetPageById", "offline");
    h.api.fail_next("GetPageById", "offline");
    let mut page = doc("notes/", "Idea");
    page.parent_id = "notes/index".into();
    open(&mut h, page);
    assert!(!h.api.called("GetChildPages"));
}

// 起動時に読まれる最近のページは、フォルダと名前の2行で並ぶ。ジャーナルは日付で、index ページはフォルダ名で、1行で並ぶ。
#[test]
fn recent_pages_show_their_folder_on_its_own_line() {
    let mut h = Harness::new();
    h.api.answer_next(
        "GetRecentPages",
        Ok(ApiOutput::Pages(vec![
            doc("sosuisen.net/article/", "Picturesque Tags"),
            doc("", "Idea"),
            page("", "2026-02-09", "journal"),
            doc("notes/", "index"),
            doc("", "index"),
        ])),
    );
    h.send(Event::Start {
        client_id: crate::rest::ClientId::new(),
        opens_new_day: true,
    });

    let labels: Vec<(Option<String>, String)> = h
        .view()
        .recent_pages
        .pages
        .iter()
        .map(|p| (p.dir_label.clone(), p.label.clone()))
        .collect();
    assert_eq!(
        labels,
        [
            (
                Some("sosuisen.net/article/".into()),
                "Picturesque Tags".into()
            ),
            (None, "Idea".into()),
            (None, "2026年2月9日 月曜日".into()),
            (None, "notes/".into()),
            (None, "(root)/".into()),
        ]
    );
}
