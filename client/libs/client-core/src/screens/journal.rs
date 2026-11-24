//! The journal screen, ported from `journal.ts`: the page of a day (or week, month, year)
//! followed by the older ones as the user scrolls, and moving between days.

use crate::app::{done, log_error, run, Cmd, Event, JournalNav, JournalState, Model, Screen};
use crate::model::{Item, JournalPage, TranscludedTrees};
use crate::ops::edit::focus_editor;
use crate::ops::moves::CrossDayMove;
use crate::screens::{navigate, page, sidebar};
use crate::util::date::{format_journal_date, Date};
use crate::util::filename::{classify_journal_file_name, JournalType};
use crate::util::week::week_page_name_after;
use crate::view::LoadState;
use crux_core::Command;

/// How many items the outline keeps loaded ahead of the scroll position.
const BUFFER_ITEMS: usize = 100;

/// Shows the journal of a date (or `today`).
/// Shows the journal of a date; the fragment is the item to scroll to.
pub fn open(model: &mut Model, name: &str, fragment: Option<String>) -> Cmd {
    let date = if name.is_empty() || name == "today" {
        model.today.clone()
    } else {
        name.to_string()
    };
    model.screen = Screen::Journal(JournalState {
        page_name: name.to_string(),
        current_date: date.clone(),
        journal_type: Some(classify_journal_file_name(&date)),
        top_visible_date: date.clone(),
        page: None,
        loaded_dates: Default::default(),
        loading: false,
        focus_item_id: fragment.filter(|f| !f.is_empty()),
    });
    // The headings of the page left behind are wrong for the journal
    model.headings_state = LoadState::Loading;
    load_journal(model, date)
}

/// The server could not answer the journal of a date. The headings of the journal still on
/// screen fail; a reload of a journal already shown keeps them.
pub fn journal_failed(model: &mut Model, date: &str) -> Cmd {
    let shown = journal(model).is_some_and(|j| j.current_date == date);
    if shown && model.headings_state == LoadState::Loading {
        model.headings_state = LoadState::Failed;
    }
    done()
}

fn journal_mut(model: &mut Model) -> Option<&mut JournalState> {
    match &mut model.screen {
        Screen::Journal(j) => Some(j),
        _ => None,
    }
}

pub fn journal(model: &Model) -> Option<&JournalState> {
    match &model.screen {
        Screen::Journal(j) => Some(j),
        _ => None,
    }
}

/// Loads the journal page of the date and the previous journals that fill the buffer.
pub fn load_journal(model: &mut Model, date: String) -> Cmd {
    let today = model.today.clone();
    run(move |api| async move {
        let result = match api.get_or_create_journal(&date).await {
            Ok(r) => r,
            Err(e) => {
                log_error(format!("Failed to load journal {date}: {e}"));
                return api.send_event(Event::JournalFailed { date });
            }
        };
        let title = if date == today {
            "Today".to_string()
        } else {
            format_journal_date(&date)
        };
        api.send_event(Event::JournalLoaded {
            date,
            title,
            loaded: Box::new(result),
        });
    })
}

/// Shows the journal of a date under the title.
fn show_journal(model: &mut Model, date: String, title: String, loaded: JournalPage) -> Cmd {
    let Some(j) = journal_mut(model) else {
        return done();
    };
    j.page = Some(loaded.page.clone());
    j.top_visible_date = date.clone();
    j.loaded_dates.clear();
    j.loaded_dates.insert(date);
    let focus_item_id = j.focus_item_id.clone();
    model.state.set_items(loaded.items);
    model.state.set_transcluded_trees(loaded.transcluded_trees);
    model.headings_state = LoadState::Loaded;
    let mut cmds = vec![
        navigate::set_title(model, title),
        sidebar::set_current_page(model, Some(loaded.page)),
        page::scroll_to_item(model, focus_item_id),
    ];
    if loaded.created {
        cmds.push(sidebar::refresh_recent_pages());
    }
    Command::all(cmds)
}

/// The server answered the journal of a date: the screen shows it, and a daily or weekly
/// journal goes on to the older ones that fill the buffer. An answer for a date the user has
/// moved on from does neither.
pub fn journal_loaded(model: &mut Model, date: String, title: String, loaded: JournalPage) -> Cmd {
    // The journals before a day the user has moved on from would go under the day shown
    if journal(model).map(|j| j.current_date.as_str()) != Some(date.as_str()) {
        return done();
    }
    let older = matches!(
        classify_journal_file_name(&date),
        JournalType::Daily | JournalType::Weekly
    );
    let shown = show_journal(model, date.clone(), title, loaded);
    if !older {
        return shown;
    }
    Command::all([shown, load_previous_journals(model, date, BUFFER_ITEMS)])
}

/// Appends older journals, one page at a time, until the outline holds at least
/// `target_item_count` items or there is no older journal. Only one load runs at a time.
fn load_previous_journals(model: &mut Model, from_name: String, target_item_count: usize) -> Cmd {
    let weekly = match journal_mut(model) {
        Some(j) if !j.loading => {
            j.loading = true;
            j.journal_type == Some(JournalType::Weekly)
        }
        _ => return done(),
    };
    load_journal_before(model, from_name, weekly, target_item_count)
}

/// Asks the server for the journal before `name`, unless the outline holds enough items.
/// Each answer comes back as [`Event::OlderJournalLoaded`], which asks for the next one.
fn load_journal_before(
    model: &mut Model,
    name: String,
    weekly: bool,
    target_item_count: usize,
) -> Cmd {
    let Some(journal) = journal_of_screen(model) else {
        return done();
    };
    if model.state.items().len() >= target_item_count {
        return previous_journals_loaded(model, &journal);
    }
    run(move |api| async move {
        match api.journal_before(&name, weekly).await {
            Ok(older) => match older.page.filter(|_| older.found) {
                Some(page) => api.send_event(Event::OlderJournalLoaded {
                    journal,
                    weekly,
                    target_item_count,
                    name: page.name,
                    items: older.items,
                    transcluded_trees: Some(older.transcluded_trees),
                }),
                None => api.send_event(Event::PreviousJournalsLoaded { journal }),
            },
            Err(e) => {
                log_error(format!("Failed to load previous journals: {e}"));
                api.send_event(Event::PreviousJournalsLoaded { journal });
            }
        }
    })
}

/// The server answered an older journal: its items go to the end of the outline, unless
/// the journal is there already. Then the one before it is asked for. An answer loaded under
/// a journal the user has moved on from is dropped.
pub fn show_older_journal(
    model: &mut Model,
    journal: &str,
    weekly: bool,
    target_item_count: usize,
    older: OlderJournal,
) -> Cmd {
    if journal_of_screen(model).as_deref() != Some(journal) {
        return done();
    }
    let is_new = journal_mut(model).is_some_and(|j| j.loaded_dates.insert(older.name.clone()));
    if is_new {
        // The collapsed set is kept as it is for the items already shown, so the new items
        // add their own flags to it
        for item in older.items.iter().filter(|i| i.collapsed) {
            model.state.set_collapsed(&item.id, true);
        }
        let mut all: Vec<_> = model.state.items().to_vec();
        all.extend(older.items.into_iter().map(std::sync::Arc::new));
        model.state.set_item_refs(all);
        if let Some(trees) = older.transcluded_trees {
            model.state.merge_transcluded_trees(trees);
        }
    }
    load_journal_before(model, older.name, weekly, target_item_count)
}

/// An older journal the server answered: its name, its items and the trees they transclude.
pub struct OlderJournal {
    pub name: String,
    pub items: Vec<Item>,
    pub transcluded_trees: Option<TranscludedTrees>,
}

/// The load of the older journals under `journal` ended: the next one may start. The end of
/// a load under a journal the user has moved on from leaves the load of the shown one going.
pub fn previous_journals_loaded(model: &mut Model, journal: &str) -> Cmd {
    if let Some(j) = journal_mut(model).filter(|j| j.current_date == journal) {
        j.loading = false;
    }
    done()
}

/// The date of the journal the screen shows, which the older journals are loaded under.
fn journal_of_screen(model: &Model) -> Option<String> {
    journal(model).map(|j| j.current_date.clone())
}

/// The outline reports its scroll position: the header shows the date of the top item, and
/// more journals are loaded when the end comes near.
pub fn outline_scrolled(
    model: &mut Model,
    scrolled_from_top: bool,
    top_visible_index: usize,
    rendered_start: usize,
    rendered_end: usize,
) -> Cmd {
    model.scrolled_from_top = scrolled_from_top;
    if journal(model).is_none() {
        return done();
    }
    let items = model.state.visible_items();
    let total = items.len();
    if total == 0 {
        return done();
    }
    // Walk backward from the top item to the owning journal page item
    let top_name = (0..=top_visible_index.min(total - 1))
        .rev()
        .find(|&j| model.state.is_own_item(&items[j].id))
        .map(|j| items[j].name.clone());
    let last_name = items[total - 1].name.clone();
    let weekly = journal(model).and_then(|j| j.journal_type) == Some(JournalType::Weekly);
    let loading = journal(model).map(|j| j.loading).unwrap_or(true);
    if let (Some(name), Some(j)) = (top_name, journal_mut(model)) {
        j.top_visible_date = name;
    }
    // Keep a buffer of items ahead of the rendered range
    if total < rendered_end + BUFFER_ITEMS && !loading {
        let journal_name = extract_journal_name(&last_name, weekly);
        if let Some(journal_name) = journal_name {
            let target = rendered_start + BUFFER_ITEMS;
            return load_previous_journals(model, journal_name, target);
        }
    }
    done()
}

/// The journal name in a file name: `2026-W07` for weekly journals, `2026-02-10` otherwise.
fn extract_journal_name(name: &str, weekly: bool) -> Option<String> {
    if weekly {
        let idx = name.find("-W")?;
        let start = idx.checked_sub(4)?;
        // A name with other letters there is no journal: it is not cut inside a letter
        let candidate = name.get(start..idx + "-W07".len())?;
        return (classify_journal_file_name(candidate) == JournalType::Weekly)
            .then(|| candidate.to_string());
    }
    name.as_bytes()
        .windows(10)
        .map(|w| std::str::from_utf8(w).unwrap_or(""))
        .find(|w| Date::parse(w).is_some())
        .map(str::to_string)
}

pub fn nav(model: &mut Model, nav: JournalNav) -> Cmd {
    let Some(j) = journal(model) else {
        return done();
    };
    let current = j.current_date.clone();
    let delta: i32 = match nav {
        JournalNav::Prev => -1,
        JournalNav::Next => 1,
    };
    if let Some(day) = Date::parse(&current) {
        return navigate::open_page(&day.shift(delta.into()).iso());
    }
    // The first day of the week moves by seven days, so a week across New Year, which two
    // numbers could name, is passed once
    match week_page_name_after(&current, delta.into(), model.config.week_numbering) {
        Some(week) => navigate::open_page(&week),
        None => done(),
    }
}

/// After an item moved to another day: navigates to that day, or reloads the journal.
pub fn on_cross_day_moved(model: &mut Model, moved: CrossDayMove) -> Cmd {
    // A page the move made shows up in the recent pages, as a page made by opening it does
    let created = match &moved.created_page {
        Some(_) => sidebar::refresh_recent_pages(),
        None => done(),
    };
    Command::all([created, show_cross_day_move(model, moved)])
}

fn show_cross_day_move(model: &mut Model, moved: CrossDayMove) -> Cmd {
    if moved.navigate {
        let timer_id = model.next_timer_id();
        let item_id = moved.item_id;
        let go = navigate::open_page(&moved.target_date);
        let edit = run(move |api| async move {
            if api.sleep(timer_id, 300).await != crate::effects::TimerOutput::Fired {
                return;
            }
            api.send_event(Event::MovedItemShown { item_id });
        });
        return Command::all([go, edit]);
    }
    let Some(date) = journal(model).map(|j| j.current_date.clone()) else {
        return done();
    };
    load_journal(model, date)
}

/// The day an item moved to is shown: the item is edited there.
pub fn edit_moved_item(model: &mut Model, item_id: &str) -> Cmd {
    model.state.add_editing_item(item_id);
    model.state.set_focused_item(item_id, None);
    focus_editor(model, Some(item_id.to_string()))
}

/// The header text: the date of the top visible item, or the journal's own date.
pub fn display_date(j: &JournalState) -> String {
    format_journal_date(shown_date(j))
}

/// The title of a window that shows the journal: the header text, with "(Today)" behind the
/// date of today. A window has no other place that says which day is today.
pub fn window_title(j: &JournalState, today: &str) -> String {
    let date = display_date(j);
    if shown_date(j) == today {
        format!("{date} (Today)")
    } else {
        date
    }
}

/// The date of the top visible item, or the journal's own date.
fn shown_date(j: &JournalState) -> &str {
    if j.top_visible_date.is_empty() {
        &j.current_date
    } else {
        &j.top_visible_date
    }
}

#[cfg(test)]
mod tests {
    use super::extract_journal_name;

    // 週のジャーナルの名前は、ページの名前から切り出す。
    #[test]
    fn the_name_of_a_weekly_journal_is_cut_out_of_the_page_name() {
        assert_eq!(
            extract_journal_name("2026-W07", true),
            Some("2026-W07".to_string())
        );
    }

    // `-W`の前後に日本語のある名前は、週のジャーナルではない。文字の途中では切らない（ADR 043）。
    #[test]
    fn a_name_with_japanese_around_w_is_not_a_weekly_journal() {
        assert_eq!(extract_journal_name("あい-W07", true), None);
        assert_eq!(extract_journal_name("2026-W0日", true), None);
    }
}
