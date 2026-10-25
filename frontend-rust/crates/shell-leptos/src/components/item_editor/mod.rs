//! The textarea an item is edited in, ported from `item-editor.ts`. This file is the
//! component; the keyboard shortcuts, the text helpers, the `[[` completion, the replay of
//! held keys each have a file of their own.

mod keys;
mod replay;
mod text;
mod wiki;

use crate::components::item_drag::Drag;
use crate::components::task_checkbox::TaskCheckbox;
use crate::core::CoreHandle;
use crate::dom;
use keys::handle_keydown;
use leptos::prelude::*;
use peta_core::model::DeferredKey;
use peta_core::view::Row;
use peta_core::Event;
use replay::replay_keys;
use text::auto_close_code_fence;
use wasm_bindgen::JsCast;
use wiki::check_wiki_link_context;

/// The font size of a heading line being edited, by heading level.
const HEADING_FONT_SIZES: [&str; 5] = ["1.6em", "1.4em", "1.2em", "1.1em", "1em"];

#[component]
pub fn ItemEditor(row: RwSignal<Row>) -> impl IntoView {
    let core: CoreHandle = expect_context();
    let vm = core.view();
    let item_id = row.with_untracked(|r| r.item.id.clone());
    let textarea = NodeRef::<leptos::html::Textarea>::new();
    let drag: Drag = expect_context();

    let id = item_id.clone();
    // A one-line heading is edited at its heading size, in bold
    let heading_line = move || row.with(|r| r.heading_line);
    let font_size = move || {
        row.with(|r| match (r.heading_line, r.heading_level) {
            (true, level @ 1..=5) => HEADING_FONT_SIZES[level as usize - 1],
            _ => "",
        })
    };

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
            core.send(Event::BodyChanged {
                item_id: id.clone(),
                body,
                now_ms: core.now_ms(),
            });
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
            let Some(data) = ev.clipboard_data() else {
                return;
            };
            let cursor = dom::selection_start_chars(&el);
            if let Some(file) = dom::image_file(&data) {
                ev.prevent_default();
                let handle = core.keep_file(file);
                core.send(Event::PasteImage {
                    item_id: id.clone(),
                    file: handle,
                    cursor,
                });
                return;
            }
            // Windows puts \r\n on the clipboard; the textarea would normalize it anyway
            let text = data
                .get_data("text/plain")
                .unwrap_or_default()
                .replace("\r\n", "\n")
                .replace('\r', "\n");
            let html = data.get_data("text/html").unwrap_or_default();
            ev.prevent_default();
            core.send(Event::PasteText {
                item_id: id.clone(),
                text,
                html,
                cursor,
            });
        }
    };

    // An image dropped on the textarea goes in at the drop point, like a pasted one at the
    // cursor. Anything else keeps the browser's own drop behaviour, e.g. dropped text.
    let on_drop = {
        let id = item_id.clone();
        move |ev: web_sys::DragEvent| {
            let Some(el) = textarea.get() else { return };
            let Some(file) = ev.data_transfer().and_then(|d| dom::image_file(&d)) else {
                return;
            };
            ev.prevent_default();
            let cursor =
                dom::textarea_offset_at_point(&el, ev.client_x() as f64, ev.client_y() as f64);
            let handle = core.keep_file(file);
            core.send(Event::PasteImage {
                item_id: id.clone(),
                file: handle,
                cursor,
            });
        }
    };

    let on_blur = {
        let id = item_id.clone();
        move |_| {
            core.send(Event::EditFinished {
                item_id: id.clone(),
            })
        }
    };

    let on_mousedown = {
        let id = item_id.clone();
        move |_: web_sys::MouseEvent| {
            if let Some(el) = textarea.get() {
                drag.start(core, &id, &el, None);
            }
        }
    };

    view! {
        <app-item-editor>
            <TaskCheckbox row=row />
            <textarea
                node_ref=textarea
                id=format!("item-editor-{id}")
                style:font-size=font_size
                style:font-weight=move || if heading_line() { "bold" } else { "" }
                rows="1"
                on:input=on_input
                on:keydown=on_keydown
                on:paste=on_paste
                on:drop=on_drop
                on:blur=on_blur
                on:mousedown=on_mousedown
            ></textarea>
        </app-item-editor>
    }
}
