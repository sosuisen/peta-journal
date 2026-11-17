//! The bodies of the requests to the journal, and the messages of its event stream. The
//! journal reads the requests with these types and the frontend core writes them with the same
//! types. The messages go the other way. So the two cannot name a field differently.
//!
//! The crate has no dependency but serde: the frontend core is also built for wasm, where the
//! journal core, with its database, cannot go.
//!
//! The types are as loose as the journal reads them. A field the journal can do without is an
//! `Option` or has a default, and the journal says what is wrong with a value. A field that is
//! not set is not written, so a request says only what its sender set.

use serde::{Deserialize, Serialize};

fn is_false(value: &bool) -> bool {
    !*value
}

// ---- pages ----

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GetOrCreateDocInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdatePageInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir_path: Option<String>,
    /// Deletes another page that has the new name, instead of refusing the update.
    #[serde(skip_serializing_if = "is_false")]
    pub overwrite: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DemotePageInput {
    /// Deletes another page that has the new name, instead of refusing the demotion.
    #[serde(skip_serializing_if = "is_false")]
    pub overwrite: bool,
}

/// The value of a property of a page or an item.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PropertyInput {
    /// No type means `text`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

// ---- journals ----

/// The items that move to the journal of a day.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToJournalInput {
    pub item_ids: Vec<String>,
    pub position: String,
    /// The text the editor of the first item has now, when it is not saved yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

// ---- items ----

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CreateItemInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    pub level: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The ID the client made for the item. The journal makes one when it is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateItemInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collapsed: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MoveItemsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_page_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_delta: Option<i32>,
    /// The body the editor of the first item has now, when it is not saved yet
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// The items an operation is about.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ItemIdsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct BatchInput {
    pub ops: Vec<BatchOpInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum BatchOpInput {
    #[serde(rename_all = "camelCase")]
    Create {
        id: String,
        page_id: String,
        position: String,
        level: i32,
        #[serde(default)]
        body: String,
    },
    Update {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        body: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        level: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        collapsed: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        task: Option<TaskStateInput>,
    },
    #[serde(rename_all = "camelCase")]
    Move {
        ids: Vec<String>,
        page_id: String,
        position: String,
    },
    Delete {
        id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TaskStateInput {
    pub task: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub begin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MergeItemsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub into_id: Option<String>,
    /// The body an editor has now for the item that goes away, when it is not saved yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// The same for the item that takes the text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub into_body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PasteInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub cursor: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// `auto`, `flat` or `raw`. No mode means `auto`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SplitInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    pub cursor: usize,
    /// The transclusion that shows the item, for an item of another page shown below it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct StashInput {
    /// The text the editor of the item has now, when it is not saved yet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ShiftLevelsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_ids: Option<Vec<String>>,
    pub delta: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TransclusionInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_item_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TreeMarkdownInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree_markdown: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FlatMarkdownInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<String>,
}

// ---- atlas ----

/// A link card: the page or the address it shows, and where it is.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AtlasLinkInput {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bounds: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AtlasGroupInput {
    /// Each a card by its item id, or a page card by the id of its page
    pub cards: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AtlasTitleInput {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub side: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AtlasCalloutInput {
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bounds: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub side: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub text: String,
}

// ---- settings ----

/// The settings to change. A setting not given stays as it is.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateConfigInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub week_numbering: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_outdent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags_dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub home_link: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AddPropertyDefinitionInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    /// Whether the key may be set on a page; true unless given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<bool>,
}

// ---- what the journal server does for its clients ----

/// The answer of the user to a question of an extension.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerConfirmInput {
    pub ok: bool,
}

/// An address for the machine of the server to open.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LaunchUrlInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// ---- the event stream ----

/// What the journal tells its clients on the event stream (`/api/events`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    /// A page changed, so a client that shows it reads it again.
    #[serde(rename_all = "camelCase")]
    PagesUpdated {
        page_id: String,
        source: ChangeSource,
        /// The client that made the change, for a change of a client (ADR-015).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        client_id: Option<String>,
    },
    /// An extension asks the user. The answer goes back with the ID of the question.
    #[serde(rename_all = "camelCase")]
    Confirm { request_id: String, message: String },
    /// The property definitions changed, so the clients read them again (ADR 046).
    PropertyDefinitionsUpdated,
}

/// Where the change of a page came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeSource {
    /// A client, through the API.
    App,
    /// An MCP tool.
    Mcp,
    /// A file of the markdown folder.
    File,
    /// An extension, which wrote items after the answer of a tag add.
    Extension,
    /// The journal itself, which wrote an atlas.
    Server,
    /// A source that a newer journal names and this client does not know.
    #[serde(other)]
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ページの変更の知らせは、変更したクライアントがいるときだけ、そのIDを書く。
    #[test]
    fn a_change_of_a_page_names_its_client_only_when_there_is_one() {
        let by_client = ClientMessage::PagesUpdated {
            page_id: "p".into(),
            source: ChangeSource::App,
            client_id: Some("c".into()),
        };
        let by_file = ClientMessage::PagesUpdated {
            page_id: "p".into(),
            source: ChangeSource::File,
            client_id: None,
        };

        assert_eq!(
            serde_json::to_value(&by_client).unwrap(),
            json!({ "type": "pagesUpdated", "pageId": "p", "source": "app", "clientId": "c" })
        );
        assert_eq!(
            serde_json::to_value(&by_file).unwrap(),
            json!({ "type": "pagesUpdated", "pageId": "p", "source": "file" })
        );
    }

    // 知らない出どころの変更も、ページの変更として読まれる。
    #[test]
    fn a_change_from_a_source_that_is_not_known_is_still_read() {
        let read: ClientMessage = serde_json::from_value(
            json!({ "type": "pagesUpdated", "pageId": "p", "source": "later" }),
        )
        .unwrap();

        assert_eq!(
            read,
            ClientMessage::PagesUpdated {
                page_id: "p".into(),
                source: ChangeSource::Other,
                client_id: None,
            }
        );
    }

    // 質問と、定義の変更の知らせは、書いた形のまま読み戻される。
    #[test]
    fn a_question_and_a_change_of_the_definitions_are_read_back_as_written() {
        let question = ClientMessage::Confirm {
            request_id: "r".into(),
            message: "Go on?".into(),
        };

        let written = serde_json::to_value(&question).unwrap();

        assert_eq!(
            written,
            json!({ "type": "confirm", "requestId": "r", "message": "Go on?" })
        );
        assert_eq!(
            serde_json::from_value::<ClientMessage>(written).unwrap(),
            question
        );
        assert_eq!(
            serde_json::to_value(ClientMessage::PropertyDefinitionsUpdated).unwrap(),
            json!({ "type": "propertyDefinitionsUpdated" })
        );
    }

    // 設定していない欄は、要求に書かれない。
    #[test]
    fn a_field_that_is_not_set_is_not_written() {
        let update = UpdatePageInput {
            title: Some("New".into()),
            ..Default::default()
        };
        let link = AtlasLinkInput {
            target: "Page".into(),
            ..Default::default()
        };

        assert_eq!(
            serde_json::to_value(update).unwrap(),
            json!({ "title": "New" })
        );
        assert_eq!(
            serde_json::to_value(link).unwrap(),
            json!({ "target": "Page" })
        );
    }

    // 上書きの指定は、指定したときだけ要求に書かれる。
    #[test]
    fn the_overwrite_is_written_only_when_it_is_asked_for() {
        let asked = DemotePageInput { overwrite: true };
        let not_asked = DemotePageInput { overwrite: false };

        assert_eq!(
            serde_json::to_value(asked).unwrap(),
            json!({ "overwrite": true })
        );
        assert_eq!(serde_json::to_value(not_asked).unwrap(), json!({}));
    }

    // 書いた要求は、同じ値に読み戻される。
    #[test]
    fn a_written_request_is_read_back_as_it_was() {
        let batch = BatchInput {
            ops: vec![
                BatchOpInput::Create {
                    id: "i".into(),
                    page_id: "p".into(),
                    position: "last".into(),
                    level: 1,
                    body: "text".into(),
                },
                BatchOpInput::Update {
                    id: "i".into(),
                    body: None,
                    level: Some(2),
                    collapsed: None,
                    task: Some(TaskStateInput {
                        task: true,
                        ..Default::default()
                    }),
                },
            ],
        };

        let written = serde_json::to_value(&batch).unwrap();

        assert_eq!(
            written,
            json!({ "ops": [
                { "op": "create", "id": "i", "pageId": "p", "position": "last", "level": 1,
                  "body": "text" },
                { "op": "update", "id": "i", "level": 2, "task": { "task": true } },
            ] })
        );
        assert_eq!(
            serde_json::from_value::<BatchInput>(written).unwrap(),
            batch
        );
    }
}
