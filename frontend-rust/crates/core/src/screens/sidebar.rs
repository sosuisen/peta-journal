//! The sidebar's data, ported from `recent-pages.service.ts` and `sidebar.ts`: the recent
//! pages and the index (sibling) pages of the current doc page.

use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::model::Page;

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
