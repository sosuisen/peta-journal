//! [`NativeShell`]: the core under Crux's `EffectRouter`, which sends each effect either to this
//! side or, as bytes, to the shell.

use crate::clock::Clock;
use crate::BorderApp;
use crate::Changes;
use client_core::changes::Shown;
use client_core::drag::{DragMove, ItemDrag};
use client_core::effects::{
    ApiError, ApiOp, ApiOutput, ApiResult, NavigateOp, SseEvent, SseOp, TimerOp, TimerOutput, UiOp,
    UiOutput,
};
use client_core::keys::{KeyMap, KeyPlatform, KeyPress};
use client_core::policy::{is_question, InOrder, Questions, Timers};
use client_core::rest::ClientId;
use client_core::typing::{self, Typing};
use client_core::{Effect, Event, RichText, Route, ViewModel};
use crux_core::bridge::Request as BorderRequest;
use crux_core::bridge::{BincodeFfiFormat, BridgeError, EffectId, FfiFormat};
use crux_core::capability::Operation;
use crux_core::effects::routes::Serialized;
use crux_core::effects::{EffectRouter, ResolveSink, Routes};
use crux_core::macros::effect;
use crux_core::render::RenderOperation;
use crux_core::{Core, Request};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::thread;
use std::time::{Duration, Instant};

/// Why bytes could not cross the border.
pub type BorderError = BridgeError<BincodeFfiFormat>;

/// The URL names nothing that can be opened.
#[derive(Debug, PartialEq, Eq)]
pub struct NothingThere(pub String);

/// The way to the journal. The time is not here: see [`Clock`] (ADR 036).
pub trait Host: Send + Sync + 'static {
    /// Performs one operation on the journal server. It may take as long as the server does.
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput>;
    /// Reads the event stream of the server, and returns when it has ended.
    fn events(&self, send: &mut dyn FnMut(SseEvent));
    /// Saves the file at `path` among the assets of the journal, for an image the shell
    /// pasted (ADR 071). A journal that takes no files refuses.
    fn upload(&self, _path: &Path) -> ApiResult<ApiOutput> {
        Err(ApiError("this journal takes no files".into()))
    }
    /// The address of the journal server, such as `http://localhost:8080`, where the browser
    /// opens a page of the web frontend and a file of the assets folder. None for a journal
    /// inside the app, which has no server.
    fn app_url(&self) -> Option<String> {
        None
    }
}

/// Puts the address of the server before an address of the server that the core orders to
/// open in the browser (ADR 083). The core does not know the server.
fn on_the_server(op: &mut UiOp, app_url: Option<String>) {
    if let (UiOp::OpenInBrowser { address }, Some(app_url)) = (op, app_url) {
        if address.starts_with('/') {
            *address = format!("{}{address}", app_url.trim_end_matches('/'));
        }
    }
}

/// The files of the images the shell pasted, by the number the core knows each by: a file
/// does not cross into the core (ADR 071).
#[derive(Default)]
struct PastedFiles {
    last: u64,
    paths: HashMap<u64, PathBuf>,
}

impl PastedFiles {
    /// Keeps the path, and gives the number of its file.
    fn add(&mut self, path: &Path) -> u64 {
        self.last += 1;
        self.paths.insert(self.last, path.to_path_buf());
        self.last
    }
}

/// The effects the shell performs. They cross as bytes.
#[effect(facet_typegen)]
pub enum ShellEffect {
    Render(RenderOperation),
    Ui(UiOp),
    Navigate(NavigateOp),
}

type Router = EffectRouter<BorderApp, Lanes>;

/// How long closing waits for the server to save what the shell typed, as Emacs does.
pub const CLOSE_LIMIT: Duration = Duration::from_secs(5);

/// The way to the shell: bytes with a list of requests.
type ToShell = Arc<dyn Fn(Vec<u8>) + Send + Sync>;

/// Where the effects go: over the border as bytes, or back to the core from this side.
#[derive(Clone)]
struct Lanes {
    router: Weak<Router>,
    border: Arc<Serialized<BorderApp, Lanes, BincodeFfiFormat>>,
    /// The timers that run, by the ID the core gave them.
    timers: Arc<Mutex<Timers<Request<TimerOp>>>>,
    /// The requests to the server, each waiting for the answer to the one before.
    to_server: Arc<Mutex<InOrder<Request<ApiOp>>>>,
    /// The questions of the core to the user. The shell has one of them at a time
    /// (`client_core::policy`).
    questions: Arc<Mutex<Questions<Request<UiOp>>>>,
    pasted: Arc<Mutex<PastedFiles>>,
}

impl Routes<BorderApp> for Lanes {
    fn new(router: Weak<Router>) -> Lanes {
        Lanes {
            border: Arc::new(Serialized::new(router.clone())),
            router,
            timers: Arc::default(),
            to_server: Arc::default(),
            questions: Arc::default(),
            pasted: Arc::default(),
        }
    }
}

impl Lanes {
    /// Gives the core the answer of this side.
    fn answer<Op: Operation>(&self, request: &mut Request<Op>, output: Op::Output) {
        // The shell is gone, and the core with it
        let Some(router) = self.router.upgrade() else {
            return;
        };
        router
            .resolve_request(request, output)
            .expect("the core waits for the answer");
    }

    /// Asks the server, one request at a time and in the order the core made them
    /// (`client_core::policy`). With a thread for each request a later one overtook a slow
    /// one: a join reached the server before the save of the text just typed, and the text
    /// was lost.
    fn call<H: Host>(&self, host: &Arc<H>, request: Request<ApiOp>) {
        if !self.to_server.lock().unwrap().push(request) {
            return;
        }
        // One thread asks until nothing waits. The next request that comes starts another
        let (lanes, host) = (self.clone(), host.clone());
        thread::spawn(move || {
            // The lock is not held while the server is asked: the core makes requests then
            let next = || lanes.to_server.lock().unwrap().take_next();
            while let Some(mut request) = next() {
                let result = lanes.perform(&*host, request.operation.clone());
                // The shell is gone, and the core with it
                let Some(router) = lanes.router.upgrade() else {
                    return;
                };
                router
                    .resolve_request(&mut request, result)
                    .expect("the core waits for the answer");
            }
        });
    }

    /// Does one operation on the server. An upload names a file by the number the core knows
    /// it by, and the server gets its path.
    fn perform<H: Host>(&self, host: &H, op: ApiOp) -> ApiResult<ApiOutput> {
        let ApiOp::UploadAsset { file } = op else {
            return host.call(op);
        };
        let path = self.pasted.lock().unwrap().paths.remove(&file);
        let Some(path) = path else {
            return Err(ApiError(format!("No pasted file {file}")));
        };
        let uploaded = host.upload(&path);
        // The file was only a way across: kept after a failure, it would never be deleted
        let _ = std::fs::remove_file(&path);
        uploaded
    }

    /// Sends a request over the border, and returns its number.
    fn send(&self, to_shell: &ToShell, effect: ShellEffect) -> u32 {
        let bytes = self.border.serialize(effect);
        let bytes = bytes.expect("a request is always written");
        let sent: Vec<BorderRequest<ShellEffectFfi>> =
            BincodeFfiFormat::deserialize(&bytes).expect("what was written can be read");
        to_shell(bytes);
        sent[0].id.0
    }

    /// Gives the shell a question, or keeps it until the questions before it are answered.
    fn ask(&self, to_shell: &ToShell, request: Request<UiOp>) {
        let give = |request| self.send(to_shell, ShellEffect::Ui(request));
        self.questions.lock().unwrap().ask(request, give);
    }

    /// The shell answered the request `id`. When that was its question, it gets the next one.
    fn answered(&self, to_shell: &ToShell, id: u32) {
        let give = |request| self.send(to_shell, ShellEffect::Ui(request));
        self.questions.lock().unwrap().answered(id, give);
    }

    fn time(&self, clock: &Arc<dyn Clock>, request: Request<TimerOp>) {
        match request.operation {
            TimerOp::Start { id, ms } => {
                self.timers.lock().unwrap().start(id, request);
                let (lanes, clock) = (self.clone(), clock.clone());
                thread::spawn(move || {
                    clock.sleep(ms);
                    lanes.end_timer(id, TimerOutput::Fired);
                });
            }
            // A notification: the core waits for the answer of the timer, not of this
            TimerOp::Cancel { id } => self.end_timer(id, TimerOutput::Cancelled),
        }
    }

    /// Answers the timer, unless it has ended another way already.
    fn end_timer(&self, id: u64, output: TimerOutput) {
        let running = self.timers.lock().unwrap().end(id);
        if let Some(mut request) = running {
            self.answer(&mut request, output);
        }
    }

    /// Reads the event stream. The core hears of every event through the one request.
    fn listen<H: Host>(&self, host: &Arc<H>, mut request: Request<SseOp>) {
        let (lanes, host) = (self.clone(), host.clone());
        thread::spawn(move || {
            host.events(&mut |event| lanes.answer(&mut request, event));
        });
    }
}

/// The core for one shell. The shell may call it from any thread.
pub struct NativeShell {
    router: Arc<Router>,
    clock: Arc<dyn Clock>,
    to_shell: ToShell,
    keys: KeyMap,
    /// The drag of the mouse that started on the text of an item, if one is on.
    drag: Mutex<ItemDrag>,
    /// What the shell was last told by [`NativeShell::changes`].
    shown: Mutex<Shown<RichText>>,
}

impl NativeShell {
    /// Starts the core. `to_shell` gets the requests for the shell: each call has a list of
    /// requests, each with its number and a [`ShellEffectFfi`]. It may be called from any thread,
    /// and for the first time before this returns.
    pub fn new<H: Host>(
        host: H,
        clock: impl Clock,
        client_id: ClientId,
        to_shell: impl Fn(Vec<u8>) + Send + Sync + 'static,
    ) -> NativeShell {
        let host = Arc::new(host);
        let clock: Arc<dyn Clock> = Arc::new(clock);
        let of_effects = clock.clone();
        let to_shell: ToShell = Arc::new(to_shell);
        let of_router = to_shell.clone();
        let router = EffectRouter::new(Core::<BorderApp>::new(), |lanes: Lanes| {
            let send = {
                let lanes = lanes.clone();
                let to_shell = of_router.clone();
                move |effect: ShellEffect| {
                    lanes.send(&to_shell, effect);
                }
            };
            move |effect| match effect {
                Effect::Api(request) => lanes.call(&host, request),
                Effect::Sse(request) => lanes.listen(&host, request),
                Effect::Timer(request) => lanes.time(&of_effects, request),
                Effect::Render(request) => send(ShellEffect::Render(request)),
                Effect::Ui(mut request) if request.operation == UiOp::LocalNow => {
                    let (local, now_ms) = of_effects.now();
                    lanes.answer(&mut request, UiOutput::Now { local, now_ms });
                }
                Effect::Ui(request) if is_question(&request.operation) => {
                    lanes.ask(&of_router, request);
                }
                Effect::Ui(mut request) => {
                    on_the_server(&mut request.operation, host.app_url());
                    send(ShellEffect::Ui(request))
                }
                Effect::Navigate(request) => send(ShellEffect::Navigate(request)),
            }
        });
        router.update(Event::Start { client_id });
        NativeShell {
            router,
            clock,
            to_shell,
            keys: KeyMap::of(key_platform()),
            drag: Mutex::new(ItemDrag::default()),
            shown: Mutex::default(),
        }
    }

    /// Takes an [`Event`] of the shell.
    pub fn update(&self, event: &[u8]) -> Result<(), BorderError> {
        self.router.routes.border.update(event)
    }

    /// Pastes the image in the file at `path` into the item, at `cursor` in characters. The
    /// file is saved among the assets, and its link is pasted once it is there (ADR 071).
    pub fn paste_image(&self, item_id: &str, path: &Path, cursor: usize) {
        let file = self.router.routes.pasted.lock().unwrap().add(path);
        self.router.update(Event::PasteImage {
            item_id: item_id.to_string(),
            file,
            cursor,
        });
    }

    /// Takes a key of the shell: a [`KeyPress`] as bytes (ADR 052). False when the key means
    /// nothing to the outline there, and the shell leaves it to its editor.
    pub fn press(&self, key: &[u8]) -> Result<bool, BorderError> {
        let press: KeyPress =
            BincodeFfiFormat::deserialize(key).map_err(BridgeError::DeserializeEvent)?;
        let (_, now_ms) = self.clock.now();
        let Some(event) = self.keys.event_of(&press, now_ms) else {
            return Ok(false);
        };
        self.router.update(event);
        Ok(true)
    }

    /// Takes a change of the Markdown of an item in its text field: a [`Typing`] as bytes. The
    /// time is that of the clock here, so that no shell reads its own.
    pub fn typed(&self, typing: &[u8]) -> Result<(), BorderError> {
        let typing: Typing =
            BincodeFfiFormat::deserialize(typing).map_err(BridgeError::DeserializeEvent)?;
        let (_, now_ms) = self.clock.now();
        self.router.update(typing::event_of(typing, now_ms));
        Ok(())
    }

    /// Takes a step of a drag of the mouse: a [`DragMove`] as bytes. True when it told the core
    /// something, as that the drag left the text and selects items; the shell then draws the
    /// view again.
    pub fn drag(&self, step: &[u8]) -> Result<bool, BorderError> {
        let step: DragMove =
            BincodeFfiFormat::deserialize(step).map_err(BridgeError::DeserializeEvent)?;
        let events = self.drag.lock().unwrap().events_of(step);
        let told = !events.is_empty();
        for event in events {
            self.router.update(event);
        }
        Ok(told)
    }

    /// Opens what the URL names, such as `/pages/notes` or `/today`, and keeps it in the
    /// history of the core (ADR 035). The core asks for this with a navigation, and the shell
    /// with the first page, with today and with a link.
    pub fn open(&self, url: &str) -> Result<(), NothingThere> {
        Route::parse_url(url).ok_or_else(|| NothingThere(url.to_string()))?;
        let url = url.to_string();
        self.router.update(Event::Open {
            url,
            today: self.today(),
        });
        Ok(())
    }

    /// Back to the screen that was open before, as the core keeps the history (ADR 035).
    /// Nothing happens when there is none.
    pub fn back(&self) {
        self.router.update(Event::Back {
            today: self.today(),
        });
    }

    /// Forward to the screen that was open before [`NativeShell::back`].
    pub fn forward(&self) {
        self.router.update(Event::Forward {
            today: self.today(),
        });
    }

    /// The local date of the clock, `YYYY-MM-DD`, for the journal of today.
    fn today(&self) -> String {
        self.clock.now().0.day_name()
    }

    /// Takes the answer of the shell to the request `id`: the output of its operation. A request
    /// that only tells the shell something, such as a render, has no answer.
    ///
    /// The shell has one question of the core at a time (`UiOp::Confirm`,
    /// `UiOp::ConfirmDestructive`, `UiOp::Prompt`). The next one comes after this answer.
    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<(), BorderError> {
        let routes = &self.router.routes;
        // What the core asks because of this answer goes behind the questions that wait
        routes.border.resolve(EffectId(id), output)?;
        routes.answered(&self.to_shell, id);
        Ok(())
    }

    /// The shell closes: the edit ends, so that the text typed and still waiting for its save
    /// is saved now (#766). Returns once the server has answered every request, or after
    /// [`CLOSE_LIMIT`]. A timer would never fire after the shell is gone.
    pub fn close(&self) {
        if let Some(item_id) = self.router.view().outline.focused_item_id {
            self.router.update(Event::EditFinished { item_id });
        }
        let deadline = Instant::now() + CLOSE_LIMIT;
        while !self.router.routes.to_server.lock().unwrap().is_idle() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// The URL on screen, as the history of the core has it (ADR 035). None while nothing was
    /// opened.
    pub fn url(&self) -> Option<String> {
        self.router.view().history.url
    }

    /// The whole [`ViewModel`](client_core::ViewModel), as bytes. Only tests read it: a shell
    /// keeps its view model from [`NativeShell::changes`], and the two are not to be mixed
    /// (ADR 076).
    #[cfg(any(test, feature = "test-support"))]
    pub fn view(&self) -> Vec<u8> {
        let view = self.router.routes.border.view();
        view.expect("the view model is always written")
    }

    /// What changed on the screen since the last call, as bytes of [`Changes`]: the changes
    /// to the rows of the outline, and the panels that changed (ADR 076). The first call gives
    /// a reset of the rows and every panel. The shell calls this from its main thread only,
    /// and draws from nothing else: what this keeps must be what the shell has.
    pub fn changes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        BincodeFfiFormat::serialize(&mut bytes, &self.take_changes())
            .expect("the changes are always written");
        bytes
    }

    /// What changed since the last call, before it is written as bytes.
    pub(crate) fn take_changes(&self) -> Changes {
        self.changes_from(self.whole_view())
    }

    /// The whole view model, as the core builds it from its model.
    pub(crate) fn whole_view(&self) -> ViewModel<RichText> {
        self.router.view()
    }

    /// What changed from what the shell has to `view`. The shell has `view` after this.
    pub(crate) fn changes_from(&self, view: ViewModel<RichText>) -> Changes {
        self.shown.lock().unwrap().changes(view)
    }
}

/// The keys of the platform the shell runs on. A Mac uses Command where Windows uses Control;
/// iOS and Android read their keys themselves, so any map does for them.
fn key_platform() -> KeyPlatform {
    if cfg!(target_os = "macos") {
        KeyPlatform::Mac
    } else {
        KeyPlatform::Windows
    }
}
