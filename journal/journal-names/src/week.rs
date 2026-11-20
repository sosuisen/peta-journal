//! The weeks of a year and their numbers.
//!
//! A week has one name, whichever of its days is asked for. In the ISO numbering, a week
//! starts on Monday and belongs to the year of its Thursday, so week 1 holds January 4th. In
//! the US numbering, a week starts on Sunday and belongs to the year of its Saturday, so week
//! 1 holds January 1st. The days of December in that week belong to the new year. Excel splits
//! that week in two and keeps those days in the old year (ADR 072).

use crate::date::{days_in_month, Date};
use crate::name::JournalName;

/// How weeks are numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekNumbering {
    /// Weeks start on Sunday, and week 1 holds January 1st.
    Us,
    /// ISO 8601: weeks start on Monday, and week 1 holds January 4th.
    Iso,
}

/// The first day of the week the date is in.
fn start_of_week(date: Date, numbering: WeekNumbering) -> Date {
    let days_into_week = match numbering {
        WeekNumbering::Us => date.weekday(),
        // Monday = 0 .. Sunday = 6
        WeekNumbering::Iso => (date.weekday() + 6) % 7,
    };
    date.shift(-i64::from(days_into_week))
}

/// The first day of week 1 of the year.
fn start_of_week_1(year: i32, numbering: WeekNumbering) -> Date {
    let held_by_week_1 = match numbering {
        WeekNumbering::Us => Date::new(year, 1, 1),
        WeekNumbering::Iso => Date::new(year, 1, 4),
    };
    start_of_week(held_by_week_1, numbering)
}

/// The year and the number of the week the date is in.
pub fn week_of(date: Date, numbering: WeekNumbering) -> (i32, u32) {
    let start = start_of_week(date, numbering);
    let year = match numbering {
        // The Saturday of the week
        WeekNumbering::Us => start.shift(6).year,
        // The Thursday of the week
        WeekNumbering::Iso => start.shift(3).year,
    };
    let weeks = (start.to_days() - start_of_week_1(year, numbering).to_days()) / 7;
    (year, weeks as u32 + 1)
}

/// The first day of a week of a year. A number past the last week of the year counts on
/// into the next year.
pub fn first_day_of_week(year: i32, week: u32, numbering: WeekNumbering) -> Date {
    start_of_week_1(year, numbering).shift(7 * (i64::from(week) - 1))
}

/// The number of weeks a year has, so that nothing skips the last one.
pub fn weeks_in_year(year: i32, numbering: WeekNumbering) -> u32 {
    let in_the_last_week = match numbering {
        // December 28th is always in the last ISO week of its year
        WeekNumbering::Iso => Date::new(year, 12, 28),
        // December 25th is always in the last US week of its year: the Saturday of its week
        // is December 25th to 31st
        WeekNumbering::Us => Date::new(year, 12, 25),
    };
    week_of(in_the_last_week, numbering).1
}

/// The weeks that hold a day of the month, in their order, each as the name of its journal.
pub fn weeks_of_month(year: i32, month: u32, numbering: WeekNumbering) -> Vec<JournalName> {
    let mut weeks: Vec<JournalName> = Vec::new();
    for day in 1..=days_in_month(year, month) {
        let (year, week) = week_of(Date::new(year, month, day), numbering);
        let name = JournalName::Weekly { year, week };
        if weeks.last() != Some(&name) {
            weeks.push(name);
        }
    }
    weeks
}

#[cfg(test)]
mod tests {
    use super::*;
    use WeekNumbering::{Iso, Us};

    fn names(weeks: Vec<JournalName>) -> Vec<String> {
        weeks.iter().map(JournalName::to_string).collect()
    }

    // ISOの週は月曜に始まり、木曜のある年に入る。
    #[test]
    fn an_iso_week_starts_on_monday_and_belongs_to_the_year_of_its_thursday() {
        assert_eq!(week_of(Date::new(2026, 1, 1), Iso), (2026, 1));
        assert_eq!(week_of(Date::new(2021, 1, 1), Iso), (2020, 53));
        assert_eq!(week_of(Date::new(2026, 2, 10), Iso), (2026, 7));
        assert_eq!(week_of(Date::new(2024, 12, 30), Iso), (2025, 1));
        // A Sunday ends an ISO week
        assert_eq!(week_of(Date::new(2026, 2, 8), Iso), (2026, 6));
        assert_eq!(week_of(Date::new(2026, 2, 9), Iso), (2026, 7));
    }

    // USの週は日曜に始まる。1月1日のある週が第1週である。
    #[test]
    fn a_us_week_starts_on_sunday_and_week_1_holds_january_1st() {
        // 2023 starts on a Sunday
        assert_eq!(week_of(Date::new(2023, 1, 1), Us), (2023, 1));
        assert_eq!(week_of(Date::new(2023, 1, 8), Us), (2023, 2));
        assert_eq!(week_of(Date::new(2026, 2, 10), Us), (2026, 7));
        assert_eq!(week_of(Date::new(2026, 2, 8), Us), (2026, 7));
        assert_eq!(week_of(Date::new(2026, 2, 7), Us), (2026, 6));
    }

    // 年をまたぐUSの週は、その週の土曜の年に入る。1月1日を含む週が、新しい年の第1週である。
    #[test]
    fn a_us_week_across_new_year_belongs_to_the_year_of_its_saturday() {
        // 2027 starts on a Friday
        assert_eq!(week_of(Date::new(2026, 12, 26), Us), (2026, 52));
        assert_eq!(week_of(Date::new(2026, 12, 27), Us), (2027, 1));
        assert_eq!(week_of(Date::new(2027, 1, 1), Us), (2027, 1));
        assert_eq!(week_of(Date::new(2027, 1, 2), Us), (2027, 1));
        assert_eq!(week_of(Date::new(2027, 1, 3), Us), (2027, 2));
    }

    // 週の最初の日は、その週の年と番号から求められる。
    #[test]
    fn the_first_day_of_a_week_is_found_from_its_year_and_number() {
        assert_eq!(first_day_of_week(2026, 8, Us), Date::new(2026, 2, 15));
        assert_eq!(first_day_of_week(2026, 8, Iso), Date::new(2026, 2, 16));
        assert_eq!(first_day_of_week(2026, 53, Us), Date::new(2026, 12, 27));
        assert_eq!(first_day_of_week(2026, 53, Iso), Date::new(2026, 12, 28));
        assert_eq!(first_day_of_week(2025, 1, Iso), Date::new(2024, 12, 30));
        // Week 1 of a US year that does not start on a Sunday starts in December
        assert_eq!(first_day_of_week(2027, 1, Us), Date::new(2026, 12, 27));
    }

    // 年の週の数は、ISOでもUSでも52か53である。
    #[test]
    fn a_year_has_52_or_53_weeks() {
        assert_eq!(weeks_in_year(2020, Iso), 53);
        assert_eq!(weeks_in_year(2026, Iso), 53);
        assert_eq!(weeks_in_year(2025, Iso), 52);
        // A US year has 53 weeks when it has 53 Saturdays
        assert_eq!(weeks_in_year(2026, Us), 52);
        assert_eq!(weeks_in_year(2023, Us), 52);
        // 2022 starts on a Saturday
        assert_eq!(weeks_in_year(2022, Us), 53);
        // 2016 is a leap year that starts on a Friday
        assert_eq!(weeks_in_year(2016, Us), 53);
        // 2028 is a leap year that starts on a Saturday
        assert_eq!(weeks_in_year(2028, Us), 53);
    }

    // 月の週は、その月の日を含む週を、順に並べたものである。
    #[test]
    fn the_weeks_of_a_month_are_the_weeks_that_hold_its_days() {
        assert_eq!(
            names(weeks_of_month(2026, 2, Us)),
            ["2026-W06", "2026-W07", "2026-W08", "2026-W09"]
        );
        assert_eq!(
            names(weeks_of_month(2026, 2, Iso)),
            ["2026-W05", "2026-W06", "2026-W07", "2026-W08", "2026-W09"]
        );
        assert!(weeks_of_month(2026, 13, Us).is_empty());
    }

    // 年をまたぐ週は、12月と1月のどちらの月でも、同じ名前である。
    #[test]
    fn a_week_across_new_year_has_the_same_name_in_december_and_in_january() {
        assert_eq!(
            names(weeks_of_month(2024, 12, Iso)).last().unwrap(),
            "2025-W01"
        );
        assert_eq!(names(weeks_of_month(2025, 1, Iso))[0], "2025-W01");
        assert_eq!(
            names(weeks_of_month(2026, 12, Iso)).last().unwrap(),
            "2026-W53"
        );
        assert_eq!(names(weeks_of_month(2027, 1, Iso))[0], "2026-W53");
        assert_eq!(
            names(weeks_of_month(2026, 12, Us))[3..],
            ["2026-W52", "2027-W01"]
        );
        assert_eq!(
            names(weeks_of_month(2027, 1, Us))[..2],
            ["2027-W01", "2027-W02"]
        );
    }
}
