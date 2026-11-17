//! Calendar dates without a clock.

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

    /// Parses `YYYY-MM-DD`; None for anything else, and for a day that does not exist.
    pub fn parse(s: &str) -> Option<Date> {
        let bytes = s.as_bytes();
        if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
            return None;
        }
        // `parse` alone would take a sign, as in `+026`
        let digits = |text: &str| text.bytes().all(|b| b.is_ascii_digit());
        if !(digits(&s[0..4]) && digits(&s[5..7]) && digits(&s[8..10])) {
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

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// The number of days of the month; 0 for a month that does not exist.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 日付は、1970-01-01からの日数にして、同じ日付に戻せる。
    #[test]
    fn a_date_is_counted_in_days_and_comes_back_from_them() {
        assert_eq!(Date::new(1970, 1, 1).to_days(), 0);
        assert_eq!(Date::new(1969, 12, 31).to_days(), -1);
        assert_eq!(Date::new(2000, 3, 1).to_days(), 11_017);
        for days in [-800_000, -1, 0, 59, 11_016, 11_017, 20_000, 800_000] {
            assert_eq!(Date::from_days(days).to_days(), days);
        }
        assert_eq!(Date::from_days(11_016), Date::new(2000, 2, 29));
        // Every day of some eight hundred years, with the ends of the centuries among them
        for days in -150_000..150_000 {
            assert_eq!(Date::from_days(days).to_days(), days);
        }
    }

    // 日付は、日数だけ前後に動かせる。月と年の変わり目も越える。
    #[test]
    fn a_date_moves_by_days_across_the_ends_of_months_and_years() {
        assert_eq!(Date::new(2026, 12, 31).shift(1), Date::new(2027, 1, 1));
        assert_eq!(Date::new(2024, 3, 1).shift(-1), Date::new(2024, 2, 29));
        assert_eq!(Date::new(2026, 2, 10).shift(0), Date::new(2026, 2, 10));
    }

    // 曜日は、日曜が0で、土曜が6である。
    #[test]
    fn the_weekday_is_0_for_a_sunday_and_6_for_a_saturday() {
        assert_eq!(Date::new(2026, 2, 8).weekday(), 0);
        assert_eq!(Date::new(2026, 2, 14).weekday(), 6);
        assert_eq!(Date::new(1969, 12, 28).weekday(), 0);
    }

    // `YYYY-MM-DD`の形で、ある日だけが日付として読まれる。
    #[test]
    fn only_a_day_that_exists_in_the_form_of_a_date_is_read() {
        assert_eq!(Date::parse("2024-02-29"), Some(Date::new(2024, 2, 29)));
        assert_eq!(Date::new(2024, 2, 29).iso(), "2024-02-29");
        assert_eq!(Date::new(987, 1, 2).iso(), "0987-01-02");
        for text in [
            "2026-02-30",
            "2026-13-01",
            "2026-00-10",
            "2026-01-00",
            "2026-1-01",
            "2026/01/01",
            "2026-01/01",
            "2026/01-01",
            "2026-01-01x",
            "+026-01-01",
            "2026-+1-01",
            "2026-01-+1",
            "",
        ] {
            assert_eq!(Date::parse(text), None, "{text}");
        }
    }

    // うるう年は、4で割れて100で割れない年と、400で割れる年である。
    #[test]
    fn a_leap_year_has_29_days_in_february() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(1900, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2026, 4), 30);
        assert_eq!(days_in_month(2026, 12), 31);
        assert_eq!(days_in_month(2026, 13), 0);
    }
}
