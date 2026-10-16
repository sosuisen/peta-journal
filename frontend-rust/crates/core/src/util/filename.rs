//! Page names: journal name patterns and the characters a file name may hold.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalType {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    None,
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

/// `2026-02-10` daily, `2026-W07` weekly, `2026-02` monthly, `2026` yearly; anything else none.
pub fn classify_journal_file_name(file_name: &str) -> JournalType {
    let parts: Vec<&str> = file_name.split('-').collect();
    match parts.as_slice() {
        [y] if y.len() == 4 && all_digits(y) => JournalType::Yearly,
        [y, m] if y.len() == 4 && all_digits(y) => {
            if m.len() == 2 && all_digits(m) {
                JournalType::Monthly
            } else if let Some(w) = m.strip_prefix('W') {
                if (1..=2).contains(&w.len()) && all_digits(w) {
                    JournalType::Weekly
                } else {
                    JournalType::None
                }
            } else {
                JournalType::None
            }
        }
        [y, m, d]
            if y.len() == 4 && m.len() == 2 && d.len() == 2 && all_digits(y) && all_digits(m) && all_digits(d) =>
        {
            JournalType::Daily
        }
        _ => JournalType::None,
    }
}

pub fn is_journal_file_name(file_name: &str) -> bool {
    classify_journal_file_name(file_name) != JournalType::None
}

const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
    "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Validates a page title for use as a file name on Windows, macOS and Linux.
/// Returns an error message when invalid.
pub fn validate_file_name(title: &str) -> Option<String> {
    for ch in title.chars() {
        if ch.is_control() {
            return Some("Title contains a control character (not allowed in filenames)".to_string());
        }
        if matches!(ch, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*') {
            return Some(format!("Title contains '{ch}' (not allowed in filenames)"));
        }
    }
    if title.starts_with('/') || title.ends_with('/') {
        return Some("Title cannot start or end with '/'".to_string());
    }
    if title.ends_with('.') || title.ends_with(' ') {
        return Some("Title cannot end with a dot or space".to_string());
    }
    if RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(title)) {
        return Some(format!("Title '{title}' is a reserved name on Windows"));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_journal_names() {
        assert_eq!(classify_journal_file_name("2026-02-10"), JournalType::Daily);
        assert_eq!(classify_journal_file_name("2026-W7"), JournalType::Weekly);
        assert_eq!(classify_journal_file_name("2026-W07"), JournalType::Weekly);
        assert_eq!(classify_journal_file_name("2026-02"), JournalType::Monthly);
        assert_eq!(classify_journal_file_name("2026"), JournalType::Yearly);
        assert_eq!(classify_journal_file_name("notes"), JournalType::None);
        assert_eq!(classify_journal_file_name(""), JournalType::None);
        assert_eq!(classify_journal_file_name("2026-W123"), JournalType::None);
    }

    #[test]
    fn validates_file_names() {
        assert_eq!(validate_file_name("hello"), None);
        assert_eq!(validate_file_name("a/b"), None);
        assert!(validate_file_name("a:b").unwrap().contains("':'"));
        assert!(validate_file_name("/a").is_some());
        assert!(validate_file_name("a.").is_some());
        assert!(validate_file_name("con").unwrap().contains("reserved"));
        assert!(validate_file_name("a\u{1}").unwrap().contains("control"));
    }
}
