//! The mouse drag that starts on an item's body and, once it leaves that body up or down,
//! turns into a selection of items.

use crate::core::CoreHandle;
use crate::listeners::DocumentListeners;
use leptos::prelude::*;
use peta_core::Event;

/// A mouse drag that started on an item's body; leaving the body starts a selection.
///
/// One per outline row, shared by the rendered view and the editor through the context. The
/// view starts it on the very mousedown that opens the editor: waiting for the editor to mount
/// would miss the mouse moves of a quick drag.
#[derive(Clone, Copy)]
pub struct Drag {
    /// The body's box while a drag is on, by which "leaving" is judged.
    rect: StoredValue<Option<web_sys::DomRect>, LocalStorage>,
    listeners: DocumentListeners,
}

impl Drag {
    pub fn new() -> Drag {
        Drag {
            rect: StoredValue::new_local(None),
            listeners: DocumentListeners::new(),
        }
    }

    /// Starts following the mouse; leaving `body` by more than 10px up or down starts a
    /// selection of `item_id`. Whether the editor's text is selected does not matter: a drag
    /// from a bare caret that leaves the body is as clear a request as one from a selection.
    pub fn start(self, core: CoreHandle, item_id: &str, body: &web_sys::Element) {
        // A plain String, not a StoredValue: a value stored here would belong to the view that
        // called us, and the view is gone (replaced by the editor) by the time the mouse moves
        let item_id = item_id.to_string();
        self.rect.set_value(Some(body.get_bounding_client_rect()));
        self.listeners.clear();
        self.listeners
            .on("mousemove", move |ev: &web_sys::MouseEvent| {
                let Some(rect) = self.rect.try_get_value().flatten() else {
                    return;
                };
                let y = ev.client_y() as f64;
                if y < rect.top() - 10.0 || y > rect.bottom() + 10.0 {
                    core.send(Event::DragBeyondBounds {
                        item_id: item_id.clone(),
                    });
                    self.end();
                }
            });
        self.listeners
            .on("mouseup", move |_: &web_sys::Event| self.end());
    }

    fn end(self) {
        self.rect.try_set_value(None);
        self.listeners.clear_later();
    }
}

impl Default for Drag {
    fn default() -> Self {
        Self::new()
    }
}
