//! Holds the Crux core and runs its effects: every event goes in through [`CoreHandle::send`],
//! the effects that come back are performed by the `effects` modules, and each `Render`
//! writes the new view model into a signal the components read.

use crate::effects;
use leptos::prelude::*;
use peta_core::effects::{Effect, TimerOp};
use peta_core::model::DeferredKey;
use peta_core::{Event, PetaApp, ViewModel};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

pub type ReplayKeys = Box<dyn Fn(Vec<DeferredKey>)>;
pub type ScrollToItem = Box<dyn Fn(String, bool)>;
pub type NavigateFn = Box<dyn Fn(&str, bool)>;

pub struct Inner {
    pub core: crux_core::Core<PetaApp>,
    pub view: RwSignal<ViewModel>,
    pub navigate: RefCell<Option<NavigateFn>>,
    /// Files handed to the core by handle, e.g. a pasted image waiting for its upload.
    pub files: RefCell<HashMap<u64, web_sys::File>>,
    pub next_handle: Cell<u64>,
    pub timers: RefCell<HashMap<u64, effects::timer::Running>>,
    pub sse: RefCell<Option<effects::sse::Connection>>,
    /// Editors that can replay held keys, by item id.
    pub replayers: RefCell<HashMap<String, ReplayKeys>>,
    /// The outline's scroller, when an outline is on screen.
    pub scroller: RefCell<Option<ScrollToItem>>,
    pub extensions: RefCell<effects::ui::Extensions>,
    /// A clipboard write begun on the user's copy key, waiting for the markdown from the core.
    pub pending_copy: RefCell<Option<effects::ui::PendingCopy>>,
}

thread_local! {
    static INNER: RefCell<Option<Rc<Inner>>> = const { RefCell::new(None) };
}

/// A copyable handle to the one core of the page. The core lives in a thread local, so the
/// handle can be captured by any closure without clones.
#[derive(Clone, Copy)]
pub struct CoreHandle;

impl CoreHandle {
    /// Creates the core; called once at start.
    pub fn init() -> CoreHandle {
        let core = crux_core::Core::<PetaApp>::new();
        let view = RwSignal::new(core.view());
        let inner = Rc::new(Inner {
            core,
            view,
            navigate: RefCell::new(None),
            files: RefCell::new(HashMap::new()),
            next_handle: Cell::new(1),
            timers: RefCell::new(HashMap::new()),
            sse: RefCell::new(None),
            replayers: RefCell::new(HashMap::new()),
            scroller: RefCell::new(None),
            extensions: RefCell::new(effects::ui::Extensions::default()),
            pending_copy: RefCell::new(None),
        });
        INNER.with(|i| *i.borrow_mut() = Some(inner));
        CoreHandle
    }

    pub fn inner(&self) -> Rc<Inner> {
        INNER.with(|i| i.borrow().clone().expect("the core is not started"))
    }

    pub fn view(&self) -> RwSignal<ViewModel> {
        self.inner().view
    }

    /// One part of the view model as a memo: what reads it re-renders when that part
    /// changes, not on every event.
    pub fn select<T>(&self, pick: impl Fn(&ViewModel) -> T + Send + Sync + 'static) -> Memo<T>
    where
        T: PartialEq + Send + Sync + 'static,
    {
        let view = self.view();
        Memo::new(move |_| view.with(|v| pick(v)))
    }

    /// Sends an event to the core and performs the effects it asks for.
    pub fn send(&self, event: Event) {
        if debug_enabled() {
            web_sys::console::log_1(&format!("event: {event:?}").into());
        }
        let effects = self.inner().core.process_event(event);
        self.process(effects);
    }

    /// Answers a request and performs what follows.
    pub fn resolve<Op: crux_core::capability::Operation>(
        &self,
        request: &mut crux_core::Request<Op>,
        output: Op::Output,
    ) {
        match self.inner().core.resolve(request, output) {
            Ok(effects) => self.process(effects),
            Err(e) => web_sys::console::warn_1(&format!("resolve failed: {e}").into()),
        }
    }

    pub fn process(&self, effects: Vec<Effect>) {
        let mut render = false;
        for effect in effects {
            match effect {
                Effect::Render(_) => render = true,
                Effect::Api(request) => effects::api::perform(*self, request),
                Effect::Timer(request) => match request.operation {
                    TimerOp::Start { id, ms } => effects::timer::start(*self, id, ms, request),
                    TimerOp::Cancel { id } => effects::timer::cancel(self, id),
                },
                Effect::Sse(request) => effects::sse::connect(*self, request),
                Effect::Ui(request) => effects::ui::perform(*self, request),
                // A notification: the core does not wait for it
                Effect::Navigate(request) => {
                    let op = &request.operation;
                    if let Some(nav) = self.inner().navigate.borrow().as_ref() {
                        nav(&op.url, op.replace);
                    }
                }
            }
        }
        if render {
            self.inner().view.set(self.inner().core.view());
        }
    }

    /// Keeps a file for the core and returns its handle.
    pub fn keep_file(&self, file: web_sys::File) -> u64 {
        let handle = self.inner().next_handle.get();
        self.inner().next_handle.set(handle + 1);
        self.inner().files.borrow_mut().insert(handle, file);
        handle
    }

    pub fn take_file(&self, handle: u64) -> Option<web_sys::File> {
        self.inner().files.borrow_mut().remove(&handle)
    }

    /// Navigates within the app, like a router link.
    pub fn navigate(&self, url: &str, replace: bool) {
        if let Some(nav) = self.inner().navigate.borrow().as_ref() {
            nav(url, replace);
        }
    }

    pub fn register_replayer(&self, item_id: &str, replay: ReplayKeys) {
        self.inner()
            .replayers
            .borrow_mut()
            .insert(item_id.to_string(), replay);
    }

    pub fn unregister_replayer(&self, item_id: &str) {
        self.inner().replayers.borrow_mut().remove(item_id);
    }

    pub fn set_scroller(&self, scroller: Option<ScrollToItem>) {
        *self.inner().scroller.borrow_mut() = scroller;
    }

    /// The local time now, for events that stamp it.
    pub fn now_ms(&self) -> u64 {
        js_sys::Date::now() as u64
    }
}

/// True when `window.PETA_DEBUG` is set: events are logged to the console.
pub fn debug_enabled() -> bool {
    js_sys::Reflect::get(&web_sys::window().unwrap(), &"PETA_DEBUG".into())
        .map(|v| v.is_truthy())
        .unwrap_or(false)
}
