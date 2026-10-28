//! The order of the sidebar's index list.

use crate::app::Event;
use crate::harness::Harness;
use crate::model::{IndexSort, IndexSortKey, Page, SortOrder};
use crate::screens::sidebar::sorted_index_pages;

fn page(name: &str, modified: i64, published: Option<&str>) -> Page {
    let (dir_path, name) = match name.strip_suffix('/') {
        Some(dir) => (format!("notes/{dir}/"), "index".to_string()),
        None => ("notes/".to_string(), name.to_string()),
    };
    let mut page = Page {
        id: name.clone(),
        dir_path,
        name,
        page_type: "doc".into(),
        modified_date: modified,
        ..Page::default()
    };
    if let Some(p) = published {
        page.properties.insert("published".into(), p.into());
    }
    page
}

fn names(pages: &[Page]) -> Vec<String> {
    pages
        .iter()
        .map(crate::screens::sidebar::index_page_name)
        .collect()
}

fn sort(key: IndexSortKey, order: SortOrder) -> IndexSort {
    IndexSort { key, order }
}

#[test]
fn by_name_a_subdirectory_sorts_by_its_own_name_not_by_index() {
    let pages = vec![
        page("gamma/", 1, None),
        page("beta/", 2, None),
        page("alpha/", 3, None),
    ];
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Name, SortOrder::Asc));
    assert_eq!(names(&sorted), vec!["alpha/", "beta/", "gamma/"]);
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Name, SortOrder::Desc));
    assert_eq!(names(&sorted), vec!["gamma/", "beta/", "alpha/"]);
}

#[test]
fn subdirectories_come_before_files_in_either_order() {
    let pages = vec![
        page("zeta", 1, None),
        page("beta/", 2, None),
        page("alpha", 3, None),
    ];
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Name, SortOrder::Asc));
    assert_eq!(names(&sorted), vec!["beta/", "alpha", "zeta"]);
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Name, SortOrder::Desc));
    assert_eq!(names(&sorted), vec!["beta/", "zeta", "alpha"]);
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Modified, SortOrder::Desc));
    assert_eq!(names(&sorted), vec!["beta/", "alpha", "zeta"]);
}

#[test]
fn by_modified_the_newest_comes_first_when_descending() {
    let pages = vec![
        page("a", 10, None),
        page("b", 30, None),
        page("c", 20, None),
    ];
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Modified, SortOrder::Desc));
    assert_eq!(names(&sorted), vec!["b", "c", "a"]);
}

#[test]
fn by_published_a_page_without_the_property_uses_its_modified_date() {
    let pages = vec![
        page("old-post", 500, Some("2026-01-01T00:00:00Z")),
        page("draft", 1_600_000_000_000, None),
        page("new-post", 1, Some("2026-03-01T00:00:00Z")),
        page("bad-date", 1_000_000_000_000, Some("someday")),
    ];
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Published, SortOrder::Desc));
    assert_eq!(
        names(&sorted),
        vec!["new-post", "old-post", "draft", "bad-date"]
    );
}

#[test]
fn pages_with_the_same_date_keep_the_name_order() {
    let pages = vec![page("b", 7, None), page("a", 7, None), page("c", 7, None)];
    let sorted = sorted_index_pages(&pages, sort(IndexSortKey::Modified, SortOrder::Desc));
    assert_eq!(names(&sorted), vec!["c", "b", "a"]);
}

#[test]
fn the_parent_entry_opens_the_index_page_of_the_parent_directory() {
    use crate::screens::sidebar::parent_index_path;
    assert_eq!(
        parent_index_path("notes/sub/"),
        Some("notes/index".to_string())
    );
    assert_eq!(parent_index_path("notes/"), Some("index".to_string()));
    assert_eq!(parent_index_path("(root)/"), None);
    assert_eq!(parent_index_path(""), None);
}

#[test]
fn the_view_sorts_the_index_list_by_the_sort_of_its_directory() {
    let mut h = Harness::with_items(vec![]);
    h.model.sidebar.index_header = "notes/".into();
    h.model.sidebar.index_pages = vec![page("a", 10, None), page("b", 30, None)];
    assert_eq!(h.view().sidebar.index_sort, IndexSort::default());

    h.send(Event::SetIndexSort {
        dir: "notes/".into(),
        sort: sort(IndexSortKey::Modified, SortOrder::Desc),
    });
    let sidebar = h.view().sidebar;
    assert_eq!(
        sidebar.index_sort,
        sort(IndexSortKey::Modified, SortOrder::Desc)
    );
    assert_eq!(
        sidebar
            .index_pages
            .iter()
            .map(|p| p.label.clone())
            .collect::<Vec<_>>(),
        vec!["b", "a"]
    );

    // The icon of a page with a published property is filled
    h.model.sidebar.index_pages = vec![
        page("a", 10, None),
        page("b", 30, Some("2026-01-01T00:00:00Z")),
    ];
    let flags: Vec<(String, bool)> = h
        .view()
        .sidebar
        .index_pages
        .iter()
        .map(|p| (p.label.clone(), p.published))
        .collect();
    assert_eq!(
        flags,
        vec![("b".to_string(), true), ("a".to_string(), false)]
    );

    // Another directory keeps the default
    h.model.sidebar.index_header = "other/".into();
    assert_eq!(h.view().sidebar.index_sort, IndexSort::default());
}
