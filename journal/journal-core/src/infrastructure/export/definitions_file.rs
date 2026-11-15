//! `settings/properties.json`: the definitions of the user keys in the Markdown folder, so
//! that every device gets them (ADR 046).

use std::collections::BTreeMap;

use serde::Serialize;

use super::export_service::format_instant;
use crate::domain::property_definitions::{Definition, Definitions};
use crate::error::AppResult;

/// Where the file is, relative to the Markdown folder.
pub const DEFINITIONS_FILE: &str = "settings/properties.json";

/// The text of the file. The keys are in dictionary order, so the same definitions give the
/// same bytes on every device.
pub fn write(definitions: &Definitions) -> String {
    let file = File {
        definitions: definitions
            .iter()
            .map(|(key, definition)| (key.as_str(), Entry::of(definition)))
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&file).expect("definitions are plain data");
    text.push('\n');
    text
}

/// The definitions in the text of the file.
pub fn read(_text: &str) -> AppResult<Definitions> {
    Ok(Definitions::new())
}

#[derive(Serialize)]
struct File<'a> {
    definitions: BTreeMap<&'a str, Entry<'a>>,
}

/// One key in the file. The fields are written in this order.
#[derive(Serialize)]
struct Entry<'a> {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    value_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted: Option<String>,
}

impl<'a> Entry<'a> {
    fn of(definition: &'a Definition) -> Entry<'a> {
        match definition {
            Definition::Defined {
                value_type,
                page,
                modified,
            } => Entry {
                value_type: Some(value_type),
                page: Some(*page),
                modified: Some(format_instant(*modified)),
                deleted: None,
            },
            Definition::Deleted { deleted } => Entry {
                value_type: None,
                page: None,
                modified: None,
                deleted: Some(format_instant(*deleted)),
            },
        }
    }
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

    // 書いたファイルを読むと、元の定義に戻る。
    #[test]
    fn a_written_file_reads_back_as_the_definitions() {
        let definitions: Definitions = [
            ("old".to_string(), Definition::Deleted { deleted: 1_500 }),
            (
                "owner".to_string(),
                Definition::Defined {
                    value_type: "text".to_string(),
                    page: true,
                    modified: 0,
                },
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(read(&write(&definitions)).unwrap(), definitions);
    }
}
