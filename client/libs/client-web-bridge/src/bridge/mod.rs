//! The core and what it asks for, without the browser. The browser is behind [`Host`], so
//! that the tests run the same code as the web app.

use crate::body::HtmlBody;
use crate::border::{read, write};
use crate::lanes::{lane_of, Lane};
use client_core::changes::{Changes, Shown};
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, Effect, LocalDateTimeDto, SseEvent, SseOp, TimerOp,
    TimerOutput, UiOp, UiOutput,
};
use client_core::policy::{InOrder, Questions, Timers};
use client_core::rest::ClientId;
use client_core::{Event, PetaApp, Route};
use crux_core::{Core, Request, RequestHandle};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// The URL of the journal of today.
const TODAY: &str = "/today";

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

    /// Sends a file to the server as an asset, and calls `done` with the answer.
    fn upload(&self, file: Self::File, done: Done<ApiResult<ApiOutput>>);

    /// Opens the stream of the server's events in place of the one open, and calls `heard`
    /// with each of its events. After an error the stream stays closed.
    fn listen(&self, heard: Box<dyn FnMut(SseEvent)>);

    /// Starts the timer `id`, which calls `fired` after `ms` milliseconds.
    fn start_timer(&self, id: u64, ms: u32, fired: Done<()>);

    /// Stops the timer `id`, so that it never fires.
    fn cancel_timer(&self, id: u64);

    /// Gives the shell an operation of the screen that only it can do, with its number. The
    /// shell answers with [`Bridge::resolve`].
    fn ask(&self, id: u32, op: &UiOp);

    /// Puts the URL into the address bar and the history of the browser, or in place of the
    /// URL there when `replace`.
    fn go_to(&self, url: &str, replace: bool);

    /// Tells the shell that the screen changed while it was not calling. It then takes the
    /// changes with [`Bridge::changes`].
    fn rendered(&self);
}

/// The core with the effects it asks for.
pub struct Bridge<H: Host> {
    core: Core<PetaApp<HtmlBody>>,
    host: H,
    me: Weak<Bridge<H>>,
    client_id: ClientId,
    shown: RefCell<Shown<HtmlBody>>,
    to_server: RefCell<InOrder<Request<ApiOp>>>,
    timers: RefCell<Timers<Request<TimerOp>>>,
    /// The files pasted, by the number the core knows each by. A file cannot cross into it.
    files: RefCell<HashMap<u64, H::File>>,
    last_file: Cell<u64>,
    /// The requests the shell was given and has not answered yet, by their number.
    asked: RefCell<HashMap<u32, Request<UiOp>>>,
    last_asked: Cell<u32>,
    /// The questions for the shell: one at a time, as in every shell (`client_core::policy`).
    questions: RefCell<Questions<Request<UiOp>>>,
    /// How deep the calls of the shell are. The changes of a call go back as what it returns.
    calls: Cell<u32>,
    /// The screen changed since the shell last took the changes.
    changed: Cell<bool>,
}

impl<H: Host> Bridge<H> {
    /// Makes the core. The server knows the changes of this window by `client_id`: the
    /// browser sends it with each request.
    pub fn new(host: H, client_id: ClientId) -> Rc<Bridge<H>> {
        Rc::new_cyclic(|me| Bridge {
            core: Core::new(),
            host,
            me: me.clone(),
            client_id,
            shown: RefCell::default(),
            to_server: RefCell::default(),
            timers: RefCell::default(),
            files: RefCell::default(),
            last_file: Cell::new(0),
            asked: RefCell::default(),
            last_asked: Cell::new(0),
            questions: RefCell::default(),
            calls: Cell::new(0),
            changed: Cell::new(false),
        })
    }

    /// Starts the core, which connects to the server, and opens what the URL names. Returns
    /// the changes.
    pub fn start(&self, url: &str) -> String {
        self.in_call(|| {
            let client_id = self.client_id.clone();
            self.send(Event::Start { client_id });
            self.open(url);
        })
    }

    /// Shows what the URL names, such as `/pages/notes` or `/today`, and returns the changes.
    /// The browser calls this when its URL changes, as with its back button.
    pub fn route(&self, url: &str) -> String {
        self.in_call(|| self.open(url))
    }

    /// Goes to the URL as a link does: the browser keeps it in its history, and its page opens.
    fn go_to(&self, url: &str, replace: bool) {
        self.host.go_to(url, replace);
        self.open(url);
    }

    /// Opens the page the URL names. A URL that names nothing gives way to today.
    fn open(&self, url: &str) {
        match Route::parse_url(url) {
            Some(route) => {
                let today = self.host.now().0.day_name();
                self.send(Event::Route { route, today });
            }
            None => self.go_to(TODAY, true),
        }
    }

    /// Takes an [`Event`] as JSON, and returns the changes.
    pub fn update(&self, event: &str) -> Result<String, String> {
        let event: Event = read(event)?;
        Ok(self.in_call(|| self.send(event)))
    }

    /// Pastes the image in the file into the item, at `cursor` in characters. The file is
    /// saved among the assets, and its link is pasted once it is there (ADR 071).
    pub fn paste_image(&self, item_id: &str, file: H::File, cursor: usize) -> String {
        let number = self.last_file.get() + 1;
        self.last_file.set(number);
        self.files.borrow_mut().insert(number, file);
        self.in_call(|| {
            self.send(Event::PasteImage {
                item_id: item_id.to_string(),
                file: number,
                cursor,
            })
        })
    }

    /// Takes the answer of the shell to the request `id`: a [`UiOutput`] as JSON. When that
    /// was its question, the shell gets the next one. Returns the changes.
    pub fn resolve(&self, id: u32, output: &str) -> Result<String, String> {
        let output: UiOutput = read(output)?;
        let request = self.asked.borrow_mut().remove(&id);
        let mut request = request.ok_or_else(|| format!("No request {id} waits for an answer"))?;
        Ok(self.in_call(|| {
            // What the core asks because of this answer goes behind the questions that wait
            self.answer(&mut request, output);
            let mut given = None;
            self.questions
                .borrow_mut()
                .answered(id, |next| self.keep(next, &mut given));
            self.tell(given);
        }))
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
                Effect::Timer(request) => self.time(request),
                Effect::Sse(request) => self.listen(request),
                Effect::Ui(request) => self.ui(request),
                // A notification: the core does not wait for it
                Effect::Navigate(request) => {
                    let op = request.operation;
                    self.go_to(&op.url, op.replace);
                }
                _ => {}
            }
        }
        if self.calls.get() == 0 && self.changed.get() {
            self.host.rendered();
        }
    }

    fn listen(&self, mut request: Request<SseOp>) {
        let me = self.me.clone();
        self.host.listen(Box::new(move |event| {
            if let Some(me) = me.upgrade() {
                me.answer(&mut request, event);
            }
        }));
    }

    fn time(&self, request: Request<TimerOp>) {
        match request.operation {
            TimerOp::Start { id, ms } => {
                self.timers.borrow_mut().start(id, request);
                let me = self.me.clone();
                let fired = Box::new(move |()| {
                    if let Some(me) = me.upgrade() {
                        me.end_timer(id, TimerOutput::Fired);
                    }
                });
                self.host.start_timer(id, ms, fired);
            }
            // A notification: the core waits for the answer of the timer, not of this
            TimerOp::Cancel { id } => {
                self.host.cancel_timer(id);
                self.end_timer(id, TimerOutput::Cancelled);
            }
        }
    }

    /// Gives the core the end of the timer, unless it has ended already.
    fn end_timer(&self, id: u64, output: TimerOutput) {
        let running = self.timers.borrow_mut().end(id);
        if let Some(mut request) = running {
            self.answer(&mut request, output);
        }
    }

    fn ui(&self, request: Request<UiOp>) {
        let mut given = None;
        match lane_of(&request.operation) {
            Lane::Shell => {
                self.keep(request, &mut given);
            }
            Lane::Question => self
                .questions
                .borrow_mut()
                .ask(request, |question| self.keep(question, &mut given)),
            Lane::Wasm => {}
        }
        self.tell(given);
    }

    /// Numbers a request for the shell, and keeps it until the shell answers. A notification
    /// has no answer, so it is not kept.
    fn keep(&self, request: Request<UiOp>, given: &mut Option<(u32, UiOp)>) -> u32 {
        let id = self.last_asked.get() + 1;
        self.last_asked.set(id);
        *given = Some((id, request.operation.clone()));
        if !matches!(request.handle, RequestHandle::Never) {
            self.asked.borrow_mut().insert(id, request);
        }
        id
    }

    /// Gives the shell the request kept. Not while the questions are borrowed: the shell may
    /// answer at once.
    fn tell(&self, given: Option<(u32, UiOp)>) {
        if let Some((id, op)) = given {
            self.host.ask(id, &op);
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
        let op = request.operation.clone();
        let me = self.me.clone();
        let done: Done<ApiResult<ApiOutput>> = Box::new(move |output| {
            // The shell is gone, and the core with it
            let Some(me) = me.upgrade() else {
                return;
            };
            me.answer(&mut request, output);
            me.call_next();
        });
        match op {
            ApiOp::UploadAsset { file: number } => {
                // Taken out first: the answer may come at once, and paste another file
                let file = self.files.borrow_mut().remove(&number);
                match file {
                    Some(file) => self.host.upload(file, done),
                    None => done(Err(ApiError(format!("No pasted file {number}")))),
                }
            }
            op => self.host.call(op, done),
        }
    }
}

#[cfg(test)]
mod tests;
