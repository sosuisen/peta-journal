//! Classifies file names into journal types based on date patterns.

/// The type of journal period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JournalType {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    None,
}

fn all_digits(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// Classifies a file name into a journal type.
pub fn classify(file_name: &str) -> JournalType {
    if file_name.is_empty() {
        return JournalType::None;
    }
    let parts: Vec<&str> = file_name.split('-').collect();
    match parts.as_slice() {
        [y, m, d] if y.len() == 4 && m.len() == 2 && d.len() == 2
            && all_digits(y) && all_digits(m) && all_digits(d) => JournalType::Daily,
        [y, w] if y.len() == 4 && all_digits(y) && w.len() >= 2 && w.len() <= 3
            && w.starts_with('W') && all_digits(&w[1..]) => JournalType::Weekly,
        [y, m] if y.len() == 4 && m.len() == 2 && all_digits(y) && all_digits(m) => JournalType::Monthly,
        [y] if y.len() == 4 && all_digits(y) => JournalType::Yearly,
        _ => JournalType::None,
    }
}

/// Whether a file name matches any journal pattern.
pub fn is_journal_file_name(file_name: &str) -> bool {
    classify(file_name) != JournalType::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies() {
        assert_eq!(classify("2026-02-18"), JournalType::Daily);
        assert_eq!(classify("2026-W08"), JournalType::Weekly);
        assert_eq!(classify("2026-W8"), JournalType::Weekly);
        assert_eq!(classify("2026-02"), JournalType::Monthly);
        assert_eq!(classify("2026"), JournalType::Yearly);
        assert_eq!(classify("Ideas"), JournalType::None);
        assert_eq!(classify("2026-W123"), JournalType::None);
        assert_eq!(classify(""), JournalType::None);
    }
}
