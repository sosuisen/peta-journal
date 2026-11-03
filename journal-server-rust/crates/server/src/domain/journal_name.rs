//! The name of a journal page: a day, a week, a month or a year.

use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;

/// The period a journal page covers, parsed from its file name once at the boundary. A name
/// that is not a date form, or that names a day or a month that does not exist, is refused
/// there, so the code inside never sees one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalName {
    /// `YYYY-MM-DD`
    Daily(NaiveDate),
    /// `YYYY-WNN`; week 1 to 54. ISO weeks end at 53, but the US numbering, in which week 1
    /// holds January 1st, reaches 54 in a leap year that starts on a Saturday, like 2028.
    Weekly { year: i32, week: u32 },
    /// `YYYY-MM`
    Monthly { year: i32, month: u32 },
    /// `YYYY`
    Yearly(i32),
}

/// The text is not the name of a journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotAJournalName(String);

impl fmt::Display for NotAJournalName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Not a journal filename: {}", self.0)
    }
}

impl std::error::Error for NotAJournalName {}

impl FromStr for JournalName {
    type Err = NotAJournalName;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parse(text).ok_or_else(|| NotAJournalName(text.to_string()))
    }
}

fn parse(text: &str) -> Option<JournalName> {
    let parts: Vec<&str> = text.split('-').collect();
    match parts.as_slice() {
        [y, m, d] if m.len() == 2 && d.len() == 2 => {
            let date = NaiveDate::from_ymd_opt(year(y)?, number(m)?, number(d)?)?;
            Some(JournalName::Daily(date))
        }
        [y, w] if (2..=3).contains(&w.len()) && w.starts_with('W') => {
            let week = number(&w[1..]).filter(|w| (1..=54).contains(w))?;
            Some(JournalName::Weekly {
                year: year(y)?,
                week,
            })
        }
        [y, m] if m.len() == 2 => {
            let month = number(m).filter(|m| (1..=12).contains(m))?;
            Some(JournalName::Monthly {
                year: year(y)?,
                month,
            })
        }
        [y] => Some(JournalName::Yearly(year(y)?)),
        _ => None,
    }
}

/// Four digits
fn year(text: &str) -> Option<i32> {
    (text.len() == 4).then(|| number(text)).flatten()
}

/// Digits only
fn number<N: FromStr>(text: &str) -> Option<N> {
    // An empty text does not parse, so it needs no check of its own
    text.bytes()
        .all(|b| b.is_ascii_digit())
        .then(|| text.parse().ok())
        .flatten()
}

impl fmt::Display for JournalName {
    /// The file name of the journal, with the week and the month in two digits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalName::Daily(date) => write!(f, "{}", date.format("%Y-%m-%d")),
            JournalName::Weekly { year, week } => write!(f, "{year}-W{week:02}"),
            JournalName::Monthly { year, month } => write!(f, "{year}-{month:02}"),
            JournalName::Yearly(year) => write!(f, "{year}"),
        }
    }
}

/// Whether a file name is the name of a journal.
pub fn is_journal_file_name(file_name: &str) -> bool {
    file_name.parse::<JournalName>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 名前は日付の形で読まれ、週と月は2桁で書き戻される。
    #[test]
    fn a_name_is_read_by_its_date_form_and_written_with_two_digit_weeks_and_months() {
        let day = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
        let cases = [
            ("2026-02-18", JournalName::Daily(day), "2026-02-18"),
            (
                "2026-W08",
                JournalName::Weekly {
                    year: 2026,
                    week: 8,
                },
                "2026-W08",
            ),
            (
                "2026-W8",
                JournalName::Weekly {
                    year: 2026,
                    week: 8,
                },
                "2026-W08",
            ),
            (
                "2026-02",
                JournalName::Monthly {
                    year: 2026,
                    month: 2,
                },
                "2026-02",
            ),
            ("2026", JournalName::Yearly(2026), "2026"),
            (
                "2028-W54",
                JournalName::Weekly {
                    year: 2028,
                    week: 54,
                },
                "2028-W54",
            ),
        ];
        for (text, name, written) in cases {
            assert_eq!(text.parse::<JournalName>().unwrap(), name, "{text}");
            assert_eq!(name.to_string(), written);
        }
    }

    // 日付の形でない名前と、ない日・週・月は、ジャーナルの名前にならない。
    #[test]
    fn a_name_not_in_a_date_form_or_naming_a_day_week_or_month_that_does_not_exist_is_refused() {
        for text in [
            "Ideas",
            "",
            "2026-W123",
            "2026-W0",
            "2026-W55",
            "2026-13",
            "2026-00",
            "2026-02-30",
            "2026-2-18",
            "2026-2",
            "+026",
            "2026-W+1",
            "26-02-18",
            "２０２６",
        ] {
            assert!(text.parse::<JournalName>().is_err(), "{text}");
        }
        assert!(!is_journal_file_name("Ideas"));
        assert!(is_journal_file_name("2024-02-29"));
    }
}
