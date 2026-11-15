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
pub fn merge(_a: &Definitions, _b: &Definitions) -> Definitions {
    Definitions::new()
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
}
