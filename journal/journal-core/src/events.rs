//! What the clients are told: which pages changed, and the questions of the extensions.
//! The messages are the JSON of the event stream of the REST API (`/api/events`). The app
//! around the core carries them to its clients: the journal server over SSE, an app that
//! embeds the core straight to its frontend.

use std::sync::Arc;

use hmtid::Hmtid;
use serde_json::json;

use crate::application::atlas_service::AtlasEvents;
use crate::application::confirm_service::ConfirmEvents;
use crate::application::extension_service::ExtensionEvents;
use crate::infrastructure::export::import_service::ImportEvents;
use crate::journal::{ChangedBy, JournalEvents};

/// Carries the messages to the clients.
pub trait EventSink: Send + Sync {
    /// Sends one message, as JSON text. False when no client listens.
    fn send(&self, message: String) -> bool;
}

/// Writes the messages for the clients and gives them to the sink.
#[derive(Clone)]
pub struct ClientMessages {
    sink: Arc<dyn EventSink>,
}

impl ClientMessages {
    pub fn new(sink: Arc<dyn EventSink>) -> ClientMessages {
        ClientMessages { sink }
    }

    /// `source` is `file` for changes picked up from the markdown folder, `server` for an
    /// atlas the journal wrote by itself, `extension` for items that extensions wrote, and
    /// `mcp` for changes that an MCP tool made.
    /// The clients read the definitions again (ADR 046).
    fn definitions_changed_message(&self) {
        self.sink
            .send(json!({ "type": "propertyDefinitionsUpdated" }).to_string());
    }

    fn page_changed_from(&self, page_id: &Hmtid, source: &str) {
        self.sink.send(
            json!({ "type": "pagesUpdated", "pageId": page_id.as_str(), "source": source })
                .to_string(),
        );
    }
}

/// A change made through the journal (ADR 029), by a client or by an MCP tool.
impl JournalEvents for ClientMessages {
    fn page_changed(&self, page_id: &Hmtid, by: &ChangedBy) {
        match by {
            // The other clients read the page again. The client that made the change knows
            // its own ID and does not (ADR-015)
            ChangedBy::Client(client) => {
                self.sink.send(
                    json!({
                        "type": "pagesUpdated",
                        "pageId": page_id.as_str(),
                        "source": "app",
                        "clientId": client.as_str(),
                    })
                    .to_string(),
                );
            }
            ChangedBy::Mcp => self.page_changed_from(page_id, "mcp"),
        }
    }

    fn definitions_changed(&self) {
        self.definitions_changed_message();
    }
}

impl AtlasEvents for ClientMessages {
    fn atlas_changed(&self, atlas_id: &Hmtid) {
        self.page_changed_from(atlas_id, "server");
    }
}

/// A file of the markdown folder changed the page, so a client that shows it reads it again.
impl ImportEvents for ClientMessages {
    fn page_changed(&self, page_id: &Hmtid) {
        self.page_changed_from(page_id, "file");
    }

    fn definitions_changed(&self) {
        self.definitions_changed_message();
    }
}

/// Extensions write items after the answer of a tag add, so a client that shows the page
/// reads it again.
impl ExtensionEvents for ClientMessages {
    /// Its own source: the clients reload the page, as `server` would reload only an atlas.
    fn page_changed(&self, page_id: &Hmtid) {
        self.page_changed_from(page_id, "extension");
    }
}

impl ConfirmEvents for ClientMessages {
    /// Every client gets the question. The first one to answer wins. With no client
    /// listening, no one can answer.
    fn ask_user(&self, request_id: &str, message: &str) -> bool {
        self.sink.send(
            json!({ "type": "confirm", "requestId": request_id, "message": message }).to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use serde_json::Value;

    use super::*;

    /// Keeps the messages. `listening` is what it answers to a message.
    struct Kept {
        messages: Mutex<Vec<Value>>,
        listening: bool,
    }

    fn messages(listening: bool) -> (ClientMessages, Arc<Kept>) {
        let kept = Arc::new(Kept {
            messages: Mutex::new(Vec::new()),
            listening,
        });
        (ClientMessages::new(kept.clone()), kept)
    }

    impl EventSink for Kept {
        fn send(&self, message: String) -> bool {
            self.messages
                .lock()
                .unwrap()
                .push(serde_json::from_str(&message).unwrap());
            self.listening
        }
    }

    impl Kept {
        fn only(&self) -> Value {
            let messages = self.messages.lock().unwrap();
            assert_eq!(messages.len(), 1, "{messages:?}");
            messages[0].clone()
        }
    }

    fn page() -> Hmtid {
        Hmtid::parse("2026-11-11-10-00-00-AAAAAAA").unwrap()
    }

    // クライアントの変更は、そのクライアントのIDと一緒に知らされる。
    #[test]
    fn a_change_of_a_client_is_told_with_the_id_of_the_client() {
        let (messages, kept) = messages(true);
        let client = "123e4567-e89b-42d3-a456-426614174000";

        JournalEvents::page_changed(
            &messages,
            &page(),
            &ChangedBy::Client(client.parse().unwrap()),
        );

        assert_eq!(
            kept.only(),
            json!({
                "type": "pagesUpdated",
                "pageId": "2026-11-11-10-00-00-AAAAAAA",
                "source": "app",
                "clientId": client,
            })
        );
    }

    // クライアント以外の変更は、どこから来たかと一緒に知らされる。
    #[test]
    fn a_change_of_no_client_is_told_with_where_it_came_from() {
        let told = |tell: fn(&ClientMessages)| {
            let (messages, kept) = messages(true);
            tell(&messages);
            kept.only()
        };
        let from = |source: &str| {
            json!({
                "type": "pagesUpdated",
                "pageId": "2026-11-11-10-00-00-AAAAAAA",
                "source": source,
            })
        };

        assert_eq!(
            told(|m| JournalEvents::page_changed(m, &page(), &ChangedBy::Mcp)),
            from("mcp")
        );
        assert_eq!(told(|m| m.atlas_changed(&page())), from("server"));
        assert_eq!(
            told(|m| ImportEvents::page_changed(m, &page())),
            from("file")
        );
        assert_eq!(
            told(|m| ExtensionEvents::page_changed(m, &page())),
            from("extension")
        );
    }

    // 拡張機能の質問は、聞いているクライアントがいるときだけ届く。
    #[test]
    fn a_question_reaches_the_clients_only_when_one_listens() {
        let (listened, kept) = messages(true);
        assert!(listened.ask_user("confirm-1", "Go on?"));
        assert_eq!(
            kept.only(),
            json!({ "type": "confirm", "requestId": "confirm-1", "message": "Go on?" })
        );

        let (unheard, _) = messages(false);
        assert!(!unheard.ask_user("confirm-1", "Go on?"));
    }
}
