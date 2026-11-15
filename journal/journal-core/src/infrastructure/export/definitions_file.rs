//! `settings/properties.json`: the definitions of the user keys in the Markdown folder, so
//! that every device gets them (ADR 046).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::export_service::{format_instant, MarkdownExportService};
use super::page_matcher::parse_instant;
use crate::domain::property_definitions::{merge, Definition, Definitions};
use crate::domain::property_key::PropertyKey;
use crate::error::{AppError, AppResult};
use crate::infrastructure::files::write_atomic;
use crate::infrastructure::persistence::database_initializer::SYSTEM_KEYS;
use crate::infrastructure::persistence::property_repository::{
    PropertyRepository, USER_VALUE_TYPES,
};

/// The folder of the settings in the Markdown folder.
const SETTINGS_DIR: &str = "settings";

/// Where the file is, relative to the Markdown folder.
pub const DEFINITIONS_FILE: &str = "settings/properties.json";

/// Writes `settings/properties.json` of this device.
pub struct DefinitionsFileService {
    properties: PropertyRepository,
    export: Arc<MarkdownExportService>,
}

impl DefinitionsFileService {
    pub fn new(
        properties: PropertyRepository,
        export: Arc<MarkdownExportService>,
    ) -> DefinitionsFileService {
        DefinitionsFileService { properties, export }
    }

    /// Writes the definitions of this device put together with those in the file, so that
    /// a definition of another device that is not taken in yet stays.
    pub fn write_file(&self) -> AppResult<()> {
        let Some(root) = self.export.markdown_root() else {
            return Ok(());
        };
        let path = root.join(DEFINITIONS_FILE);
        let on_disk = std::fs::read_to_string(&path).ok();
        let in_file = on_disk
            .as_deref()
            .and_then(|text| read(text).ok())
            .unwrap_or_default();
        let text = write(&merge(
            &self.properties.find_synced_definitions()?,
            &in_file,
        ));
        std::fs::create_dir_all(root.join(SETTINGS_DIR))?;
        write_atomic(&path, &text)?;
        Ok(())
    }
}

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
pub fn read(text: &str) -> AppResult<Definitions> {
    let file: ReadFile = serde_json::from_str(text)
        .map_err(|e| AppError::bad_request(format!("Not a file of definitions: {e}")))?;
    Ok(file
        .definitions
        .into_iter()
        .filter_map(|(key, entry)| Some((key, entry.definition()?)))
        .filter(|(key, definition)| a_user_can_make(key, definition))
        .collect())
}

/// Whether a user could have made the definition here. A file written by hand, or by
/// another version of the app, may hold others; they are left out.
fn a_user_can_make(key: &str, definition: &Definition) -> bool {
    if SYSTEM_KEYS.iter().any(|(system, _, _)| *system == key) {
        return false;
    }
    let Ok(key) = PropertyKey::parse(key) else {
        return false;
    };
    match definition {
        Definition::Defined { value_type, .. } => {
            USER_VALUE_TYPES.contains(&value_type.as_str()) && key.check_type(value_type).is_ok()
        }
        Definition::Deleted { .. } => true,
    }
}

#[derive(Deserialize)]
struct ReadFile {
    definitions: BTreeMap<String, ReadEntry>,
}

#[derive(Deserialize)]
struct ReadEntry {
    #[serde(rename = "type")]
    value_type: Option<String>,
    #[serde(default)]
    page: bool,
    modified: Option<String>,
    deleted: Option<String>,
}

impl ReadEntry {
    fn definition(self) -> Option<Definition> {
        if let Some(deleted) = self.deleted {
            return Some(Definition::Deleted {
                deleted: parse_instant(Some(&deleted))?,
            });
        }
        Some(Definition::Defined {
            value_type: self.value_type?,
            page: self.page,
            modified: parse_instant(self.modified.as_deref())?,
        })
    }
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

    // ユーザーが定義できない定義は読まず、ほかの定義は読む。システム管理キー、名前に合わない型、知らない型、キーにできない名前、日時のない定義である。
    #[test]
    fn a_definition_a_user_cannot_make_is_left_out_and_the_rest_come_in() {
        let text = r#"{
  "definitions": {
    "zoom": { "type": "number", "page": false, "modified": "2025-11-15T01:00:00Z" },
    "members": { "type": "list", "page": true, "modified": "2025-11-15T01:00:00Z" },
    "note_list": { "type": "text", "page": true, "modified": "2025-11-15T01:00:00Z" },
    "colour": { "type": "colour", "page": true, "modified": "2025-11-15T01:00:00Z" },
    "my key": { "type": "text", "page": true, "modified": "2025-11-15T01:00:00Z" },
    "undated": { "type": "text", "page": true },
    "owner": { "type": "text", "page": true, "modified": "1970-01-01T00:00:01Z" }
  }
}"#;

        let definitions = read(text).unwrap();

        assert_eq!(
            definitions.into_iter().collect::<Vec<_>>(),
            [(
                "owner".to_string(),
                Definition::Defined {
                    value_type: "text".to_string(),
                    page: true,
                    modified: 1_000,
                }
            )]
        );
    }
}
