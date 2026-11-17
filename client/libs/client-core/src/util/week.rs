//! Week numbers. The rules are in `journal-names`, which the journal numbers its weeks with
//! too, so a week has one name on both sides.

use super::date::Date;
use crate::model::WeekNumbering;
use journal_names::{week_of, JournalName};

impl From<WeekNumbering> for journal_names::WeekNumbering {
    fn from(numbering: WeekNumbering) -> Self {
        match numbering {
            WeekNumbering::Us => journal_names::WeekNumbering::Us,
            WeekNumbering::Iso => journal_names::WeekNumbering::Iso,
        }
    }
}

/// ISO 8601 week number: weeks start on Monday and week 1 holds January 4th.
pub fn iso_week_number(date: Date) -> u32 {
    week_of(date, journal_names::WeekNumbering::Iso).1
}

/// The year an ISO week belongs to, which differs from the calendar year around New Year.
pub fn iso_week_year(date: Date) -> i32 {
    week_of(date, journal_names::WeekNumbering::Iso).0
}

/// US week number: weeks start on Sunday, and are counted from the week that holds January
/// 1st of the year of that Sunday.
pub fn us_week_number(date: Date) -> u32 {
    week_of(date, journal_names::WeekNumbering::Us).1
}

/// The number of weeks a year has, so that week navigation does not skip the last one.
pub fn weeks_in_year(year: i32, numbering: WeekNumbering) -> u32 {
    journal_names::weeks_in_year(year, numbering.into())
}

/// The name of the week page of the week the date is in, like `2026-W07`. The calendar opens
/// the page by this name, so a week has one page whichever of its days is asked for.
pub fn week_page_name(date: Date, numbering: WeekNumbering) -> String {
    let (year, week) = week_of(date, numbering.into());
    JournalName::Weekly { year, week }.to_string()
}

/// The name of the week page a number of weeks after the week of this name; negative for
/// earlier weeks. None when the name is not the name of a week.
pub fn week_page_name_after(name: &str, weeks: i64, numbering: WeekNumbering) -> Option<String> {
    let Ok(JournalName::Weekly { year, week }) = name.parse() else {
        return None;
    };
    let first_day = journal_names::first_day_of_week(year, week, numbering.into());
    Some(week_page_name(first_day.shift(7 * weeks), numbering))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ISO 週番号と ISO 週の年は年末年始をまたいでも正しく、年の週数は53週の年と52週の年で区別される。
    #[test]
    fn iso_weeks() {
        assert_eq!(iso_week_number(Date::new(2026, 1, 1)), 1);
        assert_eq!(iso_week_number(Date::new(2021, 1, 1)), 53);
        assert_eq!(iso_week_year(Date::new(2021, 1, 1)), 2020);
        assert_eq!(iso_week_number(Date::new(2026, 2, 10)), 7);
        assert_eq!(iso_week_number(Date::new(2024, 12, 30)), 1);
        assert_eq!(weeks_in_year(2020, WeekNumbering::Iso), 53);
        assert_eq!(weeks_in_year(2026, WeekNumbering::Iso), 53);
        assert_eq!(weeks_in_year(2025, WeekNumbering::Iso), 52);
    }

    // US 週番号は日曜始まりで1月1日を第1週とし、土曜始まりのうるう年は54週になる。
    #[test]
    fn us_weeks() {
        // The week of a New Year that is not a Sunday is the last week of the old year
        assert_eq!(us_week_number(Date::new(2026, 1, 1)), 53);
        assert_eq!(us_week_number(Date::new(2023, 1, 1)), 1);
        assert_eq!(us_week_number(Date::new(2026, 1, 4)), 2);
        assert_eq!(us_week_number(Date::new(2026, 2, 10)), 7);
        // 2028 is a leap year starting on a Saturday: 54 weeks
        assert_eq!(weeks_in_year(2028, WeekNumbering::Us), 54);
        assert_eq!(weeks_in_year(2026, WeekNumbering::Us), 53);
    }

    // 週のページの名前は、その日を含む週の、年と週番号である。週はUSでは日曜、ISOでは月曜に始まる。
    #[test]
    fn the_week_page_of_a_date_is_named_by_the_year_and_the_number_of_its_week() {
        let tuesday = Date::new(2026, 2, 10);
        assert_eq!(week_page_name(tuesday, WeekNumbering::Us), "2026-W07");
        assert_eq!(week_page_name(tuesday, WeekNumbering::Iso), "2026-W07");
        // A Sunday starts a US week and ends an ISO week
        let sunday = Date::new(2026, 2, 8);
        assert_eq!(week_page_name(sunday, WeekNumbering::Us), "2026-W07");
        assert_eq!(week_page_name(sunday, WeekNumbering::Iso), "2026-W06");
    }

    // 年をまたぐ週は、週の最初の日の年に入る。ISOでは、木曜のある年に入る。
    #[test]
    fn a_week_across_new_year_belongs_to_one_year() {
        let new_year = Date::new(2027, 1, 1);
        assert_eq!(week_page_name(new_year, WeekNumbering::Us), "2026-W53");
        assert_eq!(week_page_name(new_year, WeekNumbering::Iso), "2026-W53");
        assert_eq!(
            week_page_name(Date::new(2024, 12, 30), WeekNumbering::Iso),
            "2025-W01"
        );
    }

    // 次の週と前の週の名前は、週の最初の日を7日動かして決まる。年の変わり目でも、同じ週が2つの名前にならない。
    #[test]
    fn the_week_after_and_before_a_week_is_named_once_across_new_year() {
        let us = WeekNumbering::Us;
        assert_eq!(
            week_page_name_after("2026-W07", 1, us).as_deref(),
            Some("2026-W08")
        );
        assert_eq!(
            week_page_name_after("2026-W53", 1, us).as_deref(),
            Some("2027-W02")
        );
        assert_eq!(
            week_page_name_after("2027-W02", -1, us).as_deref(),
            Some("2026-W53")
        );
        // `2027-W01` is another name of the week `2026-W53`, so it leads to the same weeks
        assert_eq!(
            week_page_name_after("2027-W01", 1, us).as_deref(),
            Some("2027-W02")
        );
        let iso = WeekNumbering::Iso;
        assert_eq!(
            week_page_name_after("2024-W52", 1, iso).as_deref(),
            Some("2025-W01")
        );
        assert_eq!(
            week_page_name_after("2025-W01", -1, iso).as_deref(),
            Some("2024-W52")
        );
        assert_eq!(week_page_name_after("2026-02-10", 1, iso), None);
    }
}
