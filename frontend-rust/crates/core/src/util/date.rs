//! Calendar dates without a clock: the core never asks what time it is, the shell tells it.

use std::fmt;

/// A calendar date in the proleptic Gregorian calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    pub year: i32,
    /// 1..=12
    pub month: u32,
    /// 1..=31
    pub day: u32,
}

impl Date {
    pub fn new(year: i32, month: u32, day: u32) -> Date {
        Date { year, month, day }
    }

    /// Parses `YYYY-MM-DD`; None for anything else.
    pub fn parse(s: &str) -> Option<Date> {
        let bytes = s.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        let year = s[0..4].parse().ok()?;
        let month = s[5..7].parse().ok()?;
        let day = s[8..10].parse().ok()?;
        let date = Date { year, month, day };
        if (1..=12).contains(&month) && day >= 1 && day <= days_in_month(year, month) {
            Some(date)
        } else {
            None
        }
    }

    /// Days since 1970-01-01 (negative before it). Howard Hinnant's days_from_civil.
    pub fn to_days(self) -> i64 {
        let y = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        } as i64;
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400;
        let m = self.month as i64;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + self.day as i64 - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date a number of days after 1970-01-01. Howard Hinnant's civil_from_days.
    pub fn from_days(days: i64) -> Date {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        Date {
            year: (if m <= 2 { y + 1 } else { y }) as i32,
            month: m,
            day: d,
        }
    }

    /// Moves the date by a number of days; negative for earlier dates.
    pub fn shift(self, days: i64) -> Date {
        Date::from_days(self.to_days() + days)
    }

    /// 0 = Sunday .. 6 = Saturday, like JavaScript's `getDay`.
    pub fn weekday(self) -> u32 {
        ((self.to_days() + 4).rem_euclid(7)) as u32
    }

    /// The date `YYYY-MM-DD`.
    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.iso())
    }
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

/// Moves an ISO date (YYYY-MM-DD) by a number of days. Other strings are returned as they are.
pub fn shift_iso_date(iso_date: &str, days: i64) -> String {
    match Date::parse(iso_date) {
        Some(d) => d.shift(days).iso(),
        None => iso_date.to_string(),
    }
}

const JA_WEEKDAYS: [&str; 7] = ["日", "月", "火", "水", "木", "金", "土"];

/// Formats a journal file name as the Angular app did with the `ja-JP` locale:
/// `2026` → `2026年`, `2026-02` → `2026年2月`, `2026-W07` → `2026 Week 07`,
/// `2026-02-10` → `2026年2月10日 火曜日`. Anything else is returned as it is.
pub fn format_journal_date(file_name: &str) -> String {
    if file_name.len() == 4 && file_name.chars().all(|c| c.is_ascii_digit()) {
        return format!("{file_name}年");
    }
    if let Some((y, m)) = file_name.split_once('-') {
        if y.len() == 4 && m.len() == 2 && y.chars().chain(m.chars()).all(|c| c.is_ascii_digit()) {
            let month: u32 = m.parse().unwrap_or(0);
            return format!("{y}年{month}月");
        }
        if let Some(week) = m.strip_prefix('W') {
            if y.len() == 4 && !week.is_empty() && week.chars().all(|c| c.is_ascii_digit()) {
                return format!("{y} Week {week}");
            }
        }
    }
    match Date::parse(file_name) {
        Some(d) => format!(
            "{}年{}月{}日 {}曜日",
            d.year,
            d.month,
            d.day,
            JA_WEEKDAYS[d.weekday() as usize]
        ),
        None => file_name.to_string(),
    }
}

/// A local wall-clock time the shell read for the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LocalDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl LocalDateTime {
    pub fn date(&self) -> Date {
        Date::new(self.year, self.month, self.day)
    }
}

/// `Untitled-YYYYMMDDHHMM` for a page created without a name.
pub fn generate_untitled_title(now: LocalDateTime) -> String {
    format!(
        "Untitled-{:04}{:02}{:02}{:02}{:02}",
        now.year, now.month, now.day, now.hour, now.minute
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn day_arithmetic_round_trips() {
        let d = Date::parse("2026-02-10").unwrap();
        assert_eq!(d.weekday(), 2); // Tuesday
        assert_eq!(d.shift(1).iso(), "2026-02-11");
        assert_eq!(d.shift(-41).iso(), "2025-12-31");
        assert_eq!(Date::from_days(0).iso(), "1970-01-01");
        assert_eq!(
            Date::parse("2024-02-29").unwrap().shift(366).iso(),
            "2025-03-01"
        );
        assert!(Date::parse("2026-02-30").is_none());
    }

    #[test]
    fn formats_like_the_angular_app() {
        assert_eq!(format_journal_date("2024"), "2024年");
        assert_eq!(format_journal_date("2024-08"), "2024年8月");
        assert_eq!(format_journal_date("2026-W07"), "2026 Week 07");
        assert_eq!(format_journal_date("2026-02-10"), "2026年2月10日 火曜日");
        assert_eq!(format_journal_date("notes"), "notes");
        assert_eq!(shift_iso_date("2026-03-01", -1), "2026-02-28");
    }
}
