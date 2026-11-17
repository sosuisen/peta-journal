//! [`NativeShell`]: the core under Crux's `EffectRouter`, which sends each effect either to this
//! side or, as bytes, to the shell.

use crate::clock::Clock;
use crate::BorderApp;
use client_core::drag::{DragMove, ItemDrag};
use client_core::effects::{
    ApiOp, ApiOutput, ApiResult, NavigateOp, SseEvent, SseOp, TimerOp, TimerOutput, UiOp, UiOutput,
};
use client_core::keys::{KeyMap, KeyPlatform, KeyPress};
use client_core::rest::ClientId;
use client_core::{Effect, Event, Route};
use crux_core::bridge::Request as BorderRequest;
use crux_core::bridge::{BincodeFfiFormat, BridgeError, EffectId, FfiFormat};
use crux_core::capability::Operation;
use crux_core::effects::routes::Serialized;
use crux_core::effects::{EffectRouter, ResolveSink, Routes};
use crux_core::macros::effect;
use crux_core::render::RenderOperation;
use crux_core::{Core, Request};
use std::collections::{HashMap, VecDeque};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, Weak};
use std::thread;

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
}

/// The effects the shell performs. They cross as bytes.
#[effect(facet_typegen)]
pub enum ShellEffect {
    Render(RenderOperation),
    Ui(UiOp),
    Navigate(NavigateOp),
}

type Router = EffectRouter<BorderApp, Lanes>;

/// The way to the shell: bytes with a list of requests.
type ToShell = Arc<dyn Fn(Vec<u8>) + Send + Sync>;

/// The questions of the core to the user. The shell has one of them at a time: a dialog on
/// the screen is in the way of everything else, and so is a second one above it.
#[derive(Default)]
struct Questions {
    /// The number of the request with the question the shell has now.
    asked: Option<u32>,
    /// The questions behind it, oldest first.
    later: VecDeque<Request<UiOp>>,
}

/// True for an operation that asks the user and waits for the answer.
fn is_question(op: &UiOp) -> bool {
    matches!(
        op,
        UiOp::Confirm(_) | UiOp::ConfirmDestructive(_) | UiOp::Prompt(_)
    )
}

/// Where the effects go: over the border as bytes, or back to the core from this side.
#[derive(Clone)]
struct Lanes {
    router: Weak<Router>,
    border: Arc<Serialized<BorderApp, Lanes, BincodeFfiFormat>>,
    /// The timers that run, by the ID the core gave them.
    timers: Arc<Mutex<HashMap<u64, Request<TimerOp>>>>,
    /// The way to the one thread that asks the server, once there is such a thread.
    to_server: Arc<Mutex<Option<Sender<Request<ApiOp>>>>>,
    questions: Arc<Mutex<Questions>>,
}

impl Routes<BorderApp> for Lanes {
    fn new(router: Weak<Router>) -> Lanes {
        Lanes {
            border: Arc::new(Serialized::new(router.clone())),
            router,
            timers: Arc::default(),
            to_server: Arc::default(),
            questions: Arc::default(),
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

    /// Asks the server, one request at a time and in the order the core made them. With a
    /// thread for each request a later one overtook a slow one: a join reached the server
    /// before the save of the text just typed, and the text was lost.
    fn call<H: Host>(&self, host: &Arc<H>, request: Request<ApiOp>) {
        let mut to_server = self.to_server.lock().unwrap();
        let queue = to_server.get_or_insert_with(|| {
            let (queue, requests) = channel::<Request<ApiOp>>();
            // The thread must not keep the queue open itself: it ends when the shell is gone
            let (router, host) = (self.router.clone(), host.clone());
            thread::spawn(move || {
                for mut request in requests {
                    let result = host.call(request.operation.clone());
                    let Some(router) = router.upgrade() else {
                        return;
                    };
                    router
                        .resolve_request(&mut request, result)
                        .expect("the core waits for the answer");
                }
            });
            queue
        });
        // The thread only ends with the shell, and then nobody calls this
        let _ = queue.send(request);
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
        let mut questions = self.questions.lock().unwrap();
        if questions.asked.is_some() {
            questions.later.push_back(request);
        } else {
            questions.asked = Some(self.send(to_shell, ShellEffect::Ui(request)));
        }
    }

    /// The shell answered the request `id`. When that was its question, it gets the next one.
    fn answered(&self, to_shell: &ToShell, id: u32) {
        let mut questions = self.questions.lock().unwrap();
        if questions.asked != Some(id) {
            return;
        }
        let next = questions.later.pop_front();
        questions.asked = next.map(|request| self.send(to_shell, ShellEffect::Ui(request)));
    }

    fn time(&self, clock: &Arc<dyn Clock>, request: Request<TimerOp>) {
        match request.operation {
            TimerOp::Start { id, ms } => {
                self.timers.lock().unwrap().insert(id, request);
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
        let running = self.timers.lock().unwrap().remove(&id);
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
                Effect::Ui(request) => send(ShellEffect::Ui(request)),
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
        }
    }

    /// Takes an [`Event`] of the shell.
    pub fn update(&self, event: &[u8]) -> Result<(), BorderError> {
        self.router.routes.border.update(event)
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
        let (local, _) = self.clock.now();
        format!("{:04}-{:02}-{:02}", local.year, local.month, local.day)
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

    /// The whole [`ViewModel`](client_core::ViewModel), to draw after a render request.
    pub fn view(&self) -> Vec<u8> {
        let view = self.router.routes.border.view();
        view.expect("the view model is always written")
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
