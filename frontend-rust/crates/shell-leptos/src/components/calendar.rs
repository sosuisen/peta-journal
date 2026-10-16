//! The month calendar of the sidebar and the journal header, ported from `calendar.ts`.

use crate::core::CoreHandle;
use crate::dom;
use leptos::prelude::*;
use peta_core::model::WeekNumbering;
use peta_core::util::date::{days_in_month, Date};
use peta_core::util::week::{iso_week_number, iso_week_year, us_week_number};
use peta_core::Event;

const US_DAY_LABELS: [&str; 7] = ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
const ISO_DAY_LABELS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

#[derive(Clone, PartialEq)]
struct Day {
    date: Date,
    current_month: bool,
}

/// The weeks shown for a month: full weeks, with the days of the neighbouring months greyed.
fn weeks_of(year: i32, month: u32, iso: bool) -> Vec<(u32, Vec<Day>)> {
    let first = Date::new(year, month, 1);
    let raw = first.weekday();
    let first_day = if iso { (raw + 6) % 7 } else { raw } as i64;
    let mut days: Vec<Day> = Vec::new();
    for i in (1..=first_day).rev() {
        days.push(Day {
            date: first.shift(-i),
            current_month: false,
        });
    }
    for d in 1..=days_in_month(year, month) {
        days.push(Day {
            date: Date::new(year, month, d),
            current_month: true,
        });
    }
    let remainder = days.len() % 7;
    if remainder > 0 {
        let last = days.last().unwrap().date;
        for i in 1..=(7 - remainder) as i64 {
            days.push(Day {
                date: last.shift(i),
                current_month: false,
            });
        }
    }
    days.chunks(7)
        .map(|week| {
            let first_of_week = week[0].date;
            let number = if iso {
                iso_week_number(first_of_week)
            } else {
                us_week_number(first_of_week)
            };
            (number, week.to_vec())
        })
        .collect()
}

/// Sends the shown page a page navigation; Ctrl opens a tab, Shift a window.
fn go(core: &CoreHandle, path: String, ev: &web_sys::MouseEvent) {
    core.send(Event::GoToPage {
        path,
        new_tab: ev.ctrl_key() || ev.meta_key(),
        new_window: ev.shift_key(),
    });
}

#[component]
pub fn Calendar(
    #[prop(into)] selected_date: Signal<String>,
    /// Called with the modifier state after a navigation, e.g. to close a popup calendar.
    #[prop(optional)]
    on_navigate: Option<Callback<bool>>,
) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let numbering = Memo::new(move |_| vm.with(|v| v.config.week_numbering));
    let today = dom::today_iso();
    let now = js_sys::Date::new_0();
    let view_year = RwSignal::new(now.get_full_year() as i32);
    let view_month = RwSignal::new(now.get_month() + 1);

    let weeks = Memo::new(move |_| {
        weeks_of(
            view_year.get(),
            view_month.get(),
            numbering.get() == WeekNumbering::Iso,
        )
    });
    let day_labels = move || {
        if numbering.get() == WeekNumbering::Iso {
            ISO_DAY_LABELS
        } else {
            US_DAY_LABELS
        }
    };
    let notify = move |ev: &web_sys::MouseEvent| {
        if let Some(cb) = on_navigate {
            cb.run(ev.ctrl_key() || ev.meta_key() || ev.shift_key());
        }
    };

    let prev_month = move |_| {
        if view_month.get() == 1 {
            view_year.update(|y| *y -= 1);
            view_month.set(12);
        } else {
            view_month.update(|m| *m -= 1);
        }
    };
    let next_month = move |_| {
        if view_month.get() == 12 {
            view_year.update(|y| *y += 1);
            view_month.set(1);
        } else {
            view_month.update(|m| *m += 1);
        }
    };

    view! {
        <app-calendar>
            <div class="calendar">
                <div class="calendar-header">
                    <button on:click=prev_month>"<"</button>
                    <span>
                        <span class="month-label" on:click=move |ev| { go(&core, format!("{}-{:02}", view_year.get(), view_month.get()), &ev); notify(&ev); }>
                            {move || MONTH_NAMES[(view_month.get() - 1) as usize]}
                        </span>
                        " "
                        <span class="year-label" on:click=move |ev| { go(&core, view_year.get().to_string(), &ev); notify(&ev); }>
                            {move || view_year.get()}
                        </span>
                    </span>
                    <button on:click=next_month>">"</button>
                </div>
                <div class="calendar-grid">
                    <div class="week-label"></div>
                    {move || day_labels().iter().map(|l| view! { <div class="day-label">{*l}</div> }).collect::<Vec<_>>()}
                    <For
                        each=move || weeks.get()
                        key=|(n, days)| (*n, days[0].date)
                        children={
                                                        let today = today.clone();
                            move |(number, days)| {
                                let first = days[0].date;
                                let core_week = core;
                                let week_click = move |ev: web_sys::MouseEvent| {
                                    let iso = numbering.get() == WeekNumbering::Iso;
                                    let year = if iso { iso_week_year(first) } else { first.year };
                                    go(&core_week, format!("{year}-W{number:02}"), &ev);
                                    notify(&ev);
                                };
                                let cells = days.iter().map(|day| {
                                    let date = day.date;
                                    let iso = date.iso();
                                    let is_today = iso == today;
                                    let iso2 = iso.clone();
                                    let is_selected = move || selected_date.get() == iso2;
                                    let core_day = core;
                                    view! {
                                        <div
                                            class="day-cell"
                                            class:other-month=!day.current_month
                                            class:today=is_today
                                            class:selected=is_selected
                                            on:click=move |ev| { go(&core_day, iso.clone(), &ev); notify(&ev); }
                                        >
                                            {date.day}
                                        </div>
                                    }
                                }).collect::<Vec<_>>();
                                view! {
                                    <div class="week-number" on:click=week_click>{format!("{number:02}")}</div>
                                    {cells}
                                }
                            }
                        }
                    />
                </div>
            </div>
        </app-calendar>
    }
}
