//! Page names: journal name patterns and the characters a file name may hold.

use journal_names::JournalName;

#[cfg_attr(feature = "facet_typegen", derive(facet::Facet), repr(C))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum JournalType {
    Daily,
    Weekly,
    Monthly,
    Yearly,
    None,
}

/// `2026-02-10` daily, `2026-W07` weekly, `2026-02` monthly, `2026` yearly; anything else none.
/// A name of a day, a week or a month that does not exist is none too, as the server refuses
/// it: both read the name with `journal-names`.
pub fn classify_journal_file_name(file_name: &str) -> JournalType {
    match file_name.parse::<JournalName>() {
        Ok(JournalName::Daily(_)) => JournalType::Daily,
        Ok(JournalName::Weekly { .. }) => JournalType::Weekly,
        Ok(JournalName::Monthly { .. }) => JournalType::Monthly,
        Ok(JournalName::Yearly(_)) => JournalType::Yearly,
        Err(_) => JournalType::None,
    }
}

pub fn is_journal_file_name(file_name: &str) -> bool {
    classify_journal_file_name(file_name) != JournalType::None
}

/// Validates a page title typed with its folders, like `notes/idea`, for use as a file name
/// on Windows, macOS and Linux. Returns an error message when invalid. The rule is the one of
/// the journal (`page-names`), so the journal refuses no title that passes here for its
/// characters.
pub fn validate_file_name(title: &str) -> Option<String> {
    page_names::check_path(title).err().map(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ファイル名は日次・週次・月次・年次のジャーナル種別に分類され、それ以外の名前・空文字・3桁の週番号は None になる。
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

    // ない日、ない月、ない週の名前は、サーバーが断るので、ジャーナルの名前にならない。
    #[test]
    fn a_name_of_a_day_a_month_or_a_week_that_does_not_exist_is_no_journal() {
        for name in ["2026-13", "2026-00", "2026-02-30", "2026-W00", "2026-W99"] {
            assert_eq!(
                classify_journal_file_name(name),
                JournalType::None,
                "{name}"
            );
            assert!(!is_journal_file_name(name), "{name}");
        }
        assert!(is_journal_file_name("2024-02-29"));
    }

    // 禁止文字・先頭の /・末尾のドット・Windows の予約名・制御文字を含むタイトルにはエラーメッセージが返り、有効なタイトルには None が返る。
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
