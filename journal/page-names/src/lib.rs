//! The names a page may take. A page is kept as a Markdown file of its name, so the name must
//! be a file name on Windows, macOS and Linux.
//!
//! The journal refuses a wrong name, and its clients say what is wrong before they ask. Both
//! read the rule here, so a client never lets through a name that the journal refuses.

use std::fmt;

/// The names Windows keeps for its devices, whatever their case.
const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Why a text is not the name of a page. It is shown as the message that the journal answers
/// with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotAPageName {
    ControlCharacter,
    Forbidden(char),
    SlashAtAnEnd,
    EndsWithDotOrSpace,
    Reserved(String),
}

impl fmt::Display for NotAPageName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotAPageName::ControlCharacter => {
                f.write_str("Title contains a control character (not allowed in filenames)")
            }
            NotAPageName::Forbidden(ch) => {
                write!(f, "Title contains '{ch}' (not allowed in filenames)")
            }
            NotAPageName::SlashAtAnEnd => f.write_str("Title cannot start or end with '/'"),
            NotAPageName::EndsWithDotOrSpace => f.write_str("Title cannot end with a dot or space"),
            NotAPageName::Reserved(name) => {
                write!(f, "Title '{name}' is a reserved name on Windows")
            }
        }
    }
}

impl std::error::Error for NotAPageName {}

/// Checks the name of one page, without its folders. The journal checks a name so: a `/` is
/// not a part of a file name.
pub fn check_file_name(name: &str) -> Result<(), NotAPageName> {
    check(name, false)
}

/// Checks a name typed with its folders, like `notes/idea`. A client checks a name so: a `/`
/// divides the folders from the name, and may not stand at an end.
pub fn check_path(path: &str) -> Result<(), NotAPageName> {
    check(path, true)
}

fn check(name: &str, slash_divides: bool) -> Result<(), NotAPageName> {
    for ch in name.chars() {
        if ch.is_control() {
            return Err(NotAPageName::ControlCharacter);
        }
        let forbidden = matches!(ch, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*')
            || (ch == '/' && !slash_divides);
        if forbidden {
            return Err(NotAPageName::Forbidden(ch));
        }
    }
    if name.starts_with('/') || name.ends_with('/') {
        return Err(NotAPageName::SlashAtAnEnd);
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err(NotAPageName::EndsWithDotOrSpace);
    }
    if RESERVED_NAMES.iter().any(|r| r.eq_ignore_ascii_case(name)) {
        return Err(NotAPageName::Reserved(name.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // 普通の名前、日本語の名前、途中に点や空白のある名前は、ページの名前になる。
    #[test]
    fn a_plain_name_is_a_page_name() {
        for name in ["hello", "メモ", "a.b", "a b", "con1", "COM10", "2026-02-10"] {
            assert_eq!(check_file_name(name), Ok(()), "{name}");
            assert_eq!(check_path(name), Ok(()), "{name}");
        }
    }

    // ファイル名に使えない文字は、その文字を挙げて断られる。
    #[test]
    fn a_character_that_no_file_name_holds_is_refused_by_name() {
        for ch in ['<', '>', ':', '"', '\\', '|', '?', '*'] {
            let name = format!("a{ch}b");
            assert_eq!(check_file_name(&name), Err(NotAPageName::Forbidden(ch)));
            assert_eq!(check_path(&name), Err(NotAPageName::Forbidden(ch)));
        }
        assert_eq!(
            NotAPageName::Forbidden(':').to_string(),
            "Title contains ':' (not allowed in filenames)"
        );
    }

    // 制御文字は、C0、DEL、C1のどれも断られる。
    #[test]
    fn a_control_character_is_refused() {
        for ch in [
            '\u{0}', '\u{1}', '\n', '\t', '\u{1f}', '\u{7f}', '\u{80}', '\u{9f}',
        ] {
            let name = format!("a{ch}b");
            assert_eq!(check_file_name(&name), Err(NotAPageName::ControlCharacter));
            assert_eq!(check_path(&name), Err(NotAPageName::ControlCharacter));
        }
        assert_eq!(check_file_name("a\u{a0}b"), Ok(()));
        assert_eq!(
            NotAPageName::ControlCharacter.to_string(),
            "Title contains a control character (not allowed in filenames)"
        );
    }

    // 1つのファイルの名前では、「/」は使えない文字である。
    #[test]
    fn a_slash_is_no_part_of_the_name_of_one_file() {
        assert_eq!(check_file_name("a/b"), Err(NotAPageName::Forbidden('/')));
        assert_eq!(check_file_name("/a"), Err(NotAPageName::Forbidden('/')));
    }

    // フォルダー付きの名前では、「/」は途中に書ける。先頭と末尾には書けない。
    #[test]
    fn a_slash_divides_a_path_but_does_not_stand_at_an_end() {
        assert_eq!(check_path("a/b"), Ok(()));
        assert_eq!(check_path("/a"), Err(NotAPageName::SlashAtAnEnd));
        assert_eq!(check_path("a/"), Err(NotAPageName::SlashAtAnEnd));
        assert_eq!(
            NotAPageName::SlashAtAnEnd.to_string(),
            "Title cannot start or end with '/'"
        );
    }

    // 点か空白で終わる名前は断られる。
    #[test]
    fn a_name_that_ends_with_a_dot_or_a_space_is_refused() {
        for name in ["a.", "a ", "."] {
            assert_eq!(check_file_name(name), Err(NotAPageName::EndsWithDotOrSpace));
            assert_eq!(check_path(name), Err(NotAPageName::EndsWithDotOrSpace));
        }
        assert_eq!(
            NotAPageName::EndsWithDotOrSpace.to_string(),
            "Title cannot end with a dot or space"
        );
    }

    // Windowsが装置に使う名前は、大文字と小文字を問わず断られる。
    #[test]
    fn a_name_that_windows_keeps_for_a_device_is_refused_in_any_case() {
        for name in ["CON", "con", "Nul", "COM1", "lpt9"] {
            assert_eq!(
                check_file_name(name),
                Err(NotAPageName::Reserved(name.to_string()))
            );
            assert_eq!(
                check_path(name),
                Err(NotAPageName::Reserved(name.to_string()))
            );
        }
        assert_eq!(
            NotAPageName::Reserved("con".into()).to_string(),
            "Title 'con' is a reserved name on Windows"
        );
    }
}
