//! The `[[wiki link]]` completion: what is typed inside the brackets and where the popup goes.

use crate::core::CoreHandle;
use crate::dom;
use peta_core::app::WikiLinkContext;
use peta_core::Event;

/// The text of the `[[wiki link]]` the caret is inside of.
pub(super) fn wiki_link_at_caret(value: &str, start: u32) -> Option<String> {
    let units: Vec<u16> = value.encode_utf16().collect();
    let before = String::from_utf16_lossy(&units[..(start as usize).min(units.len())]);
    let after = String::from_utf16_lossy(&units[(start as usize).min(units.len())..]);
    let open = before.rfind("[[")?;
    let between = &before[open + 2..];
    if between.contains("]]") {
        return None;
    }
    let close = after.find("]]")?;
    let text = format!("{between}{}", &after[..close]).trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// Reports the `[[` the caret is inside of, with the caret's place on the screen, or closes
/// the popup.
pub(super) fn check_wiki_link_context(
    core: &CoreHandle,
    item_id: &str,
    el: &web_sys::HtmlTextAreaElement,
) {
    let value = el.value();
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    let units: Vec<u16> = value.encode_utf16().collect();
    let before = String::from_utf16_lossy(&units[..(start as usize).min(units.len())]);
    let after = String::from_utf16_lossy(&units[(start as usize).min(units.len())..]);
    let close = |core: &CoreHandle| {
        core.send(Event::WikiQuery {
            item_id: item_id.to_string(),
            context: None,
        })
    };
    let Some(open) = before.rfind("[[") else {
        return close(core);
    };
    let between = &before[open + 2..];
    if between.contains("]]") || !after.contains("]]") {
        return close(core);
    }
    let Some((x, _top, bottom)) = dom::caret_rect(el, start) else {
        return close(core);
    };
    core.send(Event::WikiQuery {
        item_id: item_id.to_string(),
        context: Some(WikiLinkContext {
            query: between.to_string(),
            x,
            y: bottom,
        }),
    });
}
