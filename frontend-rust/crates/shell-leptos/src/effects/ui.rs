//! What only the shell can do with the page: focus, scroll, clipboard, dialogs, the title.

use crate::core::CoreHandle;
use crate::dom;
use crux_core::Request;
use leptos::prelude::{Set, WithUntracked};
use peta_core::effects::{LocalDateTimeDto, UiOp, UiOutput};
use peta_page_links::{copied_html, AppLinks};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

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

/// A `ConfirmDestructive` request waiting for the dialog's answer.
pub struct PendingConfirm {
    request: Request<UiOp>,
}

/// Answers the confirmation dialog on screen, if one is open, and closes it.
pub fn answer_confirm(handle: CoreHandle, ok: bool) {
    let inner = handle.inner();
    let Some(mut pending) = inner.confirm.take() else {
        return;
    };
    inner.confirm_message.set(None);
    handle.resolve(&mut pending.request, UiOutput::Bool(ok));
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
            // After the next render: the editor may have been added in this very update.
            // Keys pressed until then join the held keys, or they would overtake them. They
            // are kept even when no key is held: a row put at another place has blurred its
            // editor, and a key pressed before the focus is back would reach no editor at all
            let h = handle;
            if item_id.is_some() || !replay_keys.is_empty() {
                h.inner()
                    .late_keys
                    .borrow_mut()
                    .get_or_insert_with(Vec::new);
            }
            dom::after_render(move || {
                let selector = match &item_id {
                    Some(id) => format!("#item-editor-{id}"),
                    None => "app-item-editor textarea".to_string(),
                };
                if let Some(el) = dom::query::<web_sys::HtmlTextAreaElement>(&selector) {
                    dom::focus_without_scroll(&el);
                    dom::auto_resize(&el);
                }
                let mut keys = replay_keys.clone();
                // Taken before the replay: a key it plays must not be kept again
                keys.extend(h.inner().late_keys.borrow_mut().take().unwrap_or_default());
                if !keys.is_empty() {
                    if let Some(id) = &item_id {
                        if let Some(replay) = h.inner().replayers.borrow().get(id) {
                            replay(keys);
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
        UiOp::CopyMarkdown { markdown, ast } => {
            // The origin is what another app needs to open a page of this app.
            let app_url = dom::window().location().origin().unwrap_or_default();
            let tags_dir = handle.view().with_untracked(|v| v.config.tags_dir.clone());
            let html = copied_html(
                &ast,
                AppLinks {
                    app_url: &app_url,
                    tags_dir: &tags_dir,
                },
            );
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
        UiOp::ConfirmDestructive(message) => {
            // The browser's confirm answers OK to Enter, so this one is a dialog of our own
            // whose Cancel button has the focus; see `ConfirmDialog`
            let inner = handle.inner();
            if let Some(mut earlier) = inner.confirm.replace(Some(PendingConfirm { request })) {
                handle.resolve(&mut earlier.request, UiOutput::Bool(false));
            }
            inner.confirm_message.set(Some(message));
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
