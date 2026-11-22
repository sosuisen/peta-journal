//! Calendar dates without a clock: the core never asks what time it is, the shell tells it.

// The date itself is in `journal-names`, which the journal names its pages with too
pub use journal_names::{days_in_month, is_leap_year, Date};

/// Epoch millis as the server writes an instant: ISO 8601 UTC, with the milliseconds only
/// when they are not zero (`2026-01-02T03:04:05Z`, `2026-01-02T03:04:05.123Z`).
pub fn format_instant(millis: i64) -> String {
    let days = millis.div_euclid(86_400_000);
    let rest = millis.rem_euclid(86_400_000);
    let date = Date::from_days(days);
    let (h, m, s, ms) = (
        rest / 3_600_000,
        rest % 3_600_000 / 60_000,
        rest % 60_000 / 1000,
        rest % 1000,
    );
    let fraction = if ms == 0 {
        String::new()
    } else {
        format!(".{ms:03}")
    };
    format!("{}T{h:02}:{m:02}:{s:02}{fraction}Z", date.iso())
}

/// Epoch millis of an RFC 3339 instant such as `2026-01-02T03:04:05Z`,
/// `2026-01-02T03:04:05.123+09:00` or `2026-01-02T03:04+09:00`. A bare date counts as its
/// midnight UTC. None for anything else.
pub fn parse_rfc3339_millis(s: &str) -> Option<i64> {
    let s = s.trim();
    let date = Date::parse(s.get(0..10)?)?;
    let day_millis = date.to_days() * 86_400_000;
    let rest = &s[10..];
    if rest.is_empty() {
        return Some(day_millis);
    }
    let rest = rest.strip_prefix(['T', 't', ' '])?;
    let pos = rest.rfind(['Z', 'z', '+', '-'])?;
    let offset = &rest[pos..];
    let offset_millis = if offset.eq_ignore_ascii_case("Z") {
        0
    } else {
        let sign = if offset.starts_with('-') { -1 } else { 1 };
        let (h, m) = offset[1..].split_once(':')?;
        sign * (h.parse::<i64>().ok()? * 60 + m.parse::<i64>().ok()?) * 60_000
    };
    let clock = &rest[..pos];
    let (clock, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    let mut parts = clock.split(':');
    let h: i64 = parts.next()?.parse().ok()?;
    let m: i64 = parts.next()?.parse().ok()?;
    let sec: i64 = parts.next().map(|p| p.parse().ok()).unwrap_or(Some(0))?;
    if parts.next().is_some() || h > 23 || m > 59 || sec > 60 {
        return None;
    }
    let millis = if fraction.is_empty() {
        0
    } else {
        // Keep three digits: `.1` is 100 ms and `.123456` is 123 ms
        let digits: String = fraction.chars().chain("00".chars()).take(3).collect();
        digits.parse::<i64>().ok()?
    };
    Some(day_millis + ((h * 60 + m) * 60 + sec) * 1000 + millis - offset_millis)
}

/// How long a task took, from its `begin` to its `end` instant: `1h5m`, `12m`, or `30s` under a
/// minute. The seconds are shown only then. An instant that cannot be read, or an end before
/// the begin, counts as no time.
pub fn task_duration(begin: &str, end: &str) -> String {
    let millis = match (parse_rfc3339_millis(begin), parse_rfc3339_millis(end)) {
        (Some(begin), Some(end)) => (end - begin).max(0),
        _ => 0,
    };
    let seconds = millis / 1000;
    let (h, m, s) = (seconds / 3600, seconds % 3600 / 60, seconds % 60);
    match (h, m) {
        (0, 0) => format!("{s}s"),
        (0, m) => format!("{m}m"),
        (h, 0) => format!("{h}h"),
        (h, m) => format!("{h}h{m}m"),
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
    /// The name of the journal of this day, `YYYY-MM-DD`. Every shell tells the core the day
    /// of today with it.
    pub fn day_name(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
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
    use crate::effects::LocalDateTimeDto;

    // 作業時間は、時と分で示す。0の単位は書かず、1分未満のときだけ秒で示す。
    #[test]
    fn a_task_takes_hours_and_minutes_and_seconds_only_under_a_minute() {
        let at = |clock: &str| format!("2026-01-02T{clock}Z");
        let took = |end: &str| task_duration(&at("10:00:00"), &at(end));
        assert_eq!(took("11:05:59"), "1h5m");
        assert_eq!(took("12:00:30"), "2h");
        assert_eq!(took("10:12:00"), "12m");
        assert_eq!(took("10:00:30"), "30s");
    }

    // 読めない時刻と、始まりより前の終わりは、0秒である。
    #[test]
    fn an_unreadable_instant_or_an_end_before_the_begin_takes_no_time() {
        assert_eq!(task_duration("soon", "2026-01-02T10:00:00Z"), "0s");
        assert_eq!(
            task_duration("2026-01-02T10:00:00Z", "2026-01-02T09:00:00Z"),
            "0s"
        );
    }

    // 日のジャーナルの名前は、年4桁、月2桁、日2桁を「-」でつないだものである。
    #[test]
    fn the_name_of_the_journal_of_a_day_is_its_date_with_two_digits_for_month_and_day() {
        let day = LocalDateTime {
            year: 2026,
            month: 3,
            day: 5,
            hour: 23,
            minute: 59,
        };
        assert_eq!(day.day_name(), "2026-03-05");
        assert_eq!(LocalDateTimeDto::from(day).day_name(), "2026-03-05");
    }

    // RFC 3339 の時刻文字列はサーバーの書式と同じエポックミリ秒に解析され、日付だけなら UTC の0時になり、オフセットのない時刻や日付でない文字列は None になる。
    #[test]
    fn rfc3339_instants_parse_to_the_millis_the_server_formats() {
        assert_eq!(
            parse_rfc3339_millis("2020-01-02T03:04:05Z"),
            Some(1_577_934_245_000)
        );
        assert_eq!(
            parse_rfc3339_millis("2020-01-02T03:04:05.123Z"),
            Some(1_577_934_245_123)
        );
        assert_eq!(
            parse_rfc3339_millis("2020-01-02T12:04:05+09:00"),
            Some(1_577_934_245_000)
        );
        assert_eq!(
            parse_rfc3339_millis("2020-01-02T03:04Z"),
            Some(1_577_934_240_000)
        );
        assert_eq!(parse_rfc3339_millis("2020-01-02"), Some(1_577_923_200_000));
        assert_eq!(parse_rfc3339_millis("2020-01-02T03:04:05"), None);
        assert_eq!(parse_rfc3339_millis("yesterday"), None);
    }

    // エポックミリ秒はサーバーが書くのと同じ ISO 8601 UTC 形式に整形され、ミリ秒は0でないときだけ付く。
    #[test]
    fn instants_are_formatted_like_the_server_writes_them() {
        assert_eq!(format_instant(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_instant(1_577_934_245_000), "2020-01-02T03:04:05Z");
        assert_eq!(
            format_instant(1_577_934_245_123),
            "2020-01-02T03:04:05.123Z"
        );
    }

    // 日付の曜日計算・日数シフト・エポック日からの復元はうるう年をまたいでも正しく、存在しない日付の解析は None になる。
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

    // ジャーナルのファイル名は Angular アプリと同じ日本語表記（年・年月・週・年月日と曜日）に整形され、日付でない名前はそのまま返り、ISO 日付は日数分ずらされる。
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
