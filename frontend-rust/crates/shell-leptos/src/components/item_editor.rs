//! The textarea an item is edited in, ported from `item-editor.ts`: keyboard shortcuts,
//! paste, the `[[` completion context, and the drag out of the editor that starts a selection.

use crate::core::CoreHandle;
use crate::dom;
use gloo_events::EventListener;
use leptos::prelude::*;
use peta_core::app::WikiLinkContext;
use peta_core::model::DeferredKey;
use peta_core::view::Row;
use peta_core::Event;
use send_wrapper::SendWrapper;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::JsCast;

/// Tracks a mouse drag that started in the editor; leaving the editor starts a selection.
#[derive(Default)]
struct Drag {
    rect: Option<web_sys::DomRect>,
    skip_selection_check: bool,
    listeners: Vec<EventListener>,
}

#[component]
pub fn ItemEditor(row: RwSignal<Row>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let item_id = row.with_untracked(|r| r.item.id.clone());
    let textarea = NodeRef::<leptos::html::Textarea>::new();
    let drag: SendWrapper<Rc<RefCell<Drag>>> = SendWrapper::new(Rc::new(RefCell::new(Drag::default())));

    let id = item_id.clone();
    let is_task = move || row.with(|r| r.item.is_task());
    let has_end = move || row.with(|r| r.item.prop_str("end").is_some());
    let indeterminate = move || row.with(|r| r.item.prop_str("begin").is_some() && r.item.prop_str("end").is_none());
    let heading_font_size = move || {
        row.with(|r| {
            if r.item.body.contains('\n') {
                return None;
            }
            match r.item.heading_level_of_body() {
                1 => Some("1.6em"),
                2 => Some("1.4em"),
                3 => Some("1.2em"),
                4 => Some("1.1em"),
                5 => Some("1em"),
                _ => None,
            }
        })
    };
    let is_heading_only = move || row.with(|r| !r.item.body.contains('\n') && r.item.heading_level_of_body() > 0);

    // The body as the core has it: paste, undo and the wiki completion change it from outside
    {
        Effect::new(move |_| {
            let body = row.with(|r| r.item.body.clone());
            if let Some(el) = textarea.get() {
                if el.value() != body {
                    el.set_value(&body);
                    dom::auto_resize(&el);
                }
            }
        });
    }

    // Focus and the cursor the core asked for, on mount and whenever a new request comes
    {
        let consumed = StoredValue::new(0u64);
        let item_id = item_id.clone();
                let drag = drag.clone();
        Effect::new(move |_| {
            let request = vm.with(|v| v.outline.focus_request.clone());
            let Some(el) = textarea.get() else { return };
            let mine = request.filter(|r| r.item_id == item_id && r.seq != consumed.get_value());
            match mine {
                Some(req) => {
                    consumed.set_value(req.seq);
                    if let Some(cursor) = req.cursor {
                        dom::set_cursor_chars(&el, cursor);
                    }
                    dom::focus_without_scroll(&el);
                    dom::auto_resize(&el);
                    if req.mouse_drag {
                        start_drag(core, &drag, &el, &textarea, true);
                    }
                }
                None => {
                    // First render without a request of our own: the editor still takes the focus
                    if consumed.get_value() == 0 {
                        consumed.set_value(u64::MAX);
                        dom::focus_without_scroll(&el);
                        dom::auto_resize(&el);
                    }
                }
            }
        });
    }

    // Keys held while this editor was not on screen yet are played here
    {
                let id = item_id.clone();
        core.register_replayer(
            &item_id,
            Box::new(move |keys: Vec<DeferredKey>| {
                if let Some(el) = textarea.get() {
                    replay_keys(&core, &el, keys);
                }
            }),
        );
                on_cleanup(move || {
            core.unregister_replayer(&id);
        });
    }
    {
        let drag = drag.clone();
        on_cleanup(move || drag.borrow_mut().listeners.clear());
    }

    let on_checkbox = {
                let id = item_id.clone();
        move |ev: web_sys::MouseEvent| {
            ev.prevent_default();
            ev.stop_propagation();
            core.send(Event::TaskCycle { item_id: id.clone() });
        }
    };

    let on_input = {
                let id = item_id.clone();
        move |ev: web_sys::Event| {
            let Some(el) = textarea.get() else { return };
            if let Some(input) = ev.dyn_ref::<web_sys::InputEvent>() {
                if input.input_type() == "insertText" && input.data().as_deref() == Some("`") {
                    auto_close_code_fence(&el);
                }
            }
            let body = el.value();
            core.send(Event::BodyChanged { item_id: id.clone(), body, now_ms: core.now_ms() });
            dom::auto_resize(&el);
            check_wiki_link_context(&core, &id, &el);
        }
    };

    let on_keydown = {
                let id = item_id.clone();
        move |ev: web_sys::KeyboardEvent| {
            let Some(el) = textarea.get() else { return };
            handle_keydown(&core, &id, &el, &ev, row);
        }
    };

    let on_paste = {
                let id = item_id.clone();
        move |ev: web_sys::ClipboardEvent| {
            let Some(el) = textarea.get() else { return };
            let Some(data) = ev.clipboard_data() else { return };
            let cursor = dom::selection_start_chars(&el);
            if let Some(file) = image_file(&data) {
                ev.prevent_default();
                let handle = core.keep_file(file);
                core.send(Event::PasteImage { item_id: id.clone(), file: handle, cursor });
                return;
            }
            // Windows puts \r\n on the clipboard; the textarea would normalize it anyway
            let text = data.get_data("text/plain").unwrap_or_default().replace("\r\n", "\n").replace('\r', "\n");
            let html = data.get_data("text/html").unwrap_or_default();
            ev.prevent_default();
            core.send(Event::PasteText { item_id: id.clone(), text, html, cursor });
        }
    };

    let on_blur = {
                let id = item_id.clone();
        move |_| core.send(Event::EditFinished { item_id: id.clone() })
    };

    let on_mousedown = {
                let drag = drag.clone();
        move |_: web_sys::MouseEvent| {
            if let Some(el) = textarea.get() {
                start_drag(core, &drag, &el, &textarea, false);
            }
        }
    };

    view! {
        <app-item-editor>
            <Show when=is_task>
                <input
                    type="checkbox"
                    class="task-checkbox"
                    prop:checked=has_end
                    prop:indeterminate=indeterminate
                    on:click=on_checkbox.clone()
                />
            </Show>
            <textarea
                node_ref=textarea
                id=format!("item-editor-{id}")
                style:font-size=move || heading_font_size().unwrap_or("")
                style:font-weight=move || if is_heading_only() { "bold" } else { "" }
                rows="1"
                on:input=on_input
                on:keydown=on_keydown
                on:paste=on_paste
                on:blur=on_blur
                on:mousedown=on_mousedown
            ></textarea>
        </app-item-editor>
    }
}

fn image_file(data: &web_sys::DataTransfer) -> Option<web_sys::File> {
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

/// Typing the third backtick of an opening fence adds the closing fence below.
fn auto_close_code_fence(el: &web_sys::HtmlTextAreaElement) {
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

trait Utf16Pipe {
    fn pipe(self) -> String;
}
impl Utf16Pipe for Vec<u16> {
    fn pipe(self) -> String {
        String::from_utf16_lossy(&self)
    }
}

/// Replaces the selection (or a range) and reports the new body.
fn replace_range(core: &CoreHandle, item_id: &str, el: &web_sys::HtmlTextAreaElement, text: &str, start: u32, end: u32) {
    let _ = el.set_range_text_with_start_and_end_and_mode(text, start, end, "end");
    core.send(Event::BodyChanged { item_id: item_id.to_string(), body: el.value(), now_ms: core.now_ms() });
    dom::auto_resize(el);
    check_wiki_link_context(core, item_id, el);
}

fn insert_at_cursor(core: &CoreHandle, item_id: &str, el: &web_sys::HtmlTextAreaElement, text: &str, cursor_offset: u32) {
    let value = el.value();
    let (start, end) =
        (el.selection_start().ok().flatten().unwrap_or(0), el.selection_end().ok().flatten().unwrap_or(0));
    let units: Vec<u16> = value.encode_utf16().collect();
    let before = String::from_utf16_lossy(&units[..start as usize]);
    let after = String::from_utf16_lossy(&units[end as usize..]);
    el.set_value(&format!("{before}{text}{after}"));
    let pos = start + cursor_offset;
    let _ = el.set_selection_range(pos, pos);
    core.send(Event::BodyChanged { item_id: item_id.to_string(), body: el.value(), now_ms: core.now_ms() });
    dom::auto_resize(el);
}

/// The keyboard shortcuts of the editor, in the order `onKeydown` checked them.
fn handle_keydown(
    core: &CoreHandle,
    item_id: &str,
    el: &web_sys::HtmlTextAreaElement,
    ev: &web_sys::KeyboardEvent,
    row: RwSignal<Row>,
) {
    // Let the IME handle its own keys (e.g. the Enter that confirms a conversion candidate)
    if dom::is_ime_composing(ev) {
        return;
    }
    let vm = core.view();
    // Keys pressed between Enter and the focus of the next editor belong to that editor.
    // Shortcuts with Ctrl or Meta cannot be replayed, so they are left to the browser here.
    if vm.with_untracked(|v| v.outline.key_relay_holding) && !ev.ctrl_key() && !ev.meta_key() {
        ev.prevent_default();
        core.send(Event::KeyHeld(dom::deferred_key(ev)));
        return;
    }
    let key = ev.key();
    let value = el.value();
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    let end = el.selection_end().ok().flatten().unwrap_or(0);
    let len = value.encode_utf16().count() as u32;
    let id = item_id.to_string();
    let send = |event: Event| core.send(event);

    if row.with_untracked(|r| r.suggestions_visible) {
        if matches!(key.as_str(), "ArrowUp" | "ArrowDown" | "Enter" | "Escape") {
            ev.prevent_default();
            if key == "Escape" {
                send(Event::WikiQuery { item_id: id, context: None });
            } else {
                send(Event::WikiKey { key });
            }
            return;
        }
    }

    if (key == "z" || key == "Z") && ev.ctrl_key() && !ev.alt_key() {
        ev.prevent_default();
        send(if ev.shift_key() { Event::Redo } else { Event::Undo });
        return;
    }

    if key == "o" && ev.ctrl_key() {
        if let Some(link) = wiki_link_at_caret(&value, start) {
            ev.prevent_default();
            send(Event::OpenWikiLink { text: link });
            return;
        }
    }

    if key == "Enter" && ev.ctrl_key() {
        ev.prevent_default();
        send(Event::TaskCycle { item_id: id });
        return;
    }

    if key == "Enter" && ev.shift_key() && !ev.ctrl_key() {
        // Shift+Enter: a hard line break, which Flat Markdown keeps as \n
        ev.prevent_default();
        insert_at_cursor(core, item_id, el, "\n", 1);
        return;
    }

    if key == "Enter" && !ev.shift_key() && !ev.ctrl_key() {
        let before: String = value.encode_utf16().take(start as usize).collect::<Vec<_>>().pipe();
        if peta_core::util::has_unclosed_code_fence(&before) {
            // Inside a code fence: a newline instead of a split
            ev.prevent_default();
            insert_at_cursor(core, item_id, el, "\n", 1);
            return;
        }
        ev.prevent_default();
        send(Event::EnterPressed { item_id: id, cursor: dom::utf16_to_chars(&value, start), now_ms: core.now_ms() });
        return;
    }

    if key == "Backspace" && start == 0 && end == 0 {
        ev.prevent_default();
        send(Event::BackspaceAtStart { item_id: id });
        return;
    }

    if key == "Delete" && start == len && end == len {
        ev.prevent_default();
        send(Event::DeleteAtEnd { item_id: id });
        return;
    }

    if ev.ctrl_key() && (key == "ArrowUp" || key == "ArrowDown") {
        ev.prevent_default();
        send(Event::CollapseToggle { item_id: id });
        return;
    }

    if ev.shift_key() && ev.alt_key() {
        let event = match key.as_str() {
            "ArrowUp" => Some(Event::MoveUp { item_id: id.clone() }),
            "ArrowDown" => Some(Event::MoveDown { item_id: id.clone() }),
            "ArrowLeft" => Some(Event::Outdent { item_id: id.clone() }),
            "ArrowRight" => Some(Event::Indent { item_id: id.clone() }),
            _ => None,
        };
        if let Some(event) = event {
            ev.prevent_default();
            send(event);
            return;
        }
    }

    if ev.shift_key() && !ev.alt_key() && !ev.ctrl_key() {
        if (key == "ArrowUp" && dom::is_on_first_line(el)) || (key == "ArrowDown" && dom::is_on_last_line(el)) {
            ev.prevent_default();
            ev.stop_propagation();
            send(Event::ShiftSelect { item_id: id });
            return;
        }
    }

    let cursor_chars = dom::utf16_to_chars(&value, start);
    if key == "ArrowUp" && dom::is_on_first_line(el) {
        ev.prevent_default();
        send(Event::NavigateUp { item_id: id, cursor: cursor_chars });
        return;
    }
    if key == "ArrowDown" && dom::is_on_last_line(el) {
        ev.prevent_default();
        send(Event::NavigateDown { item_id: id, cursor: cursor_chars });
        return;
    }
    if key == "ArrowLeft" && start == 0 {
        ev.prevent_default();
        // A large number puts the cursor at the end of the previous item
        send(Event::NavigateUp { item_id: id, cursor: usize::MAX / 2 });
        return;
    }
    if key == "ArrowRight" && start == len {
        ev.prevent_default();
        send(Event::NavigateDown { item_id: id, cursor: 0 });
        return;
    }

    // Auto-close [[ with ]]
    if key == "[" && start > 0 {
        let units: Vec<u16> = value.encode_utf16().collect();
        let prev_is_bracket = units[start as usize - 1] == '[' as u16;
        let next_two: String = String::from_utf16_lossy(&units[start as usize..(start as usize + 2).min(units.len())]);
        if prev_is_bracket && next_two != "]]" {
            ev.prevent_default();
            insert_at_cursor(core, item_id, el, "[]]", 1);
            return;
        }
    }

    if key == "Tab" {
        ev.prevent_default();
        send(if ev.shift_key() { Event::Outdent { item_id: id } } else { Event::Indent { item_id: id } });
    }
}

/// The text of the `[[wiki link]]` the caret is inside of.
fn wiki_link_at_caret(value: &str, start: u32) -> Option<String> {
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
fn check_wiki_link_context(core: &CoreHandle, item_id: &str, el: &web_sys::HtmlTextAreaElement) {
    let value = el.value();
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    let units: Vec<u16> = value.encode_utf16().collect();
    let before = String::from_utf16_lossy(&units[..(start as usize).min(units.len())]);
    let after = String::from_utf16_lossy(&units[(start as usize).min(units.len())..]);
    let close = |core: &CoreHandle| core.send(Event::WikiQuery { item_id: item_id.to_string(), context: None });
    let Some(open) = before.rfind("[[") else { return close(core) };
    let between = &before[open + 2..];
    if between.contains("]]") || !after.contains("]]") {
        return close(core);
    }
    let Some((x, _top, bottom)) = dom::caret_rect(el, start) else { return close(core) };
    core.send(Event::WikiQuery {
        item_id: item_id.to_string(),
        context: Some(WikiLinkContext { query: between.to_string(), x, y: bottom }),
    });
}

/// Plays keys that were pressed before this editor was on screen. Typed characters go into
/// the text; other keys go through the keydown handler.
fn replay_keys(core: &CoreHandle, el: &web_sys::HtmlTextAreaElement, keys: Vec<DeferredKey>) {
    let item_id = el.id().trim_start_matches("item-editor-").to_string();
    for key in keys {
        // A replayed Enter starts a new hold; the keys after it belong to the item it creates
        if core.view().with_untracked(|v| v.outline.key_relay_holding) {
            core.send(Event::KeyHeld(key));
            continue;
        }
        if key.is_typed_character() {
            let start = el.selection_start().ok().flatten().unwrap_or(0);
            let end = el.selection_end().ok().flatten().unwrap_or(0);
            replace_range(core, &item_id, el, &key.key, start, end);
        } else if !replay_editing_key(core, &item_id, el, &key) {
            let init = web_sys::KeyboardEventInit::new();
            init.set_key(&key.key);
            init.set_code(&key.code);
            init.set_shift_key(key.shift_key);
            init.set_ctrl_key(key.ctrl_key);
            init.set_alt_key(key.alt_key);
            init.set_meta_key(key.meta_key);
            init.set_bubbles(true);
            init.set_cancelable(true);
            if let Ok(ev) = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init) {
                let _ = el.dispatch_event(&ev);
            }
        }
    }
}

/// Does what the browser would have done for a key that edits or moves inside the text.
/// A synthetic keydown has no default action, so these are done by hand.
fn replay_editing_key(core: &CoreHandle, item_id: &str, el: &web_sys::HtmlTextAreaElement, key: &DeferredKey) -> bool {
    if key.shift_key || key.alt_key {
        return false;
    }
    let start = el.selection_start().ok().flatten().unwrap_or(0);
    let end = el.selection_end().ok().flatten().unwrap_or(0);
    let len = el.value().encode_utf16().count() as u32;
    let has_selection = start != end;
    match key.key.as_str() {
        "Backspace" => {
            if !has_selection && start == 0 {
                return false; // the handler merges with the item above
            }
            replace_range(core, item_id, el, "", if has_selection { start } else { start - 1 }, end);
            true
        }
        "Delete" => {
            if !has_selection && end == len {
                return false; // the handler merges the item below
            }
            replace_range(core, item_id, el, "", start, if has_selection { end } else { end + 1 });
            true
        }
        "ArrowLeft" => {
            let p = start.saturating_sub(1);
            let _ = el.set_selection_range(p, p);
            true
        }
        "ArrowRight" => {
            let p = (end + 1).min(len);
            let _ = el.set_selection_range(p, p);
            true
        }
        "Home" => {
            let _ = el.set_selection_range(0, 0);
            true
        }
        "End" => {
            let _ = el.set_selection_range(len, len);
            true
        }
        _ => false,
    }
}

/// Starts following the mouse; leaving the editor by more than 10px starts a selection.
fn start_drag(
    core: CoreHandle,
    drag: &Rc<RefCell<Drag>>,
    el: &web_sys::HtmlTextAreaElement,
    textarea: &NodeRef<leptos::html::Textarea>,
    skip_selection_check: bool,
) {
    let item_id = el.id().trim_start_matches("item-editor-").to_string();
    let mut d = drag.borrow_mut();
    d.rect = Some(el.get_bounding_client_rect());
    d.skip_selection_check = skip_selection_check;
    d.listeners.clear();
    let document = dom::document();
    let textarea = *textarea;
    let on_move = {
        let (drag, item_id) = (drag.clone(), item_id.clone());
        EventListener::new(&document, "mousemove", move |ev| {
            let ev: &web_sys::MouseEvent = ev.unchecked_ref();
            let (rect, skip) = {
                let d = drag.borrow();
                (d.rect.clone(), d.skip_selection_check)
            };
            let Some(rect) = rect else { return };
            if !skip {
                if let Some(el) = textarea.get() {
                    if el.selection_start().ok().flatten() == el.selection_end().ok().flatten() {
                        return;
                    }
                }
            }
            let y = ev.client_y() as f64;
            if y < rect.top() - 10.0 || y > rect.bottom() + 10.0 {
                core.send(Event::DragBeyondBounds { item_id: item_id.clone() });
                end_drag(&drag);
            }
        })
    };
    let on_up = {
        let drag = drag.clone();
        EventListener::new(&document, "mouseup", move |_| end_drag(&drag))
    };
    d.listeners.push(on_move);
    d.listeners.push(on_up);
}

fn end_drag(drag: &Rc<RefCell<Drag>>) {
    // Dropping the listeners inside their own callback is not allowed; defer it
    let drag = drag.clone();
    dom::after_render(move || {
        let mut d = drag.borrow_mut();
        d.rect = None;
        d.skip_selection_check = false;
        d.listeners.clear();
    });
}
