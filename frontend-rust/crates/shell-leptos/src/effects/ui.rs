//! What only the shell can do with the page: focus, scroll, clipboard, dialogs, the title,
//! the extension iframes.

use crate::core::CoreHandle;
use crate::dom;
use crux_core::Request;
use peta_core::effects::{FunctionInfo, LocalDateTimeDto, UiOp, UiOutput};
use peta_core::Event;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

/// One hosted extension. Messages sent before its document has loaded would reach the
/// initial blank document and be lost, so they wait in `pending` until `load` fires.
pub struct ExtensionFrame {
    pub iframe: web_sys::HtmlIFrameElement,
    pub loaded: bool,
    pub pending: Vec<JsValue>,
    _on_load: Closure<dyn FnMut()>,
}

#[derive(Default)]
pub struct Extensions {
    pub frames: Vec<ExtensionFrame>,
    /// Windows that sent a message, by the handle the core answers to.
    pub sources: std::collections::HashMap<u64, web_sys::Window>,
    pub next_source: u64,
    _listener: Option<Closure<dyn FnMut(web_sys::MessageEvent)>>,
}

/// One part of a clipboard write whose content is not known yet: the promise handed to
/// `ClipboardItem` and the functions that settle it.
struct DeferredPart {
    promise: js_sys::Promise,
    resolve: js_sys::Function,
    reject: js_sys::Function,
}

impl DeferredPart {
    fn new() -> Self {
        let mut settle = None;
        let promise = js_sys::Promise::new(&mut |resolve, reject| settle = Some((resolve, reject)));
        let (resolve, reject) = settle.expect("the promise executor runs at once");
        Self {
            promise,
            resolve,
            reject,
        }
    }
}

/// A clipboard write begun while the user's key or copy event is still being handled, whose
/// text and HTML the core delivers later with `CopyMarkdown`.
///
/// Safari lets a page write the clipboard only during a user gesture, and the gesture is over
/// by the time the core has fetched the tree markdown from the server. `ClipboardItem` takes
/// promises for exactly this: the write is requested at once, the content follows.
pub struct PendingCopy {
    text: DeferredPart,
    html: DeferredPart,
}

impl PendingCopy {
    fn start() -> Result<Self, JsValue> {
        let text = DeferredPart::new();
        let html = DeferredPart::new();
        let written = dom::write_clipboard_parts(&text.promise, &html.promise)?;
        spawn_local(async move {
            if let Err(e) = JsFuture::from(written).await {
                web_sys::console::error_1(&e);
            }
        });
        Ok(Self { text, html })
    }

    fn fill(self, markdown: &str, html: &str) -> Result<(), JsValue> {
        let text_blob = dom::text_blob(markdown, "text/plain")?;
        let html_blob = dom::text_blob(html, "text/html")?;
        self.text.resolve.call1(&JsValue::NULL, &text_blob)?;
        self.html.resolve.call1(&JsValue::NULL, &html_blob)?;
        Ok(())
    }

    fn give_up(self) {
        let reason = JsValue::from_str("copy abandoned");
        let _ = self.text.reject.call1(&JsValue::NULL, &reason);
        let _ = self.html.reject.call1(&JsValue::NULL, &reason);
    }
}

/// Begins the clipboard write of a copy or cut of the selection. Called from the handler of
/// the user's key or copy event, before the event goes to the core; see [`PendingCopy`].
/// Without `ClipboardItem` (older Firefox) nothing is begun and `CopyMarkdown` writes on its own.
pub fn begin_copy(handle: CoreHandle) {
    let pending = match PendingCopy::start() {
        Ok(pending) => pending,
        Err(e) => {
            web_sys::console::warn_1(&e);
            return;
        }
    };
    if let Some(earlier) = handle.inner().pending_copy.replace(Some(pending)) {
        earlier.give_up();
    }
}

pub fn perform(handle: CoreHandle, mut request: Request<UiOp>) {
    let op = request.operation.clone();
    match op {
        UiOp::FocusEditor {
            item_id,
            replay_keys,
        } => {
            // After the next render: the editor may have been added in this very update
            let h = handle;
            dom::after_render(move || {
                let selector = match &item_id {
                    Some(id) => format!("#item-editor-{id}"),
                    None => "app-item-editor textarea".to_string(),
                };
                if let Some(el) = dom::query::<web_sys::HtmlTextAreaElement>(&selector) {
                    dom::focus_without_scroll(&el);
                    dom::auto_resize(&el);
                }
                if !replay_keys.is_empty() {
                    if let Some(id) = &item_id {
                        if let Some(replay) = h.inner().replayers.borrow().get(id) {
                            replay(replay_keys.clone());
                        }
                    }
                }
                h.resolve(&mut request, UiOutput::Done);
            });
        }
        // The operations below are notifications: the core does not wait for an answer
        UiOp::ScrollToItem { item_id, smooth } => {
            if let Some(scroller) = handle.inner().scroller.borrow().as_ref() {
                scroller(item_id, smooth);
            }
        }
        UiOp::SetTitle(title) => dom::document().set_title(&title),
        UiOp::SaveAtlasDirs(dirs) => crate::components::atlas::save_atlas_dirs(&dirs),
        UiOp::CopyText(text) => {
            let _ = dom::window().navigator().clipboard().write_text(&text);
        }
        UiOp::CopyMarkdown { markdown } => {
            let html = flatmarkdown::body_to_html(&markdown);
            let pending = handle.inner().pending_copy.take();
            match pending {
                Some(pending) => {
                    if let Err(e) = pending.fill(&markdown, &html) {
                        web_sys::console::error_1(&e);
                    }
                }
                None => spawn_local(async move {
                    if let Err(e) = dom::write_clipboard_rich(&markdown, &html).await {
                        web_sys::console::error_1(&e);
                    }
                }),
            }
        }
        UiOp::CopyFailed => {
            if let Some(pending) = handle.inner().pending_copy.take() {
                pending.give_up();
            }
        }
        UiOp::ReadClipboardText => spawn_local(async move {
            let text = JsFuture::from(dom::window().navigator().clipboard().read_text())
                .await
                .ok()
                .and_then(|v| v.as_string());
            handle.resolve(&mut request, UiOutput::Text(text));
        }),
        UiOp::Alert(message) => {
            // Shown after the current update, like the Angular `setTimeout(() => alert())`
            dom::after_render(move || {
                let _ = dom::window().alert_with_message(&message);
            });
        }
        UiOp::Confirm(message) => {
            let ok = dom::window()
                .confirm_with_message(&message)
                .unwrap_or(false);
            handle.resolve(&mut request, UiOutput::Bool(ok));
        }
        UiOp::Prompt(message) => {
            let answer = dom::window().prompt_with_message(&message).ok().flatten();
            handle.resolve(&mut request, UiOutput::Text(answer));
        }
        UiOp::OpenWindow { url, new_window } => {
            let url = dom::encode_uri(&url);
            let _ = if new_window {
                dom::window().open_with_url_and_target_and_features(&url, "_blank", "noopener")
            } else {
                dom::window().open_with_url_and_target(&url, "_blank")
            };
        }
        UiOp::ExtensionPageCreate {
            page_id,
            file_name,
            dir_name,
            page_type,
        } => {
            post_to_frames(
                &handle,
                serde_json::json!({
                    "type": "PageCreate", "pageId": page_id, "fileName": file_name,
                    "dirName": dir_name, "pageType": page_type,
                }),
            );
        }
        UiOp::ExtensionPageTagAdd {
            page_id,
            file_name,
            dir_name,
            page_type,
            tags,
        } => {
            post_to_frames(
                &handle,
                serde_json::json!({
                    "type": "PageTagAdd", "pageId": page_id, "fileName": file_name,
                    "dirName": dir_name, "pageType": page_type, "tags": tags,
                }),
            );
        }
        UiOp::ExtensionReply { source, message } => {
            if let Some(win) = handle
                .inner()
                .extensions
                .borrow_mut()
                .sources
                .remove(&source)
            {
                let _ = win.post_message(&dom::to_js(&message), "*");
            }
        }
        UiOp::LoadExtensions { functions } => {
            load_extensions(&handle, &functions);
        }
        UiOp::LocalNow => {
            let d = js_sys::Date::new_0();
            let local = LocalDateTimeDto {
                year: d.get_full_year() as i32,
                month: d.get_month() + 1,
                day: d.get_date(),
                hour: d.get_hours(),
                minute: d.get_minutes(),
            };
            handle.resolve(
                &mut request,
                UiOutput::Now {
                    local,
                    now_ms: js_sys::Date::now() as u64,
                },
            );
        }
    }
}

/// Creates a hidden iframe per extension function and listens for their messages.
fn load_extensions(handle: &CoreHandle, functions: &[FunctionInfo]) {
    let document = dom::document();
    let Some(body) = document.body() else { return };
    let inner = handle.inner();
    let mut ext = inner.extensions.borrow_mut();
    for (index, f) in functions.iter().enumerate() {
        let Ok(el) = document.create_element("iframe") else {
            continue;
        };
        let iframe: web_sys::HtmlIFrameElement = el.unchecked_into();
        let h = *handle;
        let on_load = Closure::<dyn FnMut()>::new(move || {
            let inner = h.inner();
            let mut ext = inner.extensions.borrow_mut();
            if let Some(frame) = ext.frames.get_mut(index) {
                frame.loaded = true;
                for js in frame.pending.drain(..) {
                    post_to_frame(&frame.iframe, &js);
                }
            }
        });
        let _ = iframe.add_event_listener_with_callback("load", on_load.as_ref().unchecked_ref());
        // The src is set after the listener: a cached document could load at once
        iframe.set_src(&format!("http://localhost:8087{}", f.url));
        let _ = iframe.style().set_property("display", "none");
        let _ = body.append_child(&iframe);
        ext.frames.push(ExtensionFrame {
            iframe,
            loaded: false,
            pending: Vec::new(),
            _on_load: on_load,
        });
    }
    if ext._listener.is_none() {
        let h = *handle;
        let listener =
            Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |ev: web_sys::MessageEvent| {
                on_message(&h, ev);
            });
        let _ = dom::window()
            .add_event_listener_with_callback("message", listener.as_ref().unchecked_ref());
        ext._listener = Some(listener);
    }
}

/// Posts a message to every extension; one still loading gets it once loaded.
fn post_to_frames(handle: &CoreHandle, message: serde_json::Value) {
    let js = dom::to_js(&message);
    for frame in &mut handle.inner().extensions.borrow_mut().frames {
        if frame.loaded {
            post_to_frame(&frame.iframe, &js);
        } else {
            frame.pending.push(js.clone());
        }
    }
}

fn post_to_frame(iframe: &web_sys::HtmlIFrameElement, message: &JsValue) {
    if let Some(win) = iframe.content_window() {
        let _ = win.post_message(message, "*");
    }
}

fn on_message(handle: &CoreHandle, ev: web_sys::MessageEvent) {
    let Ok(data) = serde_wasm_bindgen::from_value::<serde_json::Value>(ev.data()) else {
        return;
    };
    let kind = data.get("type").and_then(|t| t.as_str());
    if kind != Some("CreateItem") && kind != Some("Confirm") {
        return;
    }
    let source = {
        let inner = handle.inner();
        let mut ext = inner.extensions.borrow_mut();
        ext.next_source += 1;
        let id = ext.next_source;
        // The extension runs on another origin, so its window fails `instanceof Window`
        // and `dyn_into` would drop it. `postMessage` is allowed across origins all the same.
        if let Some(win) = ev.source() {
            ext.sources.insert(id, win.unchecked_into::<web_sys::Window>());
        }
        id
    };
    let str_field = |k: &str| data.get(k).and_then(|v| v.as_str()).map(str::to_string);
    if kind == Some("Confirm") {
        handle.send(Event::ExtensionConfirm {
            source,
            request_id: str_field("requestId"),
            message: str_field("message").unwrap_or_default(),
        });
        return;
    }
    handle.send(Event::ExtensionCreateItem {
        source,
        request_id: str_field("requestId"),
        page_id: str_field("pageId").unwrap_or_default(),
        position: str_field("position"),
        after_item_id: str_field("afterItemId"),
        level: data.get("level").and_then(|l| l.as_i64()).unwrap_or(1) as i32,
        body: str_field("body").unwrap_or_default(),
        transclude: str_field("transclude"),
    });
}
