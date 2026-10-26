//! The sidebar's data, ported from `recent-pages.service.ts` and `sidebar.ts`: the recent
//! pages and the index (sibling) pages of the current doc page.

use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::model::{IndexSort, IndexSortKey, Page, SortOrder};

/// Reloads the ten most recent doc pages.
pub fn refresh_recent_pages() -> Cmd {
    run(|api| async move {
        if let Ok(pages) = api.get_recent_pages(10, Some("doc")).await {
            apply(&api, "recent pages", move |m| {
                m.recent_pages = pages;
                done()
            });
        }
    })
}

/// Makes a page the current one and loads the sidebar's index pages for it.
pub fn set_current_page(model: &mut Model, page: Option<Page>) -> Cmd {
    model.current_page = page;
    load_index_pages(model)
}

/// Loads the pages listed under the current page's directory: an index page shows its
/// children, another page shows its siblings.
pub fn load_index_pages(model: &mut Model) -> Cmd {
    let page = match &model.current_page {
        Some(p) if p.page_type == "doc" => p.clone(),
        _ => {
            model.sidebar.index_header.clear();
            model.sidebar.index_pages.clear();
            return done();
        }
    };
    run(move |api| async move {
        let loaded = if page.name == "index" {
            let is_root = page.dir_path.is_empty();
            let header = if is_root {
                "(root)/".to_string()
            } else {
                page.dir_path.clone()
            };
            api.get_child_pages(&page.id)
                .await
                .map(|children| (header, is_root, children))
        } else if !page.parent_id.is_empty() {
            match api.get_page_by_id(&page.parent_id).await {
                Ok(parent) => {
                    let is_root = parent.page.dir_path.is_empty();
                    let header = if is_root {
                        "(root)/".to_string()
                    } else {
                        parent.page.dir_path.clone()
                    };
                    api.get_child_pages(&page.parent_id)
                        .await
                        .map(|siblings| (header, is_root, siblings))
                }
                Err(e) => Err(e),
            }
        } else {
            Ok((String::new(), false, Vec::new()))
        };
        match loaded {
            Ok((header, is_root, pages)) => apply(&api, "index pages", move |m| {
                m.sidebar.index_header = header;
                m.sidebar.index_pages = if is_root {
                    pages.into_iter().filter(|c| c.page_type == "doc").collect()
                } else {
                    pages
                };
                done()
            }),
            Err(e) => log_error(format!("Failed to load index pages: {e}")),
        }
    })
}

/// The sort of the directory the index list shows; by name until the user picks one.
pub fn index_sort(model: &Model) -> IndexSort {
    model
        .sidebar
        .index_sorts
        .get(&model.sidebar.index_header)
        .copied()
        .unwrap_or_default()
}

/// The index pages in the chosen order. Subdirectories come before files in either order.
/// Names compare as the list shows them, so a subdirectory sorts by its own name and not
/// by `index`. Pages with the same date keep the name order, so the list is stable while
/// the user edits.
pub fn sorted_index_pages(pages: &[Page], sort: IndexSort) -> Vec<Page> {
    let mut sorted: Vec<Page> = pages.to_vec();
    sorted.sort_by(|a, b| {
        let by_name = index_page_name(a).cmp(&index_page_name(b));
        let by_key = match sort.key {
            IndexSortKey::Name => by_name,
            IndexSortKey::Modified => a.modified_date.cmp(&b.modified_date).then(by_name),
            IndexSortKey::Published => published_or_modified_millis(a)
                .cmp(&published_or_modified_millis(b))
                .then(by_name),
        };
        let by_key = match sort.order {
            SortOrder::Asc => by_key,
            SortOrder::Desc => by_key.reverse(),
        };
        is_directory(b).cmp(&is_directory(a)).then(by_key)
    });
    sorted
}

/// True when the page is the index page of a subdirectory; the list shows it as a folder.
pub fn is_directory(page: &Page) -> bool {
    page.name == "index"
}

/// True when the page has a non-empty `published` property.
pub fn is_published(page: &Page) -> bool {
    page.properties
        .get("published")
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty())
}

/// The `published` property as epoch milliseconds, or the modified date when the page has
/// none or it does not parse.
fn published_or_modified_millis(page: &Page) -> i64 {
    page.properties
        .get("published")
        .and_then(|v| v.as_str())
        .and_then(crate::util::date::parse_rfc3339_millis)
        .unwrap_or(page.modified_date)
}

/// The display name of a page in the index list: an index page shows the last segment of
/// its directory with a slash.
pub fn index_page_name(page: &Page) -> String {
    if page.name == "index" {
        let trimmed = page.dir_path.trim_end_matches('/');
        let last = trimmed.rsplit('/').next().unwrap_or(trimmed);
        format!("{last}/")
    } else {
        page.name.clone()
    }
}

/// The display title of a page: journals as dates, index pages as their directory.
/// The label of a recent page as `(directory, name)`. A long path like
/// `sosuisen.net/article/Picturesque Tags` is split so the name stays readable: the
/// directory goes on its own line. Journals and index pages have no directory part.
pub fn recent_page_label(page: &Page) -> (Option<String>, String) {
    if page.page_type != "journal" && page.name != "index" && !page.dir_path.is_empty() {
        (Some(page.dir_path.clone()), page.name.clone())
    } else {
        (None, page_title(page))
    }
}

pub fn page_title(page: &Page) -> String {
    if page.page_type == "journal" {
        return crate::util::date::format_journal_date(&page.name);
    }
    if page.name == "index" {
        return if page.dir_path.is_empty() {
            "(root)/".to_string()
        } else {
            page.dir_path.clone()
        };
    }
    format!("{}{}", page.dir_path, page.name)
}
