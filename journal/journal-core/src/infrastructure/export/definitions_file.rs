//! `settings/properties.json`: the definitions of the user keys in the Markdown folder, so
//! that every device gets them (ADR 046).

use crate::domain::property_definitions::Definitions;

/// Where the file is, relative to the Markdown folder.
pub const DEFINITIONS_FILE: &str = "settings/properties.json";

/// The text of the file. The keys are in dictionary order, so the same definitions give the
/// same bytes on every device.
pub fn write(_definitions: &Definitions) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::property_definitions::Definition;

    // 定義はキーの辞書順に書かれ、消した定義は消した日時だけを持つ。
    #[test]
    fn the_definitions_are_written_in_the_order_of_their_keys() {
        let definitions: Definitions = [
            (
                "score".to_string(),
                Definition::Defined {
                    value_type: "number".to_string(),
                    page: false,
                    modified: 1_763_172_000_000,
                },
            ),
            (
                "old".to_string(),
                Definition::Deleted {
                    deleted: 1_763_110_800_000,
                },
            ),
            (
                "owner".to_string(),
                Definition::Defined {
                    value_type: "text".to_string(),
                    page: true,
                    modified: 1_763_168_400_000,
                },
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(
            write(&definitions),
            r#"{
  "definitions": {
    "old": {
      "deleted": "2025-11-14T09:00:00Z"
    },
    "owner": {
      "type": "text",
      "page": true,
      "modified": "2025-11-15T01:00:00Z"
    },
    "score": {
      "type": "number",
      "page": false,
      "modified": "2025-11-15T02:00:00Z"
    }
  }
}
"#
        );
    }
}
