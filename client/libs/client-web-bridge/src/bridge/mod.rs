//! The core and what it asks for, without the browser. The browser is behind [`Host`], so
//! that the tests run the same code as the web app.

use crate::body::HtmlBody;
use crate::border::{read, write};
use client_core::changes::{Changes, Shown};
use client_core::effects::{ApiOp, ApiOutput, ApiResult, Effect, LocalDateTimeDto};
use client_core::policy::InOrder;
use client_core::{Event, PetaApp, Route};
use crux_core::{Core, Request};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// What the browser calls once it has the answer.
pub type Done<T> = Box<dyn FnOnce(T)>;

/// What the browser does for the core.
pub trait Host: 'static {
    /// A file the browser holds, such as a pasted image.
    type File;

    /// The local time, and the milliseconds since the Unix epoch.
    fn now(&self) -> (LocalDateTimeDto, u64);

    /// Asks the server, and calls `done` with its answer.
    fn call(&self, op: ApiOp, done: Done<ApiResult<ApiOutput>>);

    /// Gives the shell an operation of the screen that only it can do, with its number. The
    /// shell answers with [`Bridge::resolve`].
    fn ask(&self, id: u32, op: &client_core::effects::UiOp);

    /// Tells the shell that the screen changed while it was not calling. It then takes the
    /// changes with [`Bridge::changes`].
    fn rendered(&self);
}

/// The core with the effects it asks for.
pub struct Bridge<H: Host> {
    core: Core<PetaApp<HtmlBody>>,
    host: H,
    me: Weak<Bridge<H>>,
    shown: RefCell<Shown<HtmlBody>>,
    to_server: RefCell<InOrder<Request<ApiOp>>>,
    /// How deep the calls of the shell are. The changes of a call go back as what it returns.
    calls: Cell<u32>,
    /// The screen changed since the shell last took the changes.
    changed: Cell<bool>,
}

impl<H: Host> Bridge<H> {
    pub fn new(host: H) -> Rc<Bridge<H>> {
        Rc::new_cyclic(|me| Bridge {
            core: Core::new(),
            host,
            me: me.clone(),
            shown: RefCell::default(),
            to_server: RefCell::default(),
            calls: Cell::new(0),
            changed: Cell::new(false),
        })
    }

    /// Shows what the URL names, such as `/pages/notes` or `/today`, and returns the changes.
    pub fn route(&self, url: &str) -> String {
        self.in_call(|| {
            if let Some(route) = Route::parse_url(url) {
                let today = self.host.now().0.day_name();
                self.send(Event::Route { route, today });
            }
        })
    }

    /// Takes an [`Event`] as JSON, and returns the changes.
    pub fn update(&self, event: &str) -> Result<String, String> {
        let event: Event = read(event)?;
        Ok(self.in_call(|| self.send(event)))
    }

    /// What changed on the screen since the last call, as JSON. The first call gives a reset.
    pub fn changes(&self) -> String {
        write(&self.take_changes())
    }

    fn take_changes(&self) -> Changes<HtmlBody> {
        self.changed.set(false);
        self.shown.borrow_mut().changes(self.core.view())
    }

    /// Does what the shell asks, and returns the changes it made.
    fn in_call(&self, call: impl FnOnce()) -> String {
        self.calls.set(self.calls.get() + 1);
        call();
        self.calls.set(self.calls.get() - 1);
        self.changes()
    }

    fn send(&self, event: Event) {
        let effects = self.core.process_event(event);
        self.process(effects);
    }

    fn answer<Op: crux_core::capability::Operation>(
        &self,
        request: &mut Request<Op>,
        output: Op::Output,
    ) {
        if let Ok(effects) = self.core.resolve(request, output) {
            self.process(effects);
        }
    }

    fn process(&self, effects: Vec<Effect>) {
        for effect in effects {
            match effect {
                Effect::Render(_) => self.changed.set(true),
                Effect::Api(request) => self.call(request),
                _ => {}
            }
        }
        if self.calls.get() == 0 && self.changed.get() {
            self.host.rendered();
        }
    }

    /// Asks the server, one request at a time and in the order the core made them
    /// (`client_core::policy`).
    fn call(&self, request: Request<ApiOp>) {
        if self.to_server.borrow_mut().push(request) {
            self.call_next();
        }
    }

    fn call_next(&self) {
        let next = self.to_server.borrow_mut().take_next();
        let Some(mut request) = next else {
            return;
        };
        let me = self.me.clone();
        let op = request.operation.clone();
        self.host.call(
            op,
            Box::new(move |output| {
                // The shell is gone, and the core with it
                let Some(me) = me.upgrade() else {
                    return;
                };
                me.answer(&mut request, output);
                me.call_next();
            }),
        );
    }
}

#[cfg(test)]
mod tests;
