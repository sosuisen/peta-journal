//! The month calendar of the sidebar and the journal header. The core decides what it shows
//! (`CalendarView`, ADR 052); this only draws it and sends the clicks.

use crate::core::CoreHandle;
use crate::dom;
use client_core::keys::OpenIn;
use client_core::screens::calendar::{CalendarDay, CalendarNav};
use client_core::Event;
use leptos::prelude::*;

/// Which day is drawn as the selected one.
#[derive(Clone, Copy, PartialEq)]
pub enum Selects {
    /// Today, as in the sidebar
    Today,
    /// The day of the journal on screen, as in the journal header
    JournalDay,
}

#[component]
pub fn Calendar(
    selects: Selects,
    /// Called with the modifier state after a navigation, e.g. to close a popup calendar.
    #[prop(optional)]
    on_navigate: Option<Callback<bool>>,
) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let calendar = core.select(|v| v.calendar.clone());
    // Opens a journal by its name; Ctrl opens a tab, Shift a window.
    let open = move |page: String| {
        move |ev: web_sys::MouseEvent| {
            let open_in = dom::held(&ev).open_in();
            core.send(Event::GoToPage {
                path: page.clone(),
                open_in,
            });
            if let Some(cb) = on_navigate {
                cb.run(open_in != OpenIn::Here);
            }
        }
    };
    let nav = move |nav: CalendarNav| move |_| core.send(Event::CalendarNav(nav));
    let day_cell = move |day: CalendarDay| {
        let selected = match selects {
            Selects::Today => day.today,
            Selects::JournalDay => day.selected,
        };
        view! {
            <div
                class="day-cell"
                class:other-month=day.other_month
                class:today=day.today
                class:selected=selected
                on:click=open(day.date)
            >
                {day.day}
            </div>
        }
    };

    view! {
        <app-calendar>
            <div class="calendar">
                <div class="calendar-header">
                    <button on:click=nav(CalendarNav::PrevMonth)>"<"</button>
                    {move || calendar.with(|c| view! {
                        <span>
                            <span class="month-label" on:click=open(c.month_page.clone())>{c.month_name.clone()}</span>
                            " "
                            <span class="year-label" on:click=open(c.year_page.clone())>{c.year}</span>
                        </span>
                    })}
                    <button on:click=nav(CalendarNav::NextMonth)>">"</button>
                </div>
                <div class="calendar-grid">
                    <div class="week-label"></div>
                    {move || calendar.with(|c| c.day_labels.iter().map(|label| view! { <div class="day-label">{label.clone()}</div> }).collect_view())}
                    {move || calendar.get().weeks.into_iter().map(|week| view! {
                        <div class="week-number" on:click=open(week.page)>{format!("{:02}", week.number)}</div>
                        {week.days.into_iter().map(day_cell).collect_view()}
                    }).collect_view()}
                </div>
            </div>
        </app-calendar>
    }
}
