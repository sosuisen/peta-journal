//! Event listeners on the document that belong to a component or to a gesture: the keys of
//! selection mode, the mouse while a drag or a sidebar resize is in progress, the click that
//! closes a popup menu.

use gloo_events::{EventListener, EventListenerOptions, EventListenerPhase};
use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// A set of document listeners. `Copy`, so any closure can add to it or clear it. The
/// listeners are removed from the document when cleared and when the component that made
/// the set goes away.
#[derive(Clone, Copy)]
pub struct DocumentListeners(StoredValue<Vec<EventListener>, LocalStorage>);

impl Default for DocumentListeners {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentListeners {
    pub fn new() -> Self {
        Self(StoredValue::new_local(Vec::new()))
    }

    /// Listens for `event` on the document. The handler gets the event as `E`, e.g.
    /// `web_sys::KeyboardEvent`; `web_sys::Event` fits any event.
    pub fn on<E: JsCast>(self, event: &'static str, handler: impl FnMut(&E) + 'static) {
        self.add(event, EventListenerOptions::default(), handler);
    }

    /// Like [`on`](Self::on), for a handler that may call `prevent_default`.
    pub fn on_cancelable<E: JsCast>(self, event: &'static str, handler: impl FnMut(&E) + 'static) {
        self.add(
            event,
            EventListenerOptions::enable_prevent_default(),
            handler,
        );
    }

    /// Like [`on_cancelable`](Self::on_cancelable), run in the capture phase, before the
    /// listeners of the target.
    pub fn on_capture<E: JsCast>(self, event: &'static str, handler: impl FnMut(&E) + 'static) {
        let options = EventListenerOptions {
            phase: EventListenerPhase::Capture,
            passive: false,
        };
        self.add(event, options, handler);
    }

    fn add<E: JsCast>(
        self,
        event: &'static str,
        options: EventListenerOptions,
        mut handler: impl FnMut(&E) + 'static,
    ) {
        let document = crate::dom::document();
        let listener = EventListener::new_with_options(&document, event, options, move |ev| {
            handler(ev.unchecked_ref())
        });
        // A set whose component is gone drops the listener right here
        self.0.try_update_value(|l| l.push(listener));
    }

    /// Closes a popup menu on a mousedown outside it or on Escape. Listening starts after
    /// the current render, so the click that opened the menu does not close it.
    pub fn close_menu_on_outside_or_escape(
        self,
        menu_selector: &'static str,
        close: impl Fn() + Copy + 'static,
    ) {
        crate::dom::after_render(move || {
            self.on_cancelable("mousedown", move |ev: &web_sys::MouseEvent| {
                if crate::dom::target_closest(ev, menu_selector).is_none() {
                    close();
                }
            });
            self.on("keydown", move |ev: &web_sys::KeyboardEvent| {
                if ev.key() == "Escape" {
                    ev.prevent_default();
                    close();
                }
            });
        });
    }

    /// Removes the listeners now. Not from inside one of them: use [`clear_later`](Self::clear_later).
    pub fn clear(self) {
        self.0.try_update_value(|l| l.clear());
    }

    /// Removes the listeners once the current event is over; the browser does not let a
    /// listener remove itself while it runs.
    pub fn clear_later(self) {
        crate::dom::after_render(move || self.clear());
    }
}
