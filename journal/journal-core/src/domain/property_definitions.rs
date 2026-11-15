//! The definitions of the user keys as `settings/properties.json` holds them, and how two
//! versions of them are put together key by key (ADR 046).

use std::collections::BTreeMap;

/// What one version says of a key: its definition, or that it was deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Definition {
    /// The type is not here: the name of the key gives it (ADR 047).
    Defined {
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
    /// same date, the deleted one, then the one for pages.
    fn rank(&self) -> (i64, bool, bool) {
        match self {
            Definition::Defined { page, modified } => (*modified, false, *page),
            Definition::Deleted { deleted } => (*deleted, true, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defined(page: bool, modified: i64) -> Definition {
        Definition::Defined { page, modified }
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
        let a = version(&[("owner", defined(true, 1))]);
        let b = version(&[("score", defined(false, 2))]);

        let merged = merge(&a, &b);

        assert_eq!(
            merged,
            version(&[("owner", defined(true, 1)), ("score", defined(false, 2)),])
        );
    }

    // 同じキーは、新しい方を採る。消した定義の日時は、消した日時である。
    #[test]
    fn the_newer_one_of_a_key_wins() {
        let older = version(&[("score", defined(true, 1))]);
        let newer = version(&[("score", defined(false, 2))]);
        let deleted = version(&[("score", Definition::Deleted { deleted: 3 })]);

        assert_eq!(merge(&older, &newer), newer);
        assert_eq!(merge(&newer, &deleted), deleted);
    }

    // 日時が同じなら、消した方を採る。
    #[test]
    fn of_the_same_date_the_deleted_one_wins() {
        let defined = version(&[("score", defined(true, 5))]);
        let deleted = version(&[("score", Definition::Deleted { deleted: 5 })]);

        assert_eq!(merge(&defined, &deleted), deleted);
    }

    // 日時が同じなら、pageが真の方を採る。偽にするほうが影響が大きいためである。
    #[test]
    fn of_the_same_date_the_one_for_pages_wins() {
        let for_items = version(&[("owner", defined(false, 5))]);
        let for_pages = version(&[("owner", defined(true, 5))]);

        assert_eq!(merge(&for_items, &for_pages), for_pages);
    }

    mod properties {
        use proptest::prelude::*;

        use super::super::*;

        // Few keys and dates, so that two versions often share a key and a date
        fn definition() -> impl Strategy<Value = Definition> {
            prop_oneof![
                (any::<bool>(), 0i64..3)
                    .prop_map(|(page, modified)| Definition::Defined { page, modified }),
                (0i64..3).prop_map(|deleted| Definition::Deleted { deleted }),
            ]
        }

        fn definitions() -> impl Strategy<Value = Definitions> {
            prop::collection::btree_map(
                prop::sample::select(vec!["a".to_string(), "b".to_string(), "c".to_string()]),
                definition(),
                0..4,
            )
        }

        proptest! {
            // 2つの版を入れ替えて合わせても、結果は同じである。どちらの端末が合わせても、同じ定義になる。
            #[test]
            fn the_order_of_two_versions_does_not_matter(a in definitions(), b in definitions()) {
                prop_assert_eq!(merge(&a, &b), merge(&b, &a));
            }

            // 3つの版は、どの順に合わせても、結果は同じである。版が届く順で、定義は変わらない。
            #[test]
            fn the_order_of_three_versions_does_not_matter(
                a in definitions(),
                b in definitions(),
                c in definitions(),
            ) {
                prop_assert_eq!(merge(&merge(&a, &b), &c), merge(&a, &merge(&b, &c)));
            }

            // 合わせた結果に、どちらかの版をもう一度合わせても、結果は変わらない。だから、ファイルの書き直しは止まる。
            #[test]
            fn a_merged_version_stays_when_merged_again(a in definitions(), b in definitions()) {
                let merged = merge(&a, &b);

                prop_assert_eq!(&merge(&merged, &a), &merged);
                prop_assert_eq!(&merge(&merged, &b), &merged);
            }
        }
    }
}
