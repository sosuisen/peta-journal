//! Helpers over the textarea's text: insertions that report the new body, the code fence
//! that closes itself.

use super::wiki::check_wiki_link_context;
use crate::core::CoreHandle;
use crate::dom;
use peta_core::Event;

/// Typing the third backtick of an opening fence adds the closing fence below.
pub(super) fn auto_close_code_fence(el: &web_sys::HtmlTextAreaElement) {
    let value = el.value();
    let start = el.selection_start().ok().flatten().unwrap_or(0) as usize;
    let before: String = value.encode_utf16().take(start).collect::<Vec<_>>().pipe();
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    if !before[line_start..].trim_start().starts_with("```") {
        return;
    }
    if peta_core::util::has_unclosed_code_fence(&value) {
        el.set_value(&format!("{value}\n```"));
        let _ = el.set_selection_range(start as u32, start as u32);
    }
}

pub(super) trait Utf16Pipe {
    fn pipe(self) -> String;
}
impl Utf16Pipe for Vec<u16> {
    fn pipe(self) -> String {
        String::from_utf16_lossy(&self)
    }
}

/// Replaces the selection (or a range) and reports the new body.
pub(super) fn replace_range(
    core: &CoreHandle,
    item_id: &str,
    el: &web_sys::HtmlTextAreaElement,
    text: &str,
    start: u32,
    end: u32,
) {
    let _ = el.set_range_text_with_start_and_end_and_mode(text, start, end, "end");
    core.send(Event::BodyChanged {
        item_id: item_id.to_string(),
        body: el.value(),
        now_ms: core.now_ms(),
    });
    dom::auto_resize(el);
    check_wiki_link_context(core, item_id, el);
}

pub(super) fn insert_at_cursor(
    core: &CoreHandle,
    item_id: &str,
    el: &web_sys::HtmlTextAreaElement,
    text: &str,
    cursor_offset: u32,
) {
    let value = el.value();
    let (start, end) = (
        el.selection_start().ok().flatten().unwrap_or(0),
        el.selection_end().ok().flatten().unwrap_or(0),
    );
    let units: Vec<u16> = value.encode_utf16().collect();
    let before = String::from_utf16_lossy(&units[..start as usize]);
    let after = String::from_utf16_lossy(&units[end as usize..]);
    el.set_value(&format!("{before}{text}{after}"));
    let pos = start + cursor_offset;
    let _ = el.set_selection_range(pos, pos);
    core.send(Event::BodyChanged {
        item_id: item_id.to_string(),
        body: el.value(),
        now_ms: core.now_ms(),
    });
    dom::auto_resize(el);
}
