//! Server-sent events: carries the messages of the journal core to the connected clients.
//! The core writes the messages (`journal_core::events`).

use std::convert::Infallible;

use axum::response::sse::{Event, KeepAlive, Sse};
use futures::stream::Stream;
use tokio::sync::broadcast;
use tokio_stream::wrappers::errors::BroadcastStreamRecvError;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use journal_core::events::EventSink;

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
}

impl EventSink for SseBroadcaster {
    /// Every connected client gets the message.
    fn send(&self, message: String) -> bool {
        // An error means that no client listens right now
        self.tx.send(message).is_ok()
    }
}
