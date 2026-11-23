//! The core for the prototype web shells of #971 (ADR 079). A shell in TypeScript sends its
//! events in, and takes the changes of the screen out, as the native shells do with
//! `client-native-bridge`. The requests to the server, the event stream and the timers stay
//! here, in wasm.
//!
//! The changes come out two ways, as JSON text and as JS values, so that the prototype can
//! measure both.

mod effects;

use client_core::changes::{Changes, Shown};
use client_core::effects::{Effect, UiOp, UiOutput};
use client_core::keys::{KeyMap, KeyPlatform, KeyPress};
use client_core::policy::{InOrder, Timers};
use client_core::rest::ClientId;
use client_core::typing::{self, Typing};
use client_core::{BodyModel, Event, Route};
use crux_core::Request;
use flatmarkdown_ast::Node;
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// The body of a row as HTML, which the shell puts into its element as it is (ADR 079).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct HtmlBody(pub String);

impl BodyModel for HtmlBody {
    fn of(ast: Node, tags_dir: &str) -> HtmlBody {
        let options = page_urls::html_options(tags_dir);
        HtmlBody(flatmarkdown_ast2html::ast_to_html_with(&ast, &options))
    }
}

type App = client_core::PetaApp<HtmlBody>;

/// What the core and its effects share. The callbacks of the browser hold it.
pub(crate) struct Inner {
    core: crux_core::Core<App>,
    pub(crate) client_id: ClientId,
    keys: KeyMap,
    shown: RefCell<Shown<HtmlBody>>,
    pub(crate) timers: RefCell<Timers<effects::Running>>,
    pub(crate) requests: RefCell<InOrder<Request<ApiRequest>>>,
    pub(crate) sse: RefCell<Option<effects::Connection>>,
    /// The requests of the shell that wait for its answer, by their number.
    asked: RefCell<HashMap<u32, Request<UiOp>>>,
    last_asked: Cell<u32>,
    /// Told that the screen changed. The shell then takes the changes.
    on_render: js_sys::Function,
    /// Given a request of the shell: its number and the request as JSON.
    on_ui: js_sys::Function,
    /// Given a URL to show, and whether it replaces the one in the history.
    on_navigate: js_sys::Function,
}

pub(crate) type ApiRequest = client_core::effects::ApiOp;

impl Inner {
    fn send(self: &Rc<Self>, event: Event) {
        let effects = self.core.process_event(event);
        self.process(effects);
    }

    /// Answers a request of the core, and performs what follows.
    pub(crate) fn resolve<Op: crux_core::capability::Operation>(
        self: &Rc<Self>,
        request: &mut Request<Op>,
        output: Op::Output,
    ) {
        match self.core.resolve(request, output) {
            Ok(effects) => self.process(effects),
            Err(error) => web_sys::console::warn_1(&format!("resolve failed: {error}").into()),
        }
    }

    fn process(self: &Rc<Self>, effects: Vec<Effect>) {
        let mut render = false;
        for effect in effects {
            match effect {
                Effect::Render(_) => render = true,
                Effect::Api(request) => effects::call(self.clone(), request),
                Effect::Timer(request) => effects::time(self.clone(), request),
                Effect::Sse(request) => effects::listen(self.clone(), request),
                Effect::Ui(request) => self.ui(request),
                Effect::Navigate(request) => {
                    let op = &request.operation;
                    let _ = self.on_navigate.call2(
                        &JsValue::NULL,
                        &JsValue::from_str(&op.url),
                        &JsValue::from_bool(op.replace),
                    );
                }
            }
        }
        if render {
            let _ = self.on_render.call0(&JsValue::NULL);
        }
    }

    /// Answers what wasm can answer, and hands the rest to the shell.
    fn ui(self: &Rc<Self>, mut request: Request<UiOp>) {
        match &request.operation {
            UiOp::LocalNow => {
                let now = effects::local_now();
                let now_ms = js_sys::Date::now() as u64;
                self.resolve(&mut request, UiOutput::Now { local: now, now_ms });
            }
            UiOp::SetTitle(title) => {
                if let Some(document) = web_sys::window().and_then(|w| w.document()) {
                    document.set_title(title);
                }
            }
            op => {
                let json = serde_json::to_string(op).expect("an operation is always written");
                let id = self.last_asked.get() + 1;
                self.last_asked.set(id);
                self.asked.borrow_mut().insert(id, request);
                let _ = self.on_ui.call2(
                    &JsValue::NULL,
                    &JsValue::from(id),
                    &JsValue::from_str(&json),
                );
            }
        }
    }

    fn today(&self) -> String {
        effects::local_now().day_name()
    }

    fn take_changes(&self) -> Changes<HtmlBody> {
        self.shown.borrow_mut().changes(self.core.view())
    }
}

/// Why something from the shell could not be read.
fn unreadable(error: serde_json::Error) -> JsValue {
    JsValue::from_str(&format!("unreadable: {error}"))
}

/// The JS values of the changes. Maps become objects, as in JSON, so that both ways give the
/// shell the same shape.
fn to_value<T: Serialize>(value: &T) -> JsValue {
    let serializer = serde_wasm_bindgen::Serializer::json_compatible();
    value
        .serialize(&serializer)
        .expect("the changes are always written")
}

/// The core for one prototype shell.
#[wasm_bindgen]
pub struct Shell {
    inner: Rc<Inner>,
}

#[wasm_bindgen]
impl Shell {
    /// Makes the core. It starts with [`Shell::start`], after the shell is ready for its
    /// callbacks.
    #[wasm_bindgen(constructor)]
    pub fn new(
        on_render: js_sys::Function,
        on_ui: js_sys::Function,
        on_navigate: js_sys::Function,
    ) -> Shell {
        console_error_panic_hook::set_once();
        let inner = Rc::new(Inner {
            core: crux_core::Core::new(),
            client_id: ClientId::new(),
            keys: KeyMap::of(KeyPlatform::Web),
            shown: RefCell::default(),
            timers: RefCell::default(),
            requests: RefCell::default(),
            sse: RefCell::default(),
            asked: RefCell::default(),
            last_asked: Cell::new(0),
            on_render,
            on_ui,
            on_navigate,
        });
        Shell { inner }
    }

    /// Starts the core, which connects to the server of the page.
    pub fn start(&self) {
        let client_id = self.inner.client_id.clone();
        self.inner.send(Event::Start { client_id });
    }

    /// Shows what the URL of the browser names, such as `/pages/notes` or `/today`. False when
    /// it names nothing.
    pub fn route(&self, url: &str) -> bool {
        let Some(route) = Route::parse_url(url) else {
            return false;
        };
        let today = self.inner.today();
        self.inner.send(Event::Route { route, today });
        true
    }

    /// Takes an [`Event`] as JSON.
    pub fn update(&self, event: &str) -> Result<(), JsValue> {
        let event: Event = serde_json::from_str(event).map_err(unreadable)?;
        self.inner.send(event);
        Ok(())
    }

    /// Takes a [`KeyPress`] as JSON. False when the key means nothing to the outline, and the
    /// shell leaves it to its editor.
    pub fn press(&self, key: &str) -> Result<bool, JsValue> {
        let press: KeyPress = serde_json::from_str(key).map_err(unreadable)?;
        let now_ms = js_sys::Date::now() as u64;
        let Some(event) = self.inner.keys.event_of(&press, now_ms) else {
            return Ok(false);
        };
        self.inner.send(event);
        Ok(true)
    }

    /// Takes a [`Typing`] as JSON.
    pub fn typed(&self, typing: &str) -> Result<(), JsValue> {
        let typing: Typing = serde_json::from_str(typing).map_err(unreadable)?;
        let now_ms = js_sys::Date::now() as u64;
        self.inner.send(typing::event_of(typing, now_ms));
        Ok(())
    }

    /// Answers the request `id` of the shell with a `UiOutput` as JSON.
    #[wasm_bindgen(js_name = resolveUi)]
    pub fn resolve_ui(&self, id: u32, output: &str) -> Result<(), JsValue> {
        let output: UiOutput = serde_json::from_str(output).map_err(unreadable)?;
        let request = self.inner.asked.borrow_mut().remove(&id);
        let Some(mut request) = request else {
            return Err(JsValue::from_str(&format!("no request {id}")));
        };
        self.inner.resolve(&mut request, output);
        Ok(())
    }

    /// What changed on the screen since the last call, as JSON. The first call gives a reset.
    #[wasm_bindgen(js_name = changesJson)]
    pub fn changes_json(&self) -> String {
        let changes = self.inner.take_changes();
        serde_json::to_string(&changes).expect("the changes are always written")
    }

    /// What changed on the screen since the last call, as JS values.
    #[wasm_bindgen(js_name = changesValue)]
    pub fn changes_value(&self) -> JsValue {
        to_value(&self.inner.take_changes())
    }

    /// A reset of every row on the screen now, as JSON. It does not change what the shell was
    /// told: only the measurement of #971 calls it.
    #[wasm_bindgen(js_name = resetJson)]
    pub fn reset_json(&self) -> String {
        let changes = Shown::default().changes(self.inner.core.view());
        serde_json::to_string(&changes).expect("the changes are always written")
    }

    /// The same reset as [`Shell::reset_json`], as JS values.
    #[wasm_bindgen(js_name = resetValue)]
    pub fn reset_value(&self) -> JsValue {
        to_value(&Shown::default().changes(self.inner.core.view()))
    }
}
