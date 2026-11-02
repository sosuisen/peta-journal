//! Week numbers, ported from `core/utils/week.utils.ts`.

use super::date::Date;
use crate::model::WeekNumbering;

/// The Thursday of the ISO week the date is in; ISO weeks belong to the year of their Thursday.
fn iso_thursday_of(date: Date) -> Date {
    // Monday = 0 .. Sunday = 6
    let monday_based = (date.weekday() + 6) % 7;
    date.shift(3 - monday_based as i64)
}

/// ISO 8601 week number: weeks start on Monday and week 1 holds January 4th.
pub fn iso_week_number(date: Date) -> u32 {
    let thursday = iso_thursday_of(date);
    let year_start = Date::new(thursday.year, 1, 4);
    let days = thursday.to_days() - year_start.to_days();
    let year_start_monday_based = (year_start.weekday() + 6) % 7;
    (1 + (days - 3 + year_start_monday_based as i64) / 7) as u32
}

/// The year an ISO week belongs to, which differs from the calendar year around New Year.
pub fn iso_week_year(date: Date) -> i32 {
    iso_thursday_of(date).year
}

/// US week number: weeks start on Sunday and week 1 holds January 1st.
pub fn us_week_number(date: Date) -> u32 {
    let year_start = Date::new(date.year, 1, 1);
    let year_start_sunday = year_start.shift(-(year_start.weekday() as i64));
    (1 + (date.to_days() - year_start_sunday.to_days()) / 7) as u32
}

/// The number of weeks a year has, so that week navigation does not skip the last one.
pub fn weeks_in_year(year: i32, numbering: WeekNumbering) -> u32 {
    match numbering {
        // December 28th is always in the last ISO week of its year
        WeekNumbering::Iso => iso_week_number(Date::new(year, 12, 28)),
        WeekNumbering::Us => us_week_number(Date::new(year, 12, 31)),
    }
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
        assert_eq!(us_week_number(Date::new(2026, 1, 1)), 1);
        assert_eq!(us_week_number(Date::new(2026, 1, 4)), 2);
        assert_eq!(us_week_number(Date::new(2026, 2, 10)), 7);
        // 2028 is a leap year starting on a Saturday: 54 weeks
        assert_eq!(weeks_in_year(2028, WeekNumbering::Us), 54);
        assert_eq!(weeks_in_year(2026, WeekNumbering::Us), 53);
    }
}
