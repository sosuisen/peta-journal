//! Server-sent events: tells connected clients which pages changed.

use std::convert::Infallible;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::application::atlas_service::AtlasEvents;
use crate::application::confirm_service::ConfirmEvents;
use crate::application::extension_service::ExtensionEvents;
use crate::presentation::client_id::ClientId;

#[derive(Debug, Clone)]
pub struct SseBroadcaster {
    tx: broadcast::Sender<String>,
}

impl Default for SseBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl SseBroadcaster {
    pub fn new() -> SseBroadcaster {
        let (tx, _rx) = broadcast::channel(256);
        SseBroadcaster { tx }
    }

    /// The event stream for one subscriber: an initial `connected` event, then every broadcast.
    pub fn event_stream(&self) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
        let rx = self.tx.subscribe();
        let first =
            futures::stream::once(async { Ok(Event::default().data("{\"type\":\"connected\"}")) });
        let rest = BroadcastStream::new(rx).filter_map(|msg| match msg {
            Ok(json) => Some(Ok(Event::default().data(json))),
            Err(BroadcastStreamRecvError::Lagged(_)) => None,
        });
        Sse::new(first.chain(rest)).keep_alive(KeepAlive::default())
    }

    /// Tells of a change that a client made through the API. The other clients read the page
    /// again. The client that made the change knows its own ID and does not (ADR-015).
    pub fn broadcast_pages_updated(&self, client: &ClientId, page_id: &str) {
        let json = serde_json::json!({
            "type": "pagesUpdated",
            "pageId": page_id,
            "source": "app",
            "clientId": client.as_str(),
        });
        // An error only means that nobody is listening right now
        let _ = self.tx.send(json.to_string());
    }

    /// `source` is `file` for changes picked up from the markdown folder, `server` for an atlas the server wrote by itself, and
    /// `extension` for items that extensions wrote.
    pub fn broadcast_pages_updated_from(&self, page_id: &str, source: &str) {
        let json = format!(
            "{{\"type\":\"pagesUpdated\",\"pageId\":\"{page_id}\",\"source\":\"{source}\"}}"
        );
        // An error only means that nobody is listening right now
        let _ = self.tx.send(json);
    }
}

/// Extensions write items after the answer of a tag add, so a client that shows the page reads
/// it again.
impl ExtensionEvents for SseBroadcaster {
    /// Its own source: the clients reload the page, as `server` would reload only an atlas.
    fn page_changed(&self, page_id: &hmtid::Hmtid) {
        self.broadcast_pages_updated_from(page_id.as_str(), "extension");
    }
}

impl ConfirmEvents for SseBroadcaster {
    /// Every client gets the question. The first one to answer wins.
    fn ask_user(&self, request_id: &str, message: &str) -> bool {
        let json = serde_json::json!({
            "type": "confirm",
            "requestId": request_id,
            "message": message,
        });
        // An error means that no client listens, so no one can answer
        self.tx.send(json.to_string()).is_ok()
    }
}

impl AtlasEvents for SseBroadcaster {
    fn atlas_changed(&self, atlas_id: &hmtid::Hmtid) {
        self.broadcast_pages_updated_from(atlas_id.as_str(), "server");
    }
}
