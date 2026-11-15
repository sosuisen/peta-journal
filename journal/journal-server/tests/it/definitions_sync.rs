//! The definitions of the user keys go to the other devices in `settings/properties.json`
//! (ADR 046).

use journal_core::domain::property_definitions::Definition;
use journal_core::infrastructure::export::definitions_file::{self, DEFINITIONS_FILE};
use journal_core::journal::{AddPropertyDefinitionInput, ChangedBy};

use super::atlas_sync::{two_devices, Device};

impl Device {
    fn define(&self, key: &str, value_type: &str, page: bool) {
        self.state
            .journal
            .add_property_definition(AddPropertyDefinitionInput {
                key: Some(key.to_string()),
                value_type: Some(value_type.to_string()),
                page: Some(page),
            })
            .unwrap();
    }

    fn undefine(&self, key: &str) {
        self.state
            .journal
            .delete_property_definition(key, &ChangedBy::Mcp)
            .unwrap();
    }

    /// What this device wrote in `settings/properties.json` of a key
    fn written(&self, key: &str) -> Option<Definition> {
        let text = std::fs::read_to_string(self.root.join(DEFINITIONS_FILE)).ok()?;
        definitions_file::read(&text).unwrap().remove(key)
    }
}

// ユーザーが定義を足すと、端末は、定義を日時とともにsettings/properties.jsonに書く。
#[test]
fn a_definition_the_user_adds_is_written_with_its_date() {
    let dir = tempfile::tempdir().unwrap();
    let (a, _b) = two_devices(dir.path());
    let before = journal_core::infrastructure::persistence::db::now_millis();

    a.define("score", "number", false);

    let Some(Definition::Defined {
        value_type,
        page,
        modified,
    }) = a.written("score")
    else {
        panic!("score is not written");
    };
    assert_eq!((value_type.as_str(), page), ("number", false));
    assert!(modified >= before, "{modified} < {before}");
}
