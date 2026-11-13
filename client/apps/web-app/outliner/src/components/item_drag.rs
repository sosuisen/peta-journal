//! The mouse drag that starts on an item's body: inside the body it selects text, and once it
//! leaves that body up or down it turns into a selection of items.

use crate::core::CoreHandle;
use crate::dom;
use crate::listeners::DocumentListeners;
use client_core::Event;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

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
    ///
    /// `select_from` is the character the mouse went down on when the drag starts on the
    /// rendered view. The editor that replaces the view never saw that mousedown, so the
    /// browser will not select its text by itself; the drag selects from that character to the
    /// one under the mouse instead. A drag that starts on the editor passes `None` and leaves
    /// the text selection to the browser.
    pub fn start(
        self,
        core: CoreHandle,
        item_id: &str,
        body: &web_sys::Element,
        select_from: Option<usize>,
    ) {
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
                    return;
                }
                if let Some(anchor) = select_from {
                    select_text(&item_id, anchor, ev.client_x() as f64, y);
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

/// Selects the editor's text between `anchor` and the character under the mouse. The caret
/// end of the selection follows the mouse, as in a browser's own drag.
fn select_text(item_id: &str, anchor: usize, x: f64, y: f64) {
    let Some(el) = dom::element_by_id(&format!("item-editor-{item_id}"))
        .and_then(|el| el.dyn_into::<web_sys::HtmlTextAreaElement>().ok())
    else {
        return;
    };
    let focus = dom::textarea_offset_at_point(&el, x, y);
    let value = el.value();
    let (start, end, direction) = if focus < anchor {
        (focus, anchor, "backward")
    } else {
        (anchor, focus, "forward")
    };
    let _ = el.set_selection_range_with_direction(
        dom::chars_to_utf16(&value, start),
        dom::chars_to_utf16(&value, end),
        direction,
    );
}
