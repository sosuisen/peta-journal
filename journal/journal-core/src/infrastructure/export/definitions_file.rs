//! `settings/properties.json`: the definitions of the user keys in the Markdown folder, so
//! that every device gets them (ADR 046).

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use super::export_service::{format_instant, MarkdownExportService};
use super::import_service::Outcome;
use super::page_matcher::parse_instant;
use super::path_mapper::has_copy_marker;
use crate::domain::property_definitions::{merge, Definition, Definitions};
use crate::domain::property_key::PropertyKey;
use crate::error::{AppError, AppResult};
use crate::infrastructure::files::write_atomic;
use crate::infrastructure::persistence::database_initializer::SYSTEM_KEYS;
use crate::infrastructure::persistence::property_repository::{
    PropertyDefinition, PropertyRepository,
};

/// The folder of the settings in the Markdown folder.
pub const SETTINGS_DIR: &str = "settings";

/// Where the file is, relative to the Markdown folder.
pub const DEFINITIONS_FILE: &str = "settings/properties.json";

/// Writes `settings/properties.json` of this device.
pub struct DefinitionsFileService {
    properties: PropertyRepository,
    export: Arc<MarkdownExportService>,
    /// Held while the file is read, put together and written. The requests of the clients
    /// and the import write it from many threads: two at once wrote the same temporary
    /// file, and each would write what it read without what the other added.
    file: Mutex<()>,
}

impl DefinitionsFileService {
    pub fn new(
        properties: PropertyRepository,
        export: Arc<MarkdownExportService>,
    ) -> DefinitionsFileService {
        DefinitionsFileService {
            properties,
            export,
            file: Mutex::new(()),
        }
    }

    /// Writes the definitions of this device put together with those in the file, so that
    /// a definition of another device that is not taken in yet stays.
    pub fn write_file(&self) -> AppResult<()> {
        let _file = self.file.lock();
        self.write_file_locked()
    }

    fn write_file_locked(&self) -> AppResult<()> {
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

impl DefinitionsFileService {
    /// All the definitions of this device, as the clients see them. `None` when the DB
    /// cannot tell.
    pub fn find_definitions(&self) -> Option<Vec<PropertyDefinition>> {
        self.properties.find_definitions().ok()
    }

    /// Takes in the file as another device wrote it: the definitions are put together with
    /// those of this device, key by key, and the DB gets the result (ADR 046).
    pub fn take_in(&self, root: &Path) -> AppResult<Outcome> {
        let _file = self.file.lock();
        let text = std::fs::read_to_string(root.join(DEFINITIONS_FILE))?;
        let merged = self.put_together(&read(&text)?)?;
        // The other device does not have what only this one has: it gets it in the file
        let merged_text = write(&merged);
        if merged_text != text {
            write_atomic(&root.join(DEFINITIONS_FILE), &merged_text)?;
        }
        Ok(Outcome::Imported)
    }

    /// Takes in a conflicted copy of the file as one more version, writes the file with all
    /// of it, and deletes the copy.
    pub fn take_in_copy(&self, root: &Path, rel_path: &str) -> AppResult<Outcome> {
        let _file = self.file.lock();
        let copy = root.join(rel_path);
        self.put_together(&read(&std::fs::read_to_string(&copy)?)?)?;
        self.write_file_locked()?;
        std::fs::remove_file(&copy)?;
        Ok(Outcome::Imported)
    }

    /// Puts another version together with the definitions of this device, key by key, and
    /// gives the DB the result.
    fn put_together(&self, theirs: &Definitions) -> AppResult<Definitions> {
        let ours = self.properties.find_synced_definitions()?;
        let merged = merge(&ours, theirs);
        for (key, definition) in &merged {
            if ours.get(key) == Some(definition) {
                continue;
            }
            match definition {
                Definition::Defined { page, modified } => {
                    self.properties.put_definition(key, *page, *modified)?
                }
                Definition::Deleted { deleted } => self.properties.put_deletion(key, *deleted)?,
            }
        }
        Ok(merged)
    }
}

/// Whether the file is a conflicted copy of `settings/properties.json`, as Dropbox names it.
pub fn is_copy_of_the_file(rel_path: &str) -> bool {
    rel_path
        .strip_prefix("settings/properties (")
        .and_then(|rest| rest.strip_suffix(").json"))
        .is_some_and(|note| !note.contains('/') && has_copy_marker(note))
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
        .filter_map(|(key, entry)| Some((key.clone(), entry.definition(&key)?)))
        .filter(|(key, _)| a_user_can_make(key))
        .collect())
}

/// Whether a user could have made the definition here. A file written by hand, or by
/// another version of the app, may hold others; they are left out.
fn a_user_can_make(key: &str) -> bool {
    !SYSTEM_KEYS.iter().any(|(system, _, _)| *system == key) && PropertyKey::parse(key).is_ok()
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
    /// The definition of the key. A type written by an older version must be the one the
    /// name gives, or the definition is left out (ADR 047).
    fn definition(self, key: &str) -> Option<Definition> {
        if let Some(deleted) = self.deleted {
            return Some(Definition::Deleted {
                deleted: parse_instant(Some(&deleted))?,
            });
        }
        if self
            .value_type
            .is_some_and(|value_type| value_type != property_names::type_of(key).as_str())
        {
            return None;
        }
        Some(Definition::Defined {
            page: self.page,
            modified: parse_instant(self.modified.as_deref())?,
        })
    }
}

#[derive(Serialize)]
struct File<'a> {
    definitions: BTreeMap<&'a str, Entry>,
}

/// One key in the file. The fields are written in this order.
#[derive(Serialize)]
struct Entry {
    #[serde(skip_serializing_if = "Option::is_none")]
    page: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modified: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted: Option<String>,
}

impl Entry {
    fn of(definition: &Definition) -> Entry {
        match definition {
            Definition::Defined { page, modified } => Entry {
                page: Some(*page),
                modified: Some(format_instant(*modified)),
                deleted: None,
            },
            Definition::Deleted { deleted } => Entry {
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

    // 定義はキーの辞書順に、型を書かずに書かれ、消した定義は消した日時だけを持つ。型は名前から決まるからである。
    #[test]
    fn the_definitions_are_written_in_the_order_of_their_keys_without_their_types() {
        let definitions: Definitions = [
            (
                "score_n".to_string(),
                Definition::Defined {
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
      "page": true,
      "modified": "2025-11-15T01:00:00Z"
    },
    "score_n": {
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
                    page: true,
                    modified: 0,
                },
            ),
        ]
        .into_iter()
        .collect();

        assert_eq!(read(&write(&definitions)).unwrap(), definitions);
    }

    // 型のない定義は、名前から決まる型で読まれる。
    #[test]
    fn a_definition_without_a_type_reads_as_the_type_its_name_gives() {
        let text = r#"{ "definitions": { "rating_n": { "page": true, "modified": "1970-01-01T00:00:01Z" } } }"#;

        assert_eq!(
            read(text).unwrap().remove("rating_n"),
            Some(Definition::Defined {
                page: true,
                modified: 1_000,
            })
        );
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
                    page: true,
                    modified: 1_000,
                }
            )]
        );
    }
}
