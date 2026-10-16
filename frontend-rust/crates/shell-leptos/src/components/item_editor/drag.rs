//! The mouse drag that starts in the editor and, once it leaves the editor, turns into a
//! selection of items.

use crate::core::CoreHandle;
use crate::listeners::DocumentListeners;
use leptos::prelude::*;
use peta_core::Event;

/// A mouse drag that started in the editor; leaving the editor starts a selection.
#[derive(Clone, Copy)]
pub(super) struct Drag {
    /// The editor's box while a drag is on, by which "leaving" is judged.
    rect: StoredValue<Option<web_sys::DomRect>, LocalStorage>,
    skip_selection_check: StoredValue<bool>,
    listeners: DocumentListeners,
}

impl Drag {
    pub(super) fn new() -> Drag {
        Drag {
            rect: StoredValue::new_local(None),
            skip_selection_check: StoredValue::new(false),
            listeners: DocumentListeners::new(),
        }
    }

    /// Starts following the mouse; leaving the editor by more than 10px starts a selection.
    pub(super) fn start(
        self,
        core: CoreHandle,
        el: &web_sys::HtmlTextAreaElement,
        textarea: NodeRef<leptos::html::Textarea>,
        skip_selection_check: bool,
    ) {
        let item_id = StoredValue::new(el.id().trim_start_matches("item-editor-").to_string());
        self.rect.set_value(Some(el.get_bounding_client_rect()));
        self.skip_selection_check.set_value(skip_selection_check);
        self.listeners.clear();
        self.listeners
            .on("mousemove", move |ev: &web_sys::MouseEvent| {
                let Some(rect) = self.rect.try_get_value().flatten() else {
                    return;
                };
                if !self.skip_selection_check.get_value() {
                    if let Some(el) = textarea.get() {
                        if el.selection_start().ok().flatten() == el.selection_end().ok().flatten()
                        {
                            return;
                        }
                    }
                }
                let y = ev.client_y() as f64;
                if y < rect.top() - 10.0 || y > rect.bottom() + 10.0 {
                    core.send(Event::DragBeyondBounds {
                        item_id: item_id.get_value(),
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
