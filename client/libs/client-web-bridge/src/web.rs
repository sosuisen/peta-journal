//! The browser behind the core: `fetch`, the event stream, the timers, the APIs of `window`,
//! and the history (ADR 079). The shell in TypeScript holds a [`WebShell`].
//!
//! Only the browser runs this file. The code it calls is in [`crate::bridge`], whose tests run
//! the same code with a fake browser.

use crate::bridge::{Bridge, Done, Host, Listener, ATLAS_DIRS_KEY};
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent, UiOp, UiOutput,
};
use client_core::model::Uploaded;
use client_core::rest::{rest_call, routes, ClientId, HttpReply, HttpRequest};
use gloo_net::http::Request as Http;
use gloo_timers::callback::Timeout;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};

fn window() -> web_sys::Window {
    web_sys::window().expect("the page has a window")
}

/// The path and the fragment of the URL in the address bar, such as `/pages/notes#item-1`.
fn location() -> String {
    let location = window().location();
    let path = location.pathname().unwrap_or_default();
    let hash = location.hash().unwrap_or_default();
    format!("{path}{hash}")
}

fn api_err(error: impl std::fmt::Display) -> ApiError {
    ApiError(error.to_string())
}

/// What the stream of the server calls with each of its events. Its three callbacks share it.
type Heard = Rc<RefCell<Listener>>;

fn tell(heard: &Heard, event: SseEvent) {
    (heard.borrow_mut())(event);
}

/// The event stream of the server, with the callbacks that keep it open.
struct Connection {
    source: web_sys::EventSource,
    _on_open: Closure<dyn FnMut(web_sys::Event)>,
    _on_message: Closure<dyn FnMut(web_sys::MessageEvent)>,
    _on_error: Closure<dyn FnMut(web_sys::Event)>,
}

impl Drop for Connection {
    fn drop(&mut self) {
        self.source.close();
    }
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
struct PendingCopy {
    text: DeferredPart,
    html: DeferredPart,
}

impl PendingCopy {
    fn start() -> Result<Self, JsValue> {
        let text = DeferredPart::new();
        let html = DeferredPart::new();
        let written = write_clipboard_parts(&text.promise, &html.promise)?;
        spawn_local(async move {
            if let Err(e) = JsFuture::from(written).await {
                web_sys::console::error_1(&e);
            }
        });
        Ok(Self { text, html })
    }

    fn fill(self, markdown: &str, html: &str) -> Result<(), JsValue> {
        let text = text_blob(markdown, "text/plain")?;
        let html = text_blob(html, "text/html")?;
        self.text.resolve.call1(&JsValue::NULL, &text)?;
        self.html.resolve.call1(&JsValue::NULL, &html)?;
        Ok(())
    }

    fn give_up(self) {
        let reason = JsValue::from_str("copy abandoned");
        let _ = self.text.reject.call1(&JsValue::NULL, &reason);
        let _ = self.html.reject.call1(&JsValue::NULL, &reason);
    }
}

/// A blob of `content` with the MIME type `mime`.
fn text_blob(content: &str, mime: &str) -> Result<web_sys::Blob, JsValue> {
    let parts = js_sys::Array::new();
    parts.push(&JsValue::from_str(content));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type(mime);
    web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
}

/// Starts a clipboard write of a `text/plain` and a `text/html` part, each given as a blob
/// or as a promise of one, and returns the promise of the write.
fn write_clipboard_parts(text: &JsValue, html: &JsValue) -> Result<js_sys::Promise, JsValue> {
    let record = js_sys::Object::new();
    js_sys::Reflect::set(&record, &"text/plain".into(), text)?;
    js_sys::Reflect::set(&record, &"text/html".into(), html)?;
    let item =
        web_sys::ClipboardItem::new_with_record_from_str_to_blob_promise(&record.unchecked_into())?;
    let items = js_sys::Array::new();
    items.push(&item);
    Ok(window().navigator().clipboard().write(&items))
}

/// What the browser keeps for the core between its callbacks.
#[derive(Default)]
struct Kept {
    /// The timers that run, by their id.
    timers: RefCell<HashMap<u64, Timeout>>,
    stream: RefCell<Option<Connection>>,
    copy: RefCell<Option<PendingCopy>>,
}

/// The browser, with the callbacks of the shell.
struct Browser {
    client_id: ClientId,
    kept: Rc<Kept>,
    /// Told that the screen changed. The shell then takes the changes.
    on_render: js_sys::Function,
    /// Given an operation of the screen that only the shell can do: its number and the
    /// operation as JSON.
    on_ask: js_sys::Function,
}

impl Host for Browser {
    type File = web_sys::File;

    fn now(&self) -> (LocalDateTimeDto, u64) {
        let date = js_sys::Date::new_0();
        let local = LocalDateTimeDto {
            year: date.get_full_year() as i32,
            month: date.get_month() + 1,
            day: date.get_date(),
            hour: date.get_hours(),
            minute: date.get_minutes(),
        };
        (local, date.get_time() as u64)
    }

    fn call(&self, op: ApiOp, done: Done<ApiResult<ApiOutput>>) {
        let client_id = self.client_id.clone();
        spawn_local(async move {
            let output = match rest_call(op) {
                Some(rest) => match send(&client_id, &rest.request).await {
                    Ok(reply) => rest.decode(&reply),
                    Err(error) => Err(error),
                },
                None => Err(ApiError("not a REST call".into())),
            };
            done(output);
        });
    }

    fn upload(&self, file: web_sys::File, done: Done<ApiResult<ApiOutput>>) {
        let client_id = self.client_id.clone();
        spawn_local(async move {
            done(upload(&client_id, &file).await);
        });
    }

    fn listen(&self, heard: Listener) {
        // One stream at a time: a new one replaces the old
        self.kept.stream.borrow_mut().take();
        let Ok(source) = web_sys::EventSource::new(routes::EVENTS.pattern) else {
            let mut heard = heard;
            heard(SseEvent::Error);
            return;
        };
        let heard: Heard = Rc::new(RefCell::new(heard));
        let on_open = {
            let heard = heard.clone();
            Closure::<dyn FnMut(web_sys::Event)>::new(move |_| tell(&heard, SseEvent::Open))
        };
        let on_message = {
            let heard = heard.clone();
            Closure::<dyn FnMut(web_sys::MessageEvent)>::new(move |event: web_sys::MessageEvent| {
                if let Some(data) = event.data().as_string() {
                    tell(&heard, SseEvent::Message(data));
                }
            })
        };
        let on_error = {
            let closing = source.clone();
            Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                // Closed, not dropped: the callback that runs now is kept by the connection.
                // The browser would open the stream again by itself, but the core does that
                closing.close();
                tell(&heard, SseEvent::Error);
            })
        };
        source.set_onopen(Some(on_open.as_ref().unchecked_ref()));
        source.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        source.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        *self.kept.stream.borrow_mut() = Some(Connection {
            source,
            _on_open: on_open,
            _on_message: on_message,
            _on_error: on_error,
        });
    }

    fn start_timer(&self, id: u64, ms: u32, fired: Done<()>) {
        let kept = self.kept.clone();
        let timeout = Timeout::new(ms, move || {
            // Forgotten, not dropped: it is the timeout that runs now
            if let Some(timeout) = kept.timers.borrow_mut().remove(&id) {
                timeout.forget();
            }
            fired(());
        });
        self.kept.timers.borrow_mut().insert(id, timeout);
    }

    fn cancel_timer(&self, id: u64) {
        let timeout = self.kept.timers.borrow_mut().remove(&id);
        if let Some(timeout) = timeout {
            timeout.cancel();
        }
    }

    fn screen(&self, op: &UiOp, done: Done<UiOutput>) {
        match op {
            UiOp::SetTitle(title) => {
                if let Some(document) = window().document() {
                    document.set_title(title);
                }
            }
            UiOp::SaveAtlasDirs(dirs) => {
                let json = serde_json::to_string(dirs).expect("folder names are always written");
                if let Ok(Some(storage)) = window().local_storage() {
                    let _ = storage.set_item(ATLAS_DIRS_KEY, &json);
                }
            }
            UiOp::CopyFailed => {
                if let Some(pending) = self.kept.copy.borrow_mut().take() {
                    pending.give_up();
                }
            }
            UiOp::ReadClipboardText => spawn_local(async move {
                let read = window().navigator().clipboard().read_text();
                let text = JsFuture::from(read).await.ok().and_then(|v| v.as_string());
                done(UiOutput::Text(text));
            }),
            UiOp::Alert(message) => {
                // Shown after the screen is drawn: the dialog holds the page until it closes
                let message = message.clone();
                Timeout::new(0, move || {
                    let _ = window().alert_with_message(&message);
                })
                .forget();
            }
            UiOp::Confirm(message) => {
                let ok = window().confirm_with_message(message).unwrap_or(false);
                done(UiOutput::Bool(ok));
            }
            UiOp::Prompt(message) => {
                let answer = window().prompt_with_message(message).ok().flatten();
                done(UiOutput::Text(answer));
            }
            // The core makes the URL encoded (ADR 061). A new view of the app is a new tab
            UiOp::OpenNewView { url } => {
                let _ = window().open_with_url_and_target(url, "_blank");
            }
            // The browser is a new window, that the page it opens cannot reach back from
            UiOp::OpenInBrowser { address } => {
                let _ =
                    window().open_with_url_and_target_and_features(address, "_blank", "noopener");
            }
            // The rest are not for the window (`crate::lanes`)
            _ => {}
        }
    }

    fn origin(&self) -> String {
        window().location().origin().unwrap_or_default()
    }

    fn copy(&self, markdown: &str, html: &str) {
        let pending = self.kept.copy.borrow_mut().take();
        match pending {
            Some(pending) => {
                if let Err(e) = pending.fill(markdown, html) {
                    web_sys::console::error_1(&e);
                }
            }
            // Without `ClipboardItem` promises begun in the gesture: written as it comes
            None => {
                let text = text_blob(markdown, "text/plain");
                let html = text_blob(html, "text/html");
                let written = text
                    .and_then(|text| html.map(|html| (text, html)))
                    .and_then(|(text, html)| write_clipboard_parts(&text, &html));
                match written {
                    Ok(written) => spawn_local(async move {
                        if let Err(e) = JsFuture::from(written).await {
                            web_sys::console::error_1(&e);
                        }
                    }),
                    Err(e) => web_sys::console::error_1(&e),
                }
            }
        }
    }

    fn ask(&self, id: u32, op: &UiOp) {
        let json = facet_json::to_string(op).expect("an operation is always written");
        let _ = self.on_ask.call2(
            &JsValue::NULL,
            &JsValue::from(id),
            &JsValue::from_str(&json),
        );
    }

    fn load(&self, key: &str) -> Option<String> {
        let storage = window().local_storage().ok().flatten()?;
        storage.get_item(key).ok().flatten()
    }

    fn go_to(&self, url: &str, replace: bool) {
        let Ok(history) = window().history() else {
            return;
        };
        // The core makes the URL encoded (ADR 061), as the address bar keeps it
        let _ = if replace {
            history.replace_state_with_url(&JsValue::NULL, "", Some(url))
        } else {
            history.push_state_with_url(&JsValue::NULL, "", Some(url))
        };
    }

    fn rendered(&self) {
        let _ = self.on_render.call0(&JsValue::NULL);
    }
}

async fn send(client_id: &ClientId, request: &HttpRequest) -> ApiResult<HttpReply> {
    let builder = match request.method {
        "POST" => Http::post(&request.path),
        "PUT" => Http::put(&request.path),
        "DELETE" => Http::delete(&request.path),
        _ => Http::get(&request.path),
    }
    .header(ClientId::HEADER, client_id.as_str());
    let sent = match &request.body {
        Some(body) => builder
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .map_err(api_err)?,
        None => builder.build().map_err(api_err)?,
    };
    let response = sent.send().await.map_err(api_err)?;
    Ok(HttpReply {
        status: response.status(),
        body: response.text().await.map_err(api_err)?,
    })
}

/// Sends the file as an asset. Not a REST call of the core: a `File` cannot cross into it.
async fn upload(client_id: &ClientId, file: &web_sys::File) -> ApiResult<ApiOutput> {
    let form = web_sys::FormData::new().map_err(|_| ApiError("FormData".into()))?;
    form.append_with_blob("file", file)
        .map_err(|_| ApiError("FormData".into()))?;
    let response = Http::post(routes::UPLOAD_ASSET.pattern)
        .header(ClientId::HEADER, client_id.as_str())
        .body(form)
        .map_err(api_err)?
        .send()
        .await
        .map_err(api_err)?;
    if !response.ok() {
        return Err(ApiError(format!("Upload failed: {}", response.status())));
    }
    let uploaded: Uploaded = response.json().await.map_err(api_err)?;
    Ok(ApiOutput::Uploaded(uploaded))
}

/// The core for the web shell. Each call that changes the screen returns the changes as JSON
/// (ADR 079). A change that comes later, as with an answer of the server, is told with the
/// callback `onRender`, and the shell takes it with [`WebShell::changes`].
#[wasm_bindgen]
pub struct WebShell {
    bridge: Rc<Bridge<Browser>>,
    kept: Rc<Kept>,
    _on_popstate: Closure<dyn FnMut(web_sys::Event)>,
}

#[wasm_bindgen]
impl WebShell {
    /// Makes the core. `onRender()` is told that the screen changed. `onAsk(id, json)` is
    /// given an operation of the screen that only the shell can do, as a `UiOp`; the shell
    /// answers it with [`WebShell::resolve`].
    #[wasm_bindgen(constructor)]
    pub fn new(on_render: js_sys::Function, on_ask: js_sys::Function) -> WebShell {
        console_error_panic_hook::set_once();
        let kept = Rc::new(Kept::default());
        let browser = Browser {
            client_id: ClientId::new(),
            kept: kept.clone(),
            on_render,
            on_ask,
        };
        let client_id = browser.client_id.clone();
        let bridge = Bridge::new(browser, client_id);
        let popped = Rc::downgrade(&bridge);
        let on_popstate = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            if let Some(bridge) = popped.upgrade() {
                bridge.popped(&location());
            }
        });
        window().set_onpopstate(Some(on_popstate.as_ref().unchecked_ref()));
        WebShell {
            bridge,
            kept,
            _on_popstate: on_popstate,
        }
    }

    /// Starts the core, which connects to the server, and opens the page of the address bar.
    /// Returns the changes, as JSON.
    pub fn start(&self) -> String {
        self.bridge.start(&location())
    }

    /// Opens a link of the app, such as `/pages/notes`, and keeps it in the history of the
    /// browser. Returns the changes.
    pub fn go(&self, url: &str) -> String {
        self.bridge.go(url)
    }

    /// Takes an `Event` as JSON, and returns the changes.
    pub fn update(&self, event: &str) -> Result<String, JsError> {
        self.bridge.update(event).map_err(|e| JsError::new(&e))
    }

    /// Takes a `KeyPress` as JSON. Returns the changes, or `undefined` when the key means
    /// nothing to the outline there, and the shell leaves it to its editor.
    pub fn press(&self, key: &str) -> Result<Option<String>, JsError> {
        self.bridge.press(key).map_err(|e| JsError::new(&e))
    }

    /// Takes a `Typing` as JSON, and returns the changes.
    pub fn typed(&self, typing: &str) -> Result<String, JsError> {
        self.bridge.typed(typing).map_err(|e| JsError::new(&e))
    }

    /// Takes a `DragMove` as JSON. Returns the changes, or `undefined` while the drag stays
    /// in the text.
    pub fn drag(&self, step: &str) -> Result<Option<String>, JsError> {
        self.bridge.drag(step).map_err(|e| JsError::new(&e))
    }

    /// Answers the operation `id` with a `UiOutput` as JSON, and returns the changes.
    pub fn resolve(&self, id: u32, output: &str) -> Result<String, JsError> {
        self.bridge
            .resolve(id, output)
            .map_err(|e| JsError::new(&e))
    }

    /// What changed on the screen since the last call, as JSON. The first call gives a reset.
    pub fn changes(&self) -> String {
        self.bridge.changes()
    }

    /// Pastes the image in `file` into the item, at `cursor` in characters. Returns the
    /// changes.
    #[wasm_bindgen(js_name = pasteImage)]
    pub fn paste_image(&self, item_id: &str, file: web_sys::File, cursor: usize) -> String {
        self.bridge.paste_image(item_id, file, cursor)
    }

    /// What a held key does in the editor with `text` and the selection from `start` to `end`,
    /// in UTF-16 units. Takes a `DeferredKey` as JSON, and returns a `Replay` as JSON.
    #[wasm_bindgen(js_name = replayOf)]
    pub fn replay_of(
        &self,
        key: &str,
        text: &str,
        start: u32,
        end: u32,
    ) -> Result<String, JsError> {
        crate::replay::replay_json(key, text, start.into(), end.into())
            .map_err(|e| JsError::new(&e))
    }

    /// The text typed inside the `[[wiki link]]` that the cursor at the UTF-16 place `cursor`
    /// is in. Undefined outside a link.
    #[wasm_bindgen(js_name = wikiQueryAt)]
    pub fn wiki_query_at(&self, text: &str, cursor: u32) -> Option<String> {
        crate::wiki::query_at(text, cursor.into())
    }

    /// What a text typed from `before` to `text` becomes with the cursor at the UTF-16 place
    /// `cursor`: an opening just typed gets its closing, as in the editor of a page. Undefined
    /// when it stays as it is.
    #[wasm_bindgen(js_name = autoClosed)]
    pub fn auto_closed(&self, before: &str, text: &str, cursor: u32) -> Option<String> {
        crate::wiki::closed_at(before, text, cursor.into())
    }

    /// The text with the page name `title` put into the `[[wiki link]]` that the cursor at the
    /// UTF-16 place `cursor` is in, as JSON: `{ text, cursor }`, where cursor is the UTF-16
    /// place right after the name. Undefined outside a link.
    #[wasm_bindgen(js_name = wikiPicked)]
    pub fn wiki_picked(&self, text: &str, cursor: u32, title: &str) -> Option<String> {
        let (text, cursor) = crate::wiki::picked_at(text, cursor.into(), title)?;
        Some(serde_json::json!({ "text": text, "cursor": cursor }).to_string())
    }

    /// The URL of the page of the tag `tag`, in the folder of the tags `tagsDir`.
    #[wasm_bindgen(js_name = tagUrl)]
    pub fn tag_url(&self, tags_dir: &str, tag: &str) -> String {
        crate::tags::tag_url(tags_dir, tag)
    }

    /// The names the settings show for the keys of the property definitions, given as JSON,
    /// in their order.
    #[wasm_bindgen(js_name = shownNames)]
    pub fn shown_names(&self, definitions: &str) -> Result<Vec<String>, JsError> {
        crate::settings::shown_names(definitions).map_err(|e| JsError::new(&e))
    }

    /// The types a user may give a new key, in the order the settings offer them.
    #[wasm_bindgen(js_name = userValueTypes)]
    pub fn user_value_types(&self) -> Vec<String> {
        crate::settings::user_value_types()
    }

    /// The placed cards of an atlas, the link cards among them, as `setCards` of `atlas.js`
    /// takes them. The cards and the links are given as JSON.
    #[wasm_bindgen(js_name = atlasCards)]
    pub fn atlas_cards(&self, cards: &str, links: &str) -> Result<String, JsError> {
        crate::atlas::placed_json(cards, links).map_err(|e| JsError::new(&e))
    }

    /// The whole text of a card of an atlas as HTML, drawn as the body of a row of a page is.
    /// The text is the Flat Markdown of its item; `tagsDir` is the folder of the tags.
    #[wasm_bindgen(js_name = bodyHtml)]
    pub fn body_html(&self, text: &str, tags_dir: &str) -> String {
        use client_core::BodyModel;
        crate::body::HtmlBody::of(client_core::asts::parse(text), tags_dir).0
    }

    /// The callouts of an atlas as `setCallouts` of `atlas.js` takes them. The callouts are
    /// given as JSON.
    #[wasm_bindgen(js_name = atlasCallouts)]
    pub fn atlas_callouts(&self, callouts: &str) -> Result<String, JsError> {
        crate::atlas::callouts_json(callouts).map_err(|e| JsError::new(&e))
    }

    /// How far the outline scrolls in one step while a drag selects items, with the mouse at
    /// `y` in a list seen from `top` to `bottom`. Below zero is up; zero outside the bands.
    #[wasm_bindgen(js_name = edgeStep)]
    pub fn edge_step(&self, y: f64, top: f64, bottom: f64) -> f64 {
        client_core::drag::edge_step(y, top, bottom)
    }

    /// How long one step of the scroll at an edge of the outline takes, in milliseconds.
    #[wasm_bindgen(js_name = edgeStepMs)]
    pub fn edge_step_ms(&self) -> u32 {
        client_core::drag::EDGE_STEP_MS
    }

    /// The folder above the folder `dirPath`, as the up button of an atlas names it.
    #[wasm_bindgen(js_name = parentDirLabel)]
    pub fn parent_dir_label(&self, dir_path: &str) -> String {
        crate::atlas::parent_dir_label(dir_path)
    }

    /// Begins the clipboard write of a copy or a cut of the selection. The shell calls this
    /// from the handler of the user's key or copy event, before the event goes to the core:
    /// Safari lets a page write the clipboard only then. Without `ClipboardItem` (older
    /// Firefox) nothing is begun, and the copy is written as it comes.
    #[wasm_bindgen(js_name = beginCopy)]
    pub fn begin_copy(&self) {
        let pending = match PendingCopy::start() {
            Ok(pending) => pending,
            Err(e) => {
                web_sys::console::warn_1(&e);
                return;
            }
        };
        if let Some(earlier) = self.kept.copy.borrow_mut().replace(pending) {
            earlier.give_up();
        }
    }
}

impl Drop for WebShell {
    fn drop(&mut self) {
        window().set_onpopstate(None);
    }
}
