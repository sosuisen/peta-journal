//! The month calendar: which month it shows, and what a shell draws of it (ADR 052). A shell
//! only draws the view and sends the events, so every shell numbers the weeks and names the
//! pages alike.

use crate::app::{done, Cmd, Model, Screen};
use crate::model::WeekNumbering;
use crate::util::date::{days_in_month, Date};
use crate::util::week::{iso_week_number, us_week_number, week_page_name};
use serde::Serialize;

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

/// Where the calendar goes.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, Serialize)]
pub enum CalendarNav {
    PrevMonth,
    NextMonth,
    /// Back to the month of today, as when a calendar opens.
    Today,
}

/// The month calendar as a shell draws it.
#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct CalendarView {
    pub year: i32,
    /// 1..=12
    pub month: u32,
    /// `February`
    pub month_name: String,
    /// The name of the journal of the month, like `2026-02`. A click on the month opens it.
    pub month_page: String,
    /// The name of the journal of the year, like `2026`.
    pub year_page: String,
    /// The names of the seven days of a week, from the day a week starts with.
    pub day_labels: Vec<String>,
    /// Full weeks, so the first and the last one may hold days of the months beside.
    pub weeks: Vec<CalendarWeek>,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct CalendarWeek {
    pub number: u32,
    /// The name of the journal of the week, like `2026-W07`.
    pub page: String,
    pub days: Vec<CalendarDay>,
}

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet))]
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct CalendarDay {
    /// `YYYY-MM-DD`, which is also the name of the journal of the day.
    pub date: String,
    /// The day of the month, as the cell shows it.
    pub day: u32,
    /// A day of the month before or after the one shown.
    pub other_month: bool,
    pub today: bool,
    /// The day of the journal on screen.
    pub selected: bool,
}

/// The year and the month the calendar shows: the one it was moved to, or that of today.
fn shown_month(model: &Model) -> (i32, u32) {
    model.calendar_month.unwrap_or_else(|| {
        // Before the shell told the date, there is no today
        let today = Date::parse(&model.today).unwrap_or(Date::new(1970, 1, 1));
        (today.year, today.month)
    })
}

pub fn nav(model: &mut Model, nav: CalendarNav) -> Cmd {
    let (year, month) = shown_month(model);
    model.calendar_month = match nav {
        CalendarNav::PrevMonth if month == 1 => Some((year - 1, 12)),
        CalendarNav::PrevMonth => Some((year, month - 1)),
        CalendarNav::NextMonth if month == 12 => Some((year + 1, 1)),
        CalendarNav::NextMonth => Some((year, month + 1)),
        // It follows today again, also over midnight
        CalendarNav::Today => None,
    };
    done()
}

pub fn view(model: &Model) -> CalendarView {
    let (year, month) = shown_month(model);
    let numbering = model.config.week_numbering;
    let iso = numbering == WeekNumbering::Iso;
    let selected = match &model.screen {
        Screen::Journal(journal) => journal.current_date.as_str(),
        _ => "",
    };
    let first = Date::new(year, month, 1);
    // The days of the month before, back to the day the week starts with
    let days_before = if iso {
        (first.weekday() + 6) % 7
    } else {
        first.weekday()
    };
    let start = first.shift(-i64::from(days_before));
    let cells = (days_before + days_in_month(year, month)).div_ceil(7) * 7;
    let days: Vec<Date> = (0..i64::from(cells)).map(|i| start.shift(i)).collect();
    let weeks = days
        .chunks(7)
        .map(|week| {
            let first_of_week = week[0];
            CalendarWeek {
                number: if iso {
                    iso_week_number(first_of_week)
                } else {
                    us_week_number(first_of_week)
                },
                page: week_page_name(first_of_week, numbering),
                days: week
                    .iter()
                    .map(|date| {
                        let iso_date = date.iso();
                        CalendarDay {
                            day: date.day,
                            other_month: date.month != month,
                            today: iso_date == model.today,
                            selected: iso_date == selected,
                            date: iso_date,
                        }
                    })
                    .collect(),
            }
        })
        .collect();
    let day_labels = if iso { ISO_DAY_LABELS } else { US_DAY_LABELS };
    CalendarView {
        year,
        month,
        month_name: MONTH_NAMES[(month - 1) as usize].to_string(),
        month_page: month_page_name(year, month),
        year_page: year_page_name(year),
        day_labels: day_labels.iter().map(|label| label.to_string()).collect(),
        weeks,
    }
}

/// The name of the journal of a month, like `2026-02`.
pub fn month_page_name(year: i32, month: u32) -> String {
    format!("{year:04}-{month:02}")
}

/// The name of the journal of a year, like `2026`.
pub fn year_page_name(year: i32) -> String {
    format!("{year:04}")
}
