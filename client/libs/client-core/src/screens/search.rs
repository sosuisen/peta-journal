//! The search box, ported from `search-bar.ts`: a debounced combined search, keyboard
//! navigation of the results, and creating a page from the query.

use crate::app::{cancel_timer, done, log_error, run, Cmd, Event, Model, SearchKey};
use crate::effects::TimerOutput;
use crate::model::PageExt;
use crate::model::SearchResult;
use crate::screens::navigate;
use crate::util::filename::{classify_journal_file_name, validate_file_name, JournalType};
use crux_core::Command;

/// How long the search waits for more typing before it asks the server.
pub const SEARCH_DELAY_MS: u32 = 200;

pub fn query_changed(model: &mut Model, value: String) -> Cmd {
    model.search.query = value.clone();
    // Enter and Escape close the list but leave the focus in the box, so no focus event reopens it
    model.search.show_dropdown = true;
    // The number of a selected result means another result once the create entries follow the
    // new query, and the results themselves still belong to the old one
    model.search.selected_index = None;
    model.search.held_keys.clear();
    model.search.searching = !value.trim().is_empty();
    let cancel = cancel_timer(&mut model.search.timer_id);
    if value.trim().is_empty() {
        model.search.page_results.clear();
        model.search.item_results.clear();
        model.search.show_create_button = false;
        return cancel;
    }
    let timer_id = model.next_timer_id();
    model.search.timer_id = timer_id;
    let search = run(move |api| async move {
        if api.sleep(timer_id, SEARCH_DELAY_MS).await != TimerOutput::Fired {
            return;
        }
        match api.search_combined(&value, 20).await {
            Ok(results) => api.send_event(Event::SearchAnswered {
                timer_id,
                query: value,
                results,
            }),
            Err(e) => {
                log_error(format!("Search failed: {e}"));
                api.send_event(Event::SearchFailed { timer_id });
            }
        }
    });
    Command::all([cancel, search])
}

/// Shows the results of a search, unless a later query has started since.
pub fn show_results(
    model: &mut Model,
    timer_id: u64,
    query: &str,
    results: Vec<SearchResult>,
) -> Cmd {
    if model.search.timer_id != timer_id {
        return done();
    }
    let (items, pages): (Vec<SearchResult>, Vec<SearchResult>) =
        results.into_iter().partition(|r| !r.item_id.is_empty());
    let has_tag_terms = query
        .split_whitespace()
        .any(|t| t.starts_with('#') && t.len() > 1);
    model.search.page_results = pages;
    model.search.item_results = items;
    model.search.searching = false;
    model.search.show_create_button = !has_tag_terms && !query.trim().is_empty();
    // A held key starts from the selection it was pressed on: none, or today's journal, which
    // is the first entry of the new list too
    let held = std::mem::take(&mut model.search.held_keys);
    if held.is_empty() {
        model.search.selected_index = None;
    }
    Command::all(held.into_iter().map(|k| key(model, k)).collect::<Vec<_>>())
}

/// Stops showing that a search is under way, unless a later query has started since. The
/// keys that waited for the answer are dropped.
pub fn search_failed(model: &mut Model, timer_id: u64) -> Cmd {
    if model.search.timer_id == timer_id {
        model.search.searching = false;
        model.search.held_keys.clear();
    }
    done()
}

/// Whether the list starts with the entry that opens today's journal: the query is `today` in
/// any case. It does not wait for the answer of the search.
pub fn offers_today(model: &Model) -> bool {
    model.search.query.trim().eq_ignore_ascii_case("today")
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
    if key == SearchKey::Escape {
        model.search.held_keys.clear();
    } else if waits_for_answer(model, key) {
        model.search.held_keys.push(key);
        return done();
    }
    let total = usize::from(offers_today(model))
        + model.search.page_results.len()
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

/// Whether a key has to wait for the answer to the query: the results shown still belong to
/// an older query, and the key would select or open one of them. Today's journal is known
/// without the answer. A key after a held one waits too, to keep the order.
fn waits_for_answer(model: &Model, key: SearchKey) -> bool {
    if !model.search.searching {
        return false;
    }
    if !model.search.held_keys.is_empty() {
        return true;
    }
    let on_today = offers_today(model)
        && match key {
            SearchKey::Down => model.search.selected_index.is_none(),
            SearchKey::Enter => model.search.selected_index == Some(0),
            _ => false,
        };
    !on_today
}

/// An entry of the flat list was chosen: today's journal, a page, the create entry, or an item.
pub fn result_clicked(model: &mut Model, index: usize) -> Cmd {
    let index = if offers_today(model) {
        match index.checked_sub(1) {
            Some(below) => below,
            None => return Command::all([clear(model), navigate::to_url("/today", false)]),
        }
    } else {
        index
    };
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
        navigate::to_page(&result.page_title, false, fragment),
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
    model.search.searching = false;
    model.search.held_keys.clear();
    cancel_timer(&mut model.search.timer_id)
}

/// Creates a new page with the given title (one of `create_titles`).
fn create_page(model: &mut Model, title: String) -> Cmd {
    let clear = clear(model);
    let go = run(move |api| async move {
        if classify_journal_file_name(&title) == JournalType::None {
            let stripped = title.strip_suffix(".md").unwrap_or(&title);
            let file_name = stripped.rsplit('/').next().unwrap_or(stripped);
            if let Some(error) = validate_file_name(file_name) {
                api.alert(error);
                return;
            }
        }
        api.send_event(Event::OpenPage { path: title });
    });
    Command::all([clear, go])
}
