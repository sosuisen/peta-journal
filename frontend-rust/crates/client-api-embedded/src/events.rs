//! The messages of the journal for the frontend core, where `client-api-http` reads the
//! event stream of the journal server.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;

use journal_core::events::EventSink;
use peta_core::effects::SseEvent;

/// Carries the messages of the journal to the frontend core. The journal gets it
/// as its `EventSink`, and the shell reads it with [`EventQueue::read`].
pub struct EventQueue {
    to_reader: Sender<Option<String>>,
    messages: Mutex<Receiver<Option<String>>>,
    /// Whether the frontend core reads now
    listening: AtomicBool,
}

impl Default for EventQueue {
    fn default() -> Self {
        let (to_reader, messages) = channel();
        EventQueue {
            to_reader,
            messages: Mutex::new(messages),
            listening: AtomicBool::new(false),
        }
    }
}

impl EventQueue {
    /// Gives the frontend core the messages as they come, as an event stream that is open.
    /// It returns after [`EventQueue::close`], with the error that ends a stream.
    pub fn read(&self, send: &mut dyn FnMut(SseEvent)) {
        // One reader at a time: a second one waits here for the first to end
        let messages = self.messages.lock().unwrap();
        self.listening.store(true, Ordering::SeqCst);
        send(SseEvent::Open);
        while let Ok(Some(message)) = messages.recv() {
            send(SseEvent::Message(message));
        }
        self.listening.store(false, Ordering::SeqCst);
        send(SseEvent::Error);
    }

    /// Ends the reading, as when the app goes away.
    pub fn close(&self) {
        // The receiver lives as long as the queue, so the message is taken
        let _ = self.to_reader.send(None);
    }
}

impl EventSink for EventQueue {
    /// A message sent while no one reads is dropped, as the journal server drops a message
    /// when no client is connected. A frontend that starts to read loads its pages anew.
    fn send(&self, message: String) -> bool {
        let listening = self.listening.load(Ordering::SeqCst);
        if listening {
            let _ = self.to_reader.send(Some(message));
        }
        listening
    }
}
