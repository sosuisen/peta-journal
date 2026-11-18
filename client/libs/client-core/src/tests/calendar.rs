//! The month calendar: the weeks it shows, and the month it is moved to.

use crate::app::{Event, Route};
use crate::harness::Harness;
use crate::model::WeekNumbering;
use crate::screens::calendar::{CalendarDay, CalendarNav, CalendarView};

const TODAY: &str = "2026-02-10";

fn visit(h: &mut Harness, name: &str) {
    h.send(Event::Route {
        route: Route::Page {
            name: name.into(),
            fragment: None,
        },
        today: TODAY.into(),
    });
}

fn calendar(numbering: WeekNumbering) -> (Harness, CalendarView) {
    let mut h = Harness::new();
    h.model.config.week_numbering = numbering;
    visit(&mut h, TODAY);
    let calendar = h.view().calendar;
    (h, calendar)
}

fn all_days(calendar: &CalendarView) -> Vec<&CalendarDay> {
    calendar.weeks.iter().flat_map(|w| &w.days).collect()
}

/// The days of the month of each week, in a row
fn days(calendar: &CalendarView) -> Vec<Vec<u32>> {
    calendar
        .weeks
        .iter()
        .map(|week| week.days.iter().map(|day| day.day).collect())
        .collect()
}

fn week_numbers(calendar: &CalendarView) -> Vec<u32> {
    calendar.weeks.iter().map(|week| week.number).collect()
}

/// The dates of the days that have a mark
fn marked(calendar: &CalendarView, mark: fn(&CalendarDay) -> bool) -> Vec<&str> {
    all_days(calendar)
        .into_iter()
        .filter(|d| mark(d))
        .map(|d| d.date.as_str())
        .collect()
}

// カレンダーは、今日の月を、日曜から始まる週で見せる。
#[test]
fn the_calendar_shows_the_month_of_today_in_weeks_from_sunday() {
    let (_, calendar) = calendar(WeekNumbering::Us);

    assert_eq!((calendar.year, calendar.month), (2026, 2));
    assert_eq!(calendar.month_name, "February");
    assert_eq!(
        calendar.day_labels,
        ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
    );
    // February 2026 starts on a Sunday and ends on a Saturday
    assert_eq!(
        days(&calendar),
        [
            [1, 2, 3, 4, 5, 6, 7],
            [8, 9, 10, 11, 12, 13, 14],
            [15, 16, 17, 18, 19, 20, 21],
            [22, 23, 24, 25, 26, 27, 28],
        ]
    );
    assert_eq!(week_numbers(&calendar), [6, 7, 8, 9]);
    assert!(marked(&calendar, |d| d.other_month).is_empty());
}

// ISOの週番号では、週は月曜から始まる。前の月と次の月の日で週を7日に満たし、別の月の印を付ける。
#[test]
fn iso_weeks_start_on_monday_and_are_filled_with_marked_days_of_the_months_beside() {
    let (_, calendar) = calendar(WeekNumbering::Iso);

    assert_eq!(
        calendar.day_labels,
        ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
    );
    assert_eq!(
        days(&calendar),
        [
            [26, 27, 28, 29, 30, 31, 1],
            [2, 3, 4, 5, 6, 7, 8],
            [9, 10, 11, 12, 13, 14, 15],
            [16, 17, 18, 19, 20, 21, 22],
            [23, 24, 25, 26, 27, 28, 1],
        ]
    );
    assert_eq!(week_numbers(&calendar), [5, 6, 7, 8, 9]);
    assert_eq!(
        marked(&calendar, |d| d.other_month),
        [
            "2026-01-26",
            "2026-01-27",
            "2026-01-28",
            "2026-01-29",
            "2026-01-30",
            "2026-01-31",
            "2026-03-01"
        ]
    );
}

// 日、週、月、年には、クリックで開くジャーナルの名前が付く。
#[test]
fn a_day_a_week_the_month_and_the_year_carry_the_name_of_their_journal() {
    let (_, calendar) = calendar(WeekNumbering::Iso);

    assert_eq!(calendar.month_page, "2026-02");
    assert_eq!(calendar.year_page, "2026");
    assert_eq!(calendar.weeks[2].page, "2026-W07");
    assert_eq!(calendar.weeks[2].days[1].date, "2026-02-10");
}

// 今日の日には今日の印が、画面のジャーナルの日には選択の印が付く。
#[test]
fn today_and_the_day_of_the_journal_on_screen_are_marked() {
    let mut h = Harness::new();
    visit(&mut h, "2026-02-12");
    let calendar = h.view().calendar;
    assert_eq!(marked(&calendar, |d| d.today), ["2026-02-10"]);
    assert_eq!(marked(&calendar, |d| d.selected), ["2026-02-12"]);

    // A page that is no journal selects no day
    visit(&mut h, "notes");
    assert!(marked(&h.view().calendar, |d| d.selected).is_empty());
}

// 前の月と次の月へ動かせる。年の変わり目も越える。今日へ戻すと、今日の月になる。
#[test]
fn the_calendar_moves_by_a_month_across_the_year_and_comes_back_to_today() {
    let (mut h, _) = calendar(WeekNumbering::Us);
    let shown = |h: &Harness| {
        let calendar = h.view().calendar;
        (calendar.year, calendar.month, calendar.month_name)
    };

    h.send(Event::CalendarNav(CalendarNav::PrevMonth));
    assert_eq!(shown(&h), (2026, 1, "January".to_string()));
    h.send(Event::CalendarNav(CalendarNav::PrevMonth));
    assert_eq!(shown(&h), (2025, 12, "December".to_string()));
    assert_eq!(h.view().calendar.month_page, "2025-12");
    assert_eq!(h.view().calendar.year_page, "2025");
    h.send(Event::CalendarNav(CalendarNav::NextMonth));
    assert_eq!(shown(&h), (2026, 1, "January".to_string()));
    h.send(Event::CalendarNav(CalendarNav::NextMonth));
    h.send(Event::CalendarNav(CalendarNav::NextMonth));
    assert_eq!(shown(&h), (2026, 3, "March".to_string()));

    h.send(Event::CalendarNav(CalendarNav::Today));
    assert_eq!(shown(&h), (2026, 2, "February".to_string()));
}

// 年をまたぐ週は、12月と1月のどちらのカレンダーでも、同じ番号と同じ名前である。
#[test]
fn a_week_across_new_year_has_one_number_and_one_name_in_december_and_in_january() {
    let (mut h, _) = calendar(WeekNumbering::Iso);
    h.send(Event::CalendarNav(CalendarNav::PrevMonth));
    h.send(Event::CalendarNav(CalendarNav::PrevMonth));
    let december = h.view().calendar;
    h.send(Event::CalendarNav(CalendarNav::NextMonth));
    let january = h.view().calendar;

    // Monday 2025-12-29 starts week 1 of 2026
    let last = december.weeks.last().unwrap();
    assert_eq!((last.number, last.page.as_str()), (1, "2026-W01"));
    assert_eq!(january.weeks[0].number, 1);
    assert_eq!(january.weeks[0].page, "2026-W01");
    assert_eq!(january.weeks[0].days[0].date, "2025-12-29");
}

// 6週にわたる月も、すべての日が入る。
#[test]
fn a_month_over_six_weeks_holds_all_its_days() {
    let (mut h, _) = calendar(WeekNumbering::Us);
    for _ in 0..3 {
        h.send(Event::CalendarNav(CalendarNav::NextMonth));
    }
    // May 2026 starts on a Friday
    let may = h.view().calendar;
    assert_eq!(may.weeks.len(), 6);
    assert_eq!(days(&may)[0], [26, 27, 28, 29, 30, 1, 2]);
    assert_eq!(days(&may)[5], [31, 1, 2, 3, 4, 5, 6]);
}

// シェルが日付を知らせる前のカレンダーは、今日のない月を見せる。
#[test]
fn the_calendar_shows_a_month_without_today_before_the_shell_told_the_date() {
    let calendar = Harness::new().view().calendar;
    assert_eq!((calendar.year, calendar.month), (1970, 1));
    assert!(marked(&calendar, |d| d.today).is_empty());
}
