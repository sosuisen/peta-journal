//! The journal screen, ported from `journal.ts`: the page of a day (or week, month, year)
//! followed by the older ones as the user scrolls, and moving between days.

use crate::app::{done, log_error, run, Cmd, Event, JournalNav, JournalState, Model, Screen};
use crate::effects::Api;
use crate::model::PageWithItems;
use crate::ops::edit::focus_editor;
use crate::ops::moves::CrossDayMove;
use crate::screens::{navigate, page, sidebar};
use crate::util::date::{format_journal_date, Date};
use crate::util::filename::{classify_journal_file_name, JournalType};
use crate::util::week::weeks_in_year;
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
    load_journal(model, date)
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
            Err(e) => return log_error(format!("Failed to load journal {date}: {e}")),
        };
        let title = if date == today {
            "Today".to_string()
        } else {
            format_journal_date(&date)
        };
        api.send_event(Event::JournalLoaded {
            date: date.clone(),
            title,
            loaded: Box::new(result),
        });
        match classify_journal_file_name(&date) {
            JournalType::Daily | JournalType::Weekly => {
                load_previous_journals(&api, date, BUFFER_ITEMS).await
            }
            _ => {}
        }
    })
}

/// The server answered the journal of a date: the screen shows it under the title. An
/// answer for a date the user has moved on from is not shown.
pub fn show_journal(model: &mut Model, date: String, title: String, loaded: PageWithItems) -> Cmd {
    let Some(j) = journal_mut(model) else {
        return done();
    };
    if j.current_date != date {
        return done();
    }
    j.page = Some(loaded.page.clone());
    j.top_visible_date = date.clone();
    j.loaded_dates.clear();
    j.loaded_dates.insert(date);
    let focus_item_id = j.focus_item_id.clone();
    model.state.set_items(loaded.items);
    if let Some(trees) = loaded.transcluded_trees {
        model.state.set_transcluded_trees(trees);
    }
    let mut cmds = vec![
        navigate::set_title(title),
        sidebar::set_current_page(model, Some(loaded.page)),
        page::scroll_to_item(model, focus_item_id),
    ];
    if loaded.created {
        cmds.push(sidebar::refresh_recent_pages());
    }
    Command::all(cmds)
}

/// Appends older journals, one page at a time, until the outline holds at least
/// `target_item_count` items or there is no older journal. Only one load runs at a time.
async fn load_previous_journals(api: &Api<Event>, from_name: String, target_item_count: usize) {
    let started = crate::ops::edit::read(api, |m| match journal_mut(m) {
        Some(j) if !j.loading => {
            j.loading = true;
            Some(j.journal_type == Some(JournalType::Weekly))
        }
        _ => None,
    })
    .await;
    let Some(weekly) = started else { return };
    let mut name = from_name;
    loop {
        let count = crate::ops::edit::read(api, |m| m.state.items().len()).await;
        if count >= target_item_count {
            break;
        }
        let result = match api.journal_before(&name, weekly).await {
            Ok(r) => r,
            Err(e) => {
                log_error(format!("Failed to load previous journals: {e}"));
                break;
            }
        };
        let Some(page) = result.page.filter(|_| result.found) else {
            break;
        };
        name = page.name.clone();
        let items = result.items;
        let trees = result.transcluded_trees;
        let n = name.clone();
        let appended = crate::ops::edit::read(api, move |m| {
            let Some(j) = journal_mut(m) else {
                return false;
            };
            if !j.loaded_dates.insert(n) {
                return false;
            }
            let mut all: Vec<_> = m.state.items().to_vec();
            all.extend(items.into_iter().map(std::sync::Arc::new));
            m.state.set_item_refs(all);
            if let Some(trees) = trees {
                m.state.merge_transcluded_trees(trees);
            }
            true
        })
        .await;
        let _ = appended;
    }
    api.send_event(Event::PreviousJournalsLoaded);
}

/// The load of the older journals ended: the next one may start.
pub fn previous_journals_loaded(model: &mut Model) -> Cmd {
    if let Some(j) = journal_mut(model) {
        j.loading = false;
    }
    done()
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
            return run(move |api| async move {
                load_previous_journals(&api, journal_name, target).await
            });
        }
    }
    done()
}

/// The journal name in a file name: `2026-W07` for weekly journals, `2026-02-10` otherwise.
fn extract_journal_name(name: &str, weekly: bool) -> Option<String> {
    if weekly {
        let idx = name.find("-W")?;
        let start = idx.checked_sub(4)?;
        let candidate = &name[start..(idx + 5).min(name.len())];
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
    let Some((y, w)) = current.split_once("-W") else {
        return done();
    };
    let (Ok(mut year), Ok(week)) = (y.parse::<i32>(), w.parse::<i32>()) else {
        return done();
    };
    let mut week = week + delta;
    let numbering = model.config.week_numbering;
    if week < 1 {
        year -= 1;
        week = weeks_in_year(year, numbering) as i32;
    } else if week > weeks_in_year(year, numbering) as i32 {
        year += 1;
        week = 1;
    }
    navigate::open_page(&format!("{year}-W{week:02}"))
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
            api.send_event(Event::MovedItemShown {
                item_id: item_id.clone(),
            });
            focus_editor(&api, Some(item_id)).await;
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
    done()
}

/// The header text: the date of the top visible item, or the journal's own date.
pub fn display_date(j: &JournalState) -> String {
    let d = if j.top_visible_date.is_empty() {
        &j.current_date
    } else {
        &j.top_visible_date
    };
    format_journal_date(d)
}
