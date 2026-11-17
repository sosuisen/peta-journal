//! The settings of the journal.

use serde_json::json;

use crate::common::items::{answer, input};
use crate::common::Core;

impl Core {
    /// Changes the settings given, as the client does, and gives the settings after it
    fn update_config(&self, changes: serde_json::Value) -> serde_json::Value {
        answer(self.core.journal.update_config(input(changes)))
    }
}

// ヘルスチェックはokを返し、設定は読み書きできる。
#[test]
fn health_is_ok_and_the_config_is_read_and_updated() {
    let notes = Core::new();

    let config = answer(notes.core.journal.config());
    assert_eq!(config["weekNumbering"], json!("us"));
    assert_eq!(config["logicalOutdent"], json!(false));
    assert_eq!(config["tagsDir"], json!("tags/"));

    // The tags directory is stored in the dirPath form, whatever the client typed
    let config = notes.update_config(json!({ "tagsDir": "/meta/tags" }));
    assert_eq!(config["tagsDir"], json!("meta/tags/"));
    // The root is a value of its own, not the default
    let config = notes.update_config(json!({ "tagsDir": "" }));
    assert_eq!(config["tagsDir"], json!(""));
    assert_eq!(answer(notes.core.journal.config())["tagsDir"], json!(""));

    // Other settings leave the home link as it is
    notes.update_config(json!({ "homeLink": " [[a/Foo]] " }));
    let config = notes.update_config(json!({ "logicalOutdent": false }));
    assert_eq!(config["homeLink"], json!("a/Foo"));

    let definitions = answer(notes.core.journal.property_definitions().unwrap());
    assert_eq!(definitions["propertyDefinitions"]["plan"], json!("date"));
}
