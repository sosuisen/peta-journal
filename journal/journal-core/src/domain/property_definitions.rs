//! The definitions of the user keys as `settings/properties.json` holds them, and how two
//! versions of them are put together key by key (ADR 046).

use std::collections::BTreeMap;

/// What one version says of a key: its definition, or that it was deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Definition {
    Defined {
        value_type: String,
        page: bool,
        /// When the key was defined, in milliseconds.
        modified: i64,
    },
    Deleted {
        /// When the key was deleted, in milliseconds.
        deleted: i64,
    },
}

/// The definitions of one version, by key.
pub type Definitions = BTreeMap<String, Definition>;

/// Puts two versions together, key by key.
pub fn merge(a: &Definitions, b: &Definitions) -> Definitions {
    let mut merged = a.clone();
    for (key, definition) in b {
        match merged.get(key) {
            Some(have) if have.rank() >= definition.rank() => {}
            _ => {
                merged.insert(key.clone(), definition.clone());
            }
        }
    }
    merged
}

impl Definition {
    /// Of two versions of a key, the one of the higher rank wins: the newer one, and of the
    /// same date, the deleted one, then a type other than `text`.
    fn rank(&self) -> (i64, bool, bool) {
        match self {
            Definition::Defined {
                value_type,
                modified,
                ..
            } => (*modified, false, value_type != "text"),
            Definition::Deleted { deleted } => (*deleted, true, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defined(value_type: &str, page: bool, modified: i64) -> Definition {
        Definition::Defined {
            value_type: value_type.to_string(),
            page,
            modified,
        }
    }

    fn version(entries: &[(&str, Definition)]) -> Definitions {
        entries
            .iter()
            .map(|(key, d)| (key.to_string(), d.clone()))
            .collect()
    }

    // 片方の版にしかないキーは、合わせた結果に残る。2台で別のキーを足せば、両方が残る。
    #[test]
    fn a_key_in_only_one_version_stays() {
        let a = version(&[("owner", defined("text", true, 1))]);
        let b = version(&[("score", defined("number", false, 2))]);

        let merged = merge(&a, &b);

        assert_eq!(
            merged,
            version(&[
                ("owner", defined("text", true, 1)),
                ("score", defined("number", false, 2)),
            ])
        );
    }

    // 同じキーは、新しい方を採る。どちらの版が新しくても同じである。消した定義の日時は、消した日時である。
    #[test]
    fn the_newer_one_of_a_key_wins_whichever_version_has_it() {
        let older = version(&[("score", defined("text", true, 1))]);
        let newer = version(&[("score", defined("number", false, 2))]);
        let deleted = version(&[("score", Definition::Deleted { deleted: 3 })]);

        assert_eq!(merge(&older, &newer), newer);
        assert_eq!(merge(&newer, &older), newer);
        assert_eq!(merge(&newer, &deleted), deleted);
        assert_eq!(merge(&deleted, &newer), deleted);
    }

    // 日時が同じなら、消した方を採る。
    #[test]
    fn of_the_same_date_the_deleted_one_wins() {
        let defined = version(&[("score", defined("number", true, 5))]);
        let deleted = version(&[("score", Definition::Deleted { deleted: 5 })]);

        assert_eq!(merge(&defined, &deleted), deleted);
        assert_eq!(merge(&deleted, &defined), deleted);
    }

    // 日時が同じなら、textよりtext以外の型を採る。textは、型の分からない値に与える型だからである。
    #[test]
    fn of_the_same_date_a_type_other_than_text_wins() {
        let text = version(&[("score", defined("text", true, 5))]);
        let number = version(&[("score", defined("number", true, 5))]);

        assert_eq!(merge(&text, &number), number);
        assert_eq!(merge(&number, &text), number);
    }

    // 日時が同じで、どちらもtext以外なら、型の名前の辞書順で後の方を採る。
    #[test]
    fn of_the_same_date_the_type_later_in_the_dictionary_wins() {
        let date = version(&[("plan", defined("date", true, 5))]);
        let datetime = version(&[("plan", defined("datetime", true, 5))]);

        assert_eq!(merge(&date, &datetime), datetime);
        assert_eq!(merge(&datetime, &date), datetime);
    }
}
