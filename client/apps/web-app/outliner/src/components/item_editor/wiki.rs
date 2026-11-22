//! The `[[wiki link]]` completion: what is typed inside the brackets and where the popup goes.

use crate::core::CoreHandle;
use crate::dom;
use client_core::app::WikiLinkContext;
use client_core::ops::wiki::query_at;
use client_core::util::utf16;
use client_core::Event;

/// Reports the `[[` the caret is inside of, with the caret's place on the screen, or closes
/// the popup.
pub(super) fn check_wiki_link_context(
    core: &CoreHandle,
    item_id: &str,
    el: &web_sys::HtmlTextAreaElement,
) {
    let value = el.value();
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    let cursor = utf16::chars_of_utf16(&value, start as usize);
    let context = query_at(&value, cursor).and_then(|query| {
        let (x, _top, bottom) = dom::caret_rect(el, start)?;
        Some(WikiLinkContext {
            query,
            x,
            y: bottom,
        })
    });
    core.send(Event::WikiQuery {
        item_id: item_id.to_string(),
        context,
    });
}
