//! S2 of ADR 064: every kind of `Event` shows in the sources of a shell, or is excluded with a
//! reason.

use std::collections::BTreeMap;

use serde::Deserialize;

/// The language of the sources of a shell, which says how they name a kind of `Event`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// The generated Swift types name `Event::PasteText` as `.pasteText`, an implicit member.
    Swift,
    /// The generated C# types name `Event::PasteText` as `Event.PasteText`, a nested class.
    CSharp,
}

impl Language {
    /// The file extension of a source in this language.
    pub fn extension(self) -> &'static str {
        match self {
            Language::Swift => "swift",
            Language::CSharp => "cs",
        }
    }

    /// True when `text` names the kind `variant` of `Event`.
    ///
    /// In Swift the name is an implicit member such as `.pasteText`. The dot must not follow a
    /// name or a closing bracket: `core.pasteImage(...)` is a call of a method, not the event.
    /// A member of another enum with the same name, such as the `.redo` of `SelectionKey`,
    /// still counts: the check reads text, not types.
    ///
    /// In C# the name is a nested class of `Event`, such as `Event.PasteText`. `Event` must be a
    /// whole name: the `Back` of `KeyEvent.Back` is not the event.
    pub fn names(self, text: &str, variant: &str) -> bool {
        match self {
            Language::Swift => names_implicit_member(text, &lower_camel(variant)),
            Language::CSharp => names_whole(text, &format!("Event.{variant}")),
        }
    }
}

/// `name` starts with an ASCII letter, as every kind of `Event` does.
fn lower_camel(name: &str) -> String {
    let mut lower = name.to_string();
    lower[..1].make_ascii_lowercase();
    lower
}

/// True when `text` has `name` with no name character just before or after it.
fn names_whole(text: &str, name: &str) -> bool {
    text.match_indices(name).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + name.len()..].chars().next();
        !before.is_some_and(is_name_char) && !after.is_some_and(is_name_char)
    })
}

fn names_implicit_member(text: &str, member: &str) -> bool {
    let needle = format!(".{member}");
    text.match_indices(&needle).any(|(at, _)| {
        let before = text[..at].chars().next_back();
        let after = text[at + needle.len()..].chars().next();
        !before.is_some_and(ends_an_expression) && !after.is_some_and(is_name_char)
    })
}

fn ends_an_expression(c: char) -> bool {
    is_name_char(c) || matches!(c, ')' | ']' | '?' | '!')
}

fn is_name_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// What S2 found for one shell.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// The kinds of `Event` that the sources do not name and that no list excludes.
    pub missing: Vec<String>,
    /// The names in the exclusion lists that are no kind of `Event`, such as one that was
    /// renamed or removed from the core.
    pub unknown_exclusions: Vec<String>,
}

impl Report {
    /// True when S2 found nothing.
    pub fn is_clean(&self) -> bool {
        self.missing.is_empty() && self.unknown_exclusions.is_empty()
    }
}

/// Checks that each of `variants` is named in `sources`, or is excluded by one of `exclusions`.
/// Each exclusion list maps a kind to the reason why the shell need not name it.
pub fn check(
    variants: &[String],
    sources: &str,
    language: Language,
    exclusions: &[&BTreeMap<String, String>],
) -> Report {
    let excluded = |name: &str| exclusions.iter().any(|list| list.contains_key(name));
    let missing = variants
        .iter()
        .filter(|variant| !excluded(variant) && !language.names(sources, variant))
        .cloned()
        .collect();
    let unknown_exclusions = exclusions
        .iter()
        .flat_map(|list| list.keys())
        .filter(|name| !variants.contains(name))
        .cloned()
        .collect();
    Report {
        missing,
        unknown_exclusions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    fn exclusions(list: &[&str]) -> BTreeMap<String, String> {
        list.iter()
            .map(|name| (name.to_string(), "a reason".to_string()))
            .collect()
    }

    // Swiftでは、`Event::PasteText`は暗黙のメンバー`.pasteText`として書かれる。
    #[test]
    fn swift_names_a_kind_as_an_implicit_member() {
        assert!(Language::Swift.names("send(.pasteText(itemId: id))", "PasteText"));
        assert!(Language::Swift.names("turn(.back)", "Back"));
        assert!(Language::Swift.names("let e: Event = .undo\n", "Undo"));
    }

    // Swiftで、名前やかっこの後のドットはメソッドやプロパティなので、`Event`の種類に数えない。
    #[test]
    fn swift_does_not_count_a_member_of_a_value() {
        assert!(!Language::Swift.names("core.pasteImage(file)", "PasteImage"));
        assert!(!Language::Swift.names("shell().back()", "Back"));
        assert!(!Language::Swift.names("items[0].undo", "Undo"));
    }

    // Swiftで、長い名前の頭が一致しても、別の名前なので数えない。
    #[test]
    fn swift_does_not_count_a_longer_name() {
        assert!(!Language::Swift.names("send(.pasteTextLater)", "PasteText"));
    }

    // C#では、`Event::PasteText`は入れ子のクラス`Event.PasteText`として書かれる。
    #[test]
    fn csharp_names_a_kind_as_a_nested_class_of_event() {
        assert!(Language::CSharp.names("Send(new Event.PasteText(id, text));", "PasteText"));
        assert!(Language::CSharp.names("Turn(new Event.Back(Today()))", "Back"));
        assert!(Language::CSharp.names("if (sent is Event.Undo)", "Undo"));
    }

    // C#で、`Event`で終わる別の名前の中の同じ名前は、`Event`の種類に数えない。
    #[test]
    fn csharp_does_not_count_a_member_of_another_name() {
        assert!(!Language::CSharp.names("new KeyEvent.Back()", "Back"));
        assert!(!Language::CSharp.names("core.PasteImage(file)", "PasteImage"));
    }

    // C#で、長い名前の頭が一致しても、別の名前なので数えない。
    #[test]
    fn csharp_does_not_count_a_longer_name() {
        assert!(!Language::CSharp.names("new Event.PasteTextLater()", "PasteText"));
    }

    // ソースに出てこず、除外リストにも無い種類が、問題として出る。
    #[test]
    fn a_kind_neither_named_nor_excluded_is_missing() {
        let report = check(
            &names(&["Start", "PasteText", "PageLoaded"]),
            "send(.start)",
            Language::Swift,
            &[&exclusions(&["PageLoaded"])],
        );
        assert_eq!(report.missing, names(&["PasteText"]));
        assert!(report.unknown_exclusions.is_empty());
    }

    // 除外リストが複数あれば、どれか1つにある種類は問題にならない。
    #[test]
    fn a_kind_in_any_exclusion_list_is_not_missing() {
        let report = check(
            &names(&["PageLoaded", "Indent"]),
            "",
            Language::Swift,
            &[&exclusions(&["PageLoaded"]), &exclusions(&["Indent"])],
        );
        assert!(report.is_clean());
    }

    // `Event`に無い名前が除外リストにあれば、問題として出る。
    #[test]
    fn an_exclusion_that_is_no_kind_of_event_is_reported() {
        let report = check(
            &names(&["Start"]),
            "send(.start)",
            Language::Swift,
            &[&exclusions(&["Removed"])],
        );
        assert_eq!(report.unknown_exclusions, names(&["Removed"]));
        assert!(!report.is_clean());
    }
}
