//! Server-sent events: tells connected clients which pages changed.

use std::convert::Infallible;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::application::atlas_service::AtlasEvents;
use crate::application::extension_service::ExtensionEvents;

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

    pub fn broadcast_pages_updated(&self, page_id: &str) {
        self.broadcast_pages_updated_from(page_id, "app");
    }

    /// `source` is `app` for changes made through the API, `file` for changes picked up
    /// from the markdown folder, and `server` for changes the server made by itself.
    pub fn broadcast_pages_updated_from(&self, page_id: &str, source: &str) {
        let json = format!(
            "{{\"type\":\"pagesUpdated\",\"pageId\":\"{page_id}\",\"source\":\"{source}\"}}"
        );
        // An error only means that nobody is listening right now
        let _ = self.tx.send(json);
    }
}

/// The server writes an atlas page by itself, so a client that shows it reads it again.
impl ExtensionEvents for SseBroadcaster {
    /// Its own source: the clients reload the page, as `server` would reload only an atlas.
    fn page_changed(&self, page_id: &hmtid::Hmtid) {
        self.broadcast_pages_updated_from(page_id.as_str(), "extension");
    }
}

impl AtlasEvents for SseBroadcaster {
    fn atlas_changed(&self, atlas_id: &hmtid::Hmtid) {
        self.broadcast_pages_updated_from(atlas_id.as_str(), "server");
    }
}
