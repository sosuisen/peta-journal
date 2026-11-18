//! Small helpers over `web-sys`: the pieces of the Angular components that read or write
//! the page directly.

use client_core::effects::LocalDateTimeDto;
use client_core::model::DeferredKey;
use client_core::util::utf16;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no window")
}

pub fn document() -> web_sys::Document {
    window().document().expect("no document")
}

pub fn query<T: JsCast>(selector: &str) -> Option<T> {
    document()
        .query_selector(selector)
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<T>().ok())
}

/// The element an event happened on. Not `current_target`: Leptos hands an event on from an
/// element further up.
pub fn target_element(ev: &web_sys::Event) -> Option<web_sys::Element> {
    ev.target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
}

/// The closest ancestor-or-self of the event's target that matches the selector.
pub fn target_closest(ev: &web_sys::Event, selector: &str) -> Option<web_sys::Element> {
    target_element(ev).and_then(|t| t.closest(selector).ok().flatten())
}

pub fn element_by_id(id: &str) -> Option<web_sys::HtmlElement> {
    document()
        .get_element_by_id(id)
        .and_then(|el| el.dyn_into::<web_sys::HtmlElement>().ok())
}

/// Runs a closure after the browser has rendered the current changes.
pub fn after_render(f: impl FnOnce() + 'static) {
    gloo_timers::callback::Timeout::new(0, f).forget();
}

pub fn set_timeout(ms: u32, f: impl FnOnce() + 'static) {
    gloo_timers::callback::Timeout::new(ms, f).forget();
}

pub fn encode_uri(s: &str) -> String {
    js_sys::encode_uri(s)
        .as_string()
        .unwrap_or_else(|| s.to_string())
}

pub fn decode_uri(s: &str) -> String {
    js_sys::decode_uri_component(s)
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| s.to_string())
}

pub fn focus_without_scroll(el: &web_sys::HtmlElement) {
    let options = web_sys::FocusOptions::new();
    options.set_prevent_scroll(true);
    let _ = el.focus_with_options(&options);
}

/// Grows the textarea to its content.
pub fn auto_resize(el: &web_sys::HtmlTextAreaElement) {
    let _ = el.style().set_property("height", "auto");
    let _ = el
        .style()
        .set_property("height", &format!("{}px", el.scroll_height()));
}

/// Returns true when a keydown event belongs to an IME composition session, e.g. the Enter
/// that confirms a Japanese conversion candidate. Chromium reports this via `isComposing`;
/// Safari fires the confirming Enter after `compositionend` with the legacy keyCode 229.
pub fn is_ime_composing(ev: &web_sys::KeyboardEvent) -> bool {
    ev.is_composing() || ev.key_code() == 229
}

pub fn deferred_key(ev: &web_sys::KeyboardEvent) -> DeferredKey {
    DeferredKey {
        key: ev.key(),
        code: ev.code(),
        shift_key: ev.shift_key(),
        ctrl_key: ev.ctrl_key(),
        alt_key: ev.alt_key(),
        meta_key: ev.meta_key(),
    }
}

/// The number of characters before a UTF-16 offset (ADR 043).
pub fn utf16_to_chars(text: &str, utf16_offset: u32) -> usize {
    utf16::chars_of_utf16(text, utf16_offset as usize)
}

/// The UTF-16 offset of a character index.
pub fn chars_to_utf16(text: &str, chars: usize) -> u32 {
    utf16::utf16_of_chars(text, chars) as u32
}

pub fn selection_start_chars(el: &web_sys::HtmlTextAreaElement) -> usize {
    let value = el.value();
    utf16_to_chars(&value, el.selection_start().ok().flatten().unwrap_or(0))
}

pub fn set_cursor_chars(el: &web_sys::HtmlTextAreaElement, chars: usize) {
    let value = el.value();
    let pos = chars_to_utf16(&value, chars.min(value.chars().count()));
    let _ = el.set_selection_range(pos, pos);
}

/// Builds a hidden copy of the textarea's text and measures where a position ends up.
/// Used for the caret rectangle and the first/last line checks.
///
/// The whole text is copied, not only the part before the position. Wrapping depends on the
/// words that follow: a word that did not fit on a line in the textarea would fit in a copy
/// that stops in the middle of it, so a copy of the prefix alone lands on the wrong line.
/// Likewise the position is measured at the character that follows it rather than at a marker
/// appended to the prefix. At a wrap point the browser draws the caret before the first
/// character of the next line, while a narrow marker may still squeeze onto the line above.
fn measure_marker(
    el: &web_sys::HtmlTextAreaElement,
    utf16_position: u32,
    marker_text: &str,
) -> Option<(f64, f64)> {
    let document = document();
    let div: web_sys::HtmlElement = document.create_element("div").ok()?.unchecked_into();
    let style = window().get_computed_style(el).ok()??;
    let dstyle = div.style();
    let _ = dstyle.set_property("position", "absolute");
    let _ = dstyle.set_property("visibility", "hidden");
    let _ = dstyle.set_property("white-space", "pre-wrap");
    let _ = dstyle.set_property("word-wrap", "break-word");
    let _ = dstyle.set_property("box-sizing", "border-box");
    let _ = dstyle.set_property("width", &format!("{}px", el.client_width()));
    for prop in [
        "font",
        "letter-spacing",
        "line-height",
        "padding",
        "word-break",
        "overflow-wrap",
    ] {
        let _ = dstyle.set_property(prop, &style.get_property_value(prop).unwrap_or_default());
    }
    document.body()?.append_child(&div).ok()?;
    let units: Vec<u16> = el.value().encode_utf16().collect();
    let position = (utf16_position as usize).min(units.len());
    let before = units[..position].to_vec().pipe_to_string();
    let after = units[position..].to_vec().pipe_to_string();
    // A newline is not a drawn character: the caret before it sits at the end of the current
    // line, where a marker lands too
    let (marked, rest) = match after.chars().next() {
        Some(c) if c != '\n' => (c.to_string(), after[c.len_utf8()..].to_string()),
        _ => (marker_text.to_string(), after),
    };
    let marker: web_sys::HtmlElement = document.create_element("span").ok()?.unchecked_into();
    marker.set_text_content(Some(&marked));
    let _ = div.append_child(&document.create_text_node(&before));
    let _ = div.append_child(&marker);
    let _ = div.append_child(&document.create_text_node(&rest));
    let result = (marker.offset_left() as f64, marker.offset_top() as f64);
    let _ = document.body()?.remove_child(&div);
    Some(result)
}

trait PipeToString {
    fn pipe_to_string(self) -> String;
}

impl PipeToString for Vec<u16> {
    fn pipe_to_string(self) -> String {
        String::from_utf16_lossy(&self)
    }
}

fn line_height_of(el: &web_sys::HtmlTextAreaElement) -> f64 {
    window()
        .get_computed_style(el)
        .ok()
        .flatten()
        .and_then(|s| s.get_property_value("line-height").ok())
        .and_then(|v| v.trim_end_matches("px").parse::<f64>().ok())
        .unwrap_or(20.0)
}

/// The rectangle of the caret at a position, in viewport coordinates: (left, top, bottom).
pub fn caret_rect(
    el: &web_sys::HtmlTextAreaElement,
    utf16_position: u32,
) -> Option<(f64, f64, f64)> {
    let (left, top) = measure_marker(el, utf16_position, "\u{200B}")?;
    let rect = el.get_bounding_client_rect();
    let line_height = line_height_of(el);
    let x = rect.left() + left - el.scroll_left() as f64;
    let y = rect.top() + top - el.scroll_top() as f64;
    Some((x, y, y + line_height))
}

fn y_at(el: &web_sys::HtmlTextAreaElement, utf16_position: u32) -> f64 {
    measure_marker(el, utf16_position, "|")
        .map(|(_, y)| y)
        .unwrap_or(0.0)
}

pub fn is_on_first_line(el: &web_sys::HtmlTextAreaElement) -> bool {
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    if el.value().is_empty() || start == 0 {
        return true;
    }
    (y_at(el, start) - y_at(el, 0)).abs() < 5.0
}

pub fn is_on_last_line(el: &web_sys::HtmlTextAreaElement) -> bool {
    let value = el.value();
    let len = value.encode_utf16().count() as u32;
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    if value.is_empty() || start == len {
        return true;
    }
    (y_at(el, len) - y_at(el, start)).abs() < 5.0
}

/// The text node and the offset inside it that lie under a point on the screen.
/// Firefox has caretPositionFromPoint; Chrome and Safari have caretRangeFromPoint.
pub fn caret_point_from_point(x: f64, y: f64) -> Option<(web_sys::Node, u32)> {
    let document = document();
    if let Some(pos) = document.caret_position_from_point(x as f32, y as f32) {
        return Some((pos.offset_node()?, pos.offset()));
    }
    // Chrome and Safari: caretRangeFromPoint, which web-sys does not bind
    let f = js_sys::Reflect::get(&document, &"caretRangeFromPoint".into()).ok()?;
    let f: js_sys::Function = f.dyn_into().ok()?;
    let range = f.call2(&document, &x.into(), &y.into()).ok()?;
    let range: web_sys::Range = range.dyn_into().ok()?;
    Some((range.start_container().ok()?, range.start_offset().ok()?))
}

/// The character offset in a textarea's value under a point on the screen. A point outside
/// the textarea is clamped to its nearest edge, so a drag that strays sideways still selects.
pub fn textarea_offset_at_point(el: &web_sys::HtmlTextAreaElement, x: f64, y: f64) -> usize {
    let rect = el.get_bounding_client_rect();
    let x = x.clamp(rect.left() + 1.0, rect.right() - 1.0);
    let y = y.clamp(rect.top() + 1.0, rect.bottom() - 1.0);
    let value = el.value();
    // Firefox and Chrome 128+ answer for a text control with the control itself and an offset
    // into its value
    if let Some(pos) = document().caret_position_from_point(x as f32, y as f32) {
        if pos.offset_node().is_some_and(|n| n.is_same_node(Some(el))) {
            return utf16_to_chars(&value, pos.offset());
        }
    }
    // Elsewhere a mirror of the text is laid over the textarea and asked instead: a textarea's
    // own text is out of reach of caretRangeFromPoint
    mirror_offset_at_point(el, &value, &rect, x, y).unwrap_or(0)
}

fn mirror_offset_at_point(
    el: &web_sys::HtmlTextAreaElement,
    value: &str,
    rect: &web_sys::DomRect,
    x: f64,
    y: f64,
) -> Option<usize> {
    let document = document();
    let div: web_sys::HtmlElement = document.create_element("div").ok()?.unchecked_into();
    let style = window().get_computed_style(el).ok()??;
    let dstyle = div.style();
    let _ = dstyle.set_property("position", "fixed");
    let _ = dstyle.set_property("left", &format!("{}px", rect.left()));
    let _ = dstyle.set_property("top", &format!("{}px", rect.top() - el.scroll_top() as f64));
    let _ = dstyle.set_property("width", &format!("{}px", el.client_width()));
    // Invisible but still hit-tested, unlike `visibility: hidden`
    let _ = dstyle.set_property("opacity", "0");
    let _ = dstyle.set_property("z-index", "2147483647");
    let _ = dstyle.set_property("white-space", "pre-wrap");
    let _ = dstyle.set_property("word-wrap", "break-word");
    for prop in [
        "font",
        "letter-spacing",
        "line-height",
        "padding",
        "box-sizing",
    ] {
        let _ = dstyle.set_property(prop, &style.get_property_value(prop).unwrap_or_default());
    }
    let text = document.create_text_node(value);
    let _ = div.append_child(&text);
    document.body()?.append_child(&div).ok()?;
    let hit = caret_point_from_point(x, y);
    let _ = document.body()?.remove_child(&div);
    let (node, offset) = hit?;
    if !node.is_same_node(Some(&text)) {
        return None;
    }
    Some(utf16_to_chars(value, offset))
}

/// The letters of an element's text before a point, across all its text nodes. `None` when
/// the point is on no text of the element, such as beside its letters.
pub fn text_before_point(container: &web_sys::Element, x: f64, y: f64) -> Option<String> {
    let (node, offset) = caret_point_from_point(x, y)?;
    if node.node_type() != web_sys::Node::TEXT_NODE || !container.contains(Some(&node)) {
        return None;
    }
    let walker = document()
        .create_tree_walker_with_what_to_show(container, 4 /* SHOW_TEXT */)
        .ok()?;
    let mut before = String::new();
    while let Ok(Some(current)) = walker.next_node() {
        let text = current.text_content().unwrap_or_default();
        if current.is_same_node(Some(&node)) {
            before.extend(text.chars().take(utf16_to_chars(&text, offset)));
            return Some(before);
        }
        before.push_str(&text);
    }
    None
}

/// A blob of `content` with the MIME type `mime`.
pub fn text_blob(content: &str, mime: &str) -> Result<web_sys::Blob, JsValue> {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(content));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
}

/// Starts a clipboard write of a `text/plain` and a `text/html` part, each given as a blob
/// or as a promise of one, and returns the promise of the write.
pub fn write_clipboard_parts(text: &JsValue, html: &JsValue) -> Result<js_sys::Promise, JsValue> {
    let record = js_sys::Object::new();
    js_sys::Reflect::set(&record, &"text/plain".into(), text)?;
    js_sys::Reflect::set(&record, &"text/html".into(), html)?;
    let item =
        web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&record.unchecked_into())?;
    let items = js_sys::Array::new();
    items.push(&item);
    Ok(window().navigator().clipboard().write(&items))
}

/// Writes text and HTML to the clipboard with `ClipboardItem`.
pub async fn write_clipboard_rich(text: &str, html: &str) -> Result<(), JsValue> {
    let written = write_clipboard_parts(
        &text_blob(text, "text/plain")?.into(),
        &text_blob(html, "text/html")?.into(),
    )?;
    JsFuture::from(written).await?;
    Ok(())
}

/// `HH:MM` of an ISO timestamp in the local time zone.
pub fn local_time(iso: &str) -> String {
    let d = js_sys::Date::new(&JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return iso.to_string();
    }
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

pub fn epoch_ms(iso: &str) -> f64 {
    js_sys::Date::new(&JsValue::from_str(iso)).get_time()
}

/// A date-time as a `datetime-local` input shows it: local time to the minute,
/// `YYYY-MM-DDTHH:MM`. Empty when the text is not a date-time.
pub fn local_minutes(iso: &str) -> String {
    let d = js_sys::Date::new(&JsValue::from_str(iso));
    if d.get_time().is_nan() {
        return String::new();
    }
    local_minutes_of(&d)
}

/// The current local time to the minute, `YYYY-MM-DDTHH:MM`.
pub fn now_local_minutes() -> String {
    local_minutes_of(&js_sys::Date::new_0())
}

fn local_minutes_of(d: &js_sys::Date) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}",
        d.get_full_year(),
        d.get_month() + 1,
        d.get_date(),
        d.get_hours(),
        d.get_minutes()
    )
}

/// A `datetime-local` value (local time, `YYYY-MM-DDTHH:MM`) as the server stores a
/// date-time: UTC to the second, `YYYY-MM-DDTHH:MM:SSZ`. Empty when the text is not a
/// date-time. A date-time without an offset is read as local time, as the HTML form does.
pub fn local_minutes_to_utc(local: &str) -> String {
    let d = js_sys::Date::new(&JsValue::from_str(local));
    if d.get_time().is_nan() {
        return String::new();
    }
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        d.get_utc_full_year(),
        d.get_utc_month() + 1,
        d.get_utc_date(),
        d.get_utc_hours(),
        d.get_utc_minutes(),
        d.get_utc_seconds()
    )
}

/// The local date and time of the browser, as the core takes it.
pub fn local_now() -> LocalDateTimeDto {
    let d = js_sys::Date::new_0();
    LocalDateTimeDto {
        year: d.get_full_year() as i32,
        month: d.get_month() + 1,
        day: d.get_date(),
        hour: d.get_hours(),
        minute: d.get_minutes(),
    }
}

pub fn local_storage_get(key: &str) -> Option<String> {
    window()
        .local_storage()
        .ok()
        .flatten()
        .and_then(|s| s.get_item(key).ok().flatten())
}

pub fn local_storage_set(key: &str, value: &str) {
    if let Some(s) = window().local_storage().ok().flatten() {
        let _ = s.set_item(key, value);
    }
}

/// A value stored as JSON; None when there is none or it does not read as `T`.
pub fn local_storage_json<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    local_storage_get(key).and_then(|s| serde_json::from_str(&s).ok())
}

/// Stores a value as JSON.
pub fn local_storage_set_json<T: serde::Serialize + ?Sized>(key: &str, value: &T) {
    local_storage_set(key, &serde_json::to_string(value).unwrap_or_default());
}

/// The first image file in a paste or a drop, if there is one.
pub fn image_file(data: &web_sys::DataTransfer) -> Option<web_sys::File> {
    let items = data.items();
    for i in 0..items.length() {
        if let Some(item) = items.get(i) {
            if item.type_().starts_with("image/") {
                if let Ok(Some(file)) = item.get_as_file() {
                    return Some(file);
                }
            }
        }
    }
    None
}
