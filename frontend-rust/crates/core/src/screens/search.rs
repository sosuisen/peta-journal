//! The search box, ported from `search-bar.ts`: a debounced combined search, keyboard
//! navigation of the results, and creating a page from the query.

use crate::app::{apply, done, log_error, run, Cmd, Model, SearchKey};
use crate::effects::{TimerOp, TimerOutput, UiOp};
use crate::model::SearchResult;
use crate::screens::navigate;
use crate::util::filename::{classify_journal_file_name, validate_file_name, JournalType};
use crux_core::Command;

const DEBOUNCE_MS: u32 = 200;

pub fn query_changed(model: &mut Model, value: String) -> Cmd {
    model.search.query = value.clone();
    let cancel = cancel_timer(model);
    if value.trim().is_empty() {
        model.search.page_results.clear();
        model.search.item_results.clear();
        model.search.show_create_button = false;
        return cancel;
    }
    let timer_id = model.next_timer_id();
    model.search.timer_id = timer_id;
    let search = run(move |api| async move {
        if api.sleep(timer_id, DEBOUNCE_MS).await != TimerOutput::Fired {
            return;
        }
        match api.search_combined(&value, 20).await {
            Ok(results) => apply(&api, "search results", move |m| {
                if m.search.timer_id != timer_id {
                    return done();
                }
                let (items, pages): (Vec<SearchResult>, Vec<SearchResult>) =
                    results.into_iter().partition(|r| !r.item_id.is_empty());
                let has_tag_terms = value
                    .split_whitespace()
                    .any(|t| t.starts_with('#') && t.len() > 1);
                m.search.page_results = pages;
                m.search.item_results = items;
                m.search.selected_index = None;
                m.search.show_create_button = !has_tag_terms && !value.trim().is_empty();
                done()
            }),
            Err(e) => log_error(format!("Search failed: {e}")),
        }
    });
    Command::all([cancel, search])
}

fn cancel_timer(model: &mut Model) -> Cmd {
    if model.search.timer_id == 0 {
        return done();
    }
    let id = std::mem::take(&mut model.search.timer_id);
    Command::notify_shell(TimerOp::Cancel { id }).into()
}

/// The titles offered by the "create" entries between the page and item results, in display
/// order. The second entry puts the page in the directory of the open doc page. Each entry is
/// left out when its page already exists. Journals and root pages have no directory, so they
/// offer only the bare title.
pub fn create_titles(model: &Model) -> Vec<String> {
    let title = model.search.query.trim();
    if !model.search.show_create_button || title.is_empty() {
        return Vec::new();
    }
    let mut titles = Vec::new();
    if !page_exists(model, title) {
        titles.push(title.to_string());
    }
    let dir = model
        .current_page
        .as_ref()
        .filter(|p| !p.is_journal() && !p.dir_path.is_empty())
        .map(|p| p.dir_path.trim_end_matches('/'));
    if let Some(dir) = dir {
        let path = format!("{dir}/{title}");
        if !page_exists(model, &path) {
            titles.push(path);
        }
    }
    titles
}

/// Whether a page with this full path is known to the model: the search hits carry the full
/// path as their title, and the sidebar lists the pages of the current directory.
fn page_exists(model: &Model, path: &str) -> bool {
    let path = path.to_lowercase();
    model
        .search
        .page_results
        .iter()
        .any(|r| r.page_title.to_lowercase() == path)
        || model
            .sidebar
            .index_pages
            .iter()
            .any(|p| format!("{}{}", p.dir_path, p.name).to_lowercase() == path)
}

pub fn key(model: &mut Model, key: SearchKey) -> Cmd {
    let total = model.search.page_results.len()
        + create_titles(model).len()
        + model.search.item_results.len();
    match key {
        SearchKey::Down => {
            if total > 0 {
                model.search.selected_index = Some(match model.search.selected_index {
                    Some(i) if i + 1 < total => i + 1,
                    _ => 0,
                });
            }
            done()
        }
        SearchKey::Up => {
            if total > 0 {
                model.search.selected_index = Some(match model.search.selected_index {
                    Some(i) if i > 0 => i - 1,
                    _ => total - 1,
                });
            }
            done()
        }
        SearchKey::Enter => match model.search.selected_index {
            Some(idx) => result_clicked(model, idx),
            None => done(),
        },
        SearchKey::Escape => {
            model.search.show_dropdown = false;
            model.search.selected_index = None;
            done()
        }
    }
}

/// An entry of the flat list was chosen: a page, the create entry, or an item.
pub fn result_clicked(model: &mut Model, index: usize) -> Cmd {
    let titles = create_titles(model);
    let page_count = model.search.page_results.len();
    if index < page_count {
        let result = model.search.page_results[index].clone();
        return navigate_to_result(model, &result);
    }
    if let Some(title) = titles.get(index - page_count) {
        return create_page(model, title.clone());
    }
    let item_idx = index - page_count - titles.len();
    match model.search.item_results.get(item_idx).cloned() {
        Some(result) => navigate_to_result(model, &result),
        None => done(),
    }
}

fn navigate_to_result(model: &mut Model, result: &SearchResult) -> Cmd {
    let clear = clear(model);
    let fragment = (!result.item_id.is_empty()).then_some(result.item_id.as_str());
    Command::all([
        clear,
        navigate::to_page(&result.page_title, false, false, false, fragment),
    ])
}

/// Empties the query, drops the results and closes the dropdown.
pub fn clear(model: &mut Model) -> Cmd {
    model.search.show_dropdown = false;
    model.search.query.clear();
    model.search.page_results.clear();
    model.search.item_results.clear();
    model.search.show_create_button = false;
    model.search.selected_index = None;
    cancel_timer(model)
}

/// Creates a new page with the given title (one of `create_titles`).
fn create_page(model: &mut Model, title: String) -> Cmd {
    let clear = clear(model);
    let go = run(move |api| async move {
        if classify_journal_file_name(&title) == JournalType::None {
            let stripped = title.strip_suffix(".md").unwrap_or(&title);
            let file_name = stripped.rsplit('/').next().unwrap_or(stripped);
            if let Some(error) = validate_file_name(file_name) {
                api.notify(UiOp::Alert(error));
                return;
            }
        }
        apply(&api, "search: create page", move |_| {
            navigate::open_page(&title)
        });
    });
    Command::all([clear, go])
}
