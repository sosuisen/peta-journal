//! [`NativeShell`]: the core under Crux's `EffectRouter`, which sends each effect either to this
//! side or, as bytes, to the shell.

use crate::clock::Clock;
use client_core::effects::{
    ApiOp, ApiOutput, ApiResult, NavigateOp, SseEvent, SseOp, TimerOp, TimerOutput, UiOp, UiOutput,
};
use client_core::rest::ClientId;
use client_core::{Effect, Event, PetaApp, Route};
use crux_core::bridge::{BincodeFfiFormat, BridgeError, EffectId};
use crux_core::capability::Operation;
use crux_core::effects::routes::Serialized;
use crux_core::effects::{EffectRouter, ResolveSink, Routes};
use crux_core::macros::effect;
use crux_core::render::RenderOperation;
use crux_core::{Core, Request};
use std::collections::HashMap;
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

type Router = EffectRouter<PetaApp, Lanes>;

/// Where the effects go: over the border as bytes, or back to the core from this side.
#[derive(Clone)]
struct Lanes {
    router: Weak<Router>,
    border: Arc<Serialized<PetaApp, Lanes, BincodeFfiFormat>>,
    /// The timers that run, by the ID the core gave them.
    timers: Arc<Mutex<HashMap<u64, Request<TimerOp>>>>,
    /// The way to the one thread that asks the server, once there is such a thread.
    to_server: Arc<Mutex<Option<Sender<Request<ApiOp>>>>>,
}

impl Routes<PetaApp> for Lanes {
    fn new(router: Weak<Router>) -> Lanes {
        Lanes {
            border: Arc::new(Serialized::new(router.clone())),
            router,
            timers: Arc::default(),
            to_server: Arc::default(),
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
        let router = EffectRouter::new(Core::<PetaApp>::new(), |lanes: Lanes| {
            let send = {
                let border = lanes.border.clone();
                move |effect: ShellEffect| {
                    let bytes = border.serialize(effect);
                    to_shell(bytes.expect("a request is always written"));
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
                Effect::Ui(request) => send(ShellEffect::Ui(request)),
                Effect::Navigate(request) => send(ShellEffect::Navigate(request)),
            }
        });
        router.update(Event::Start { client_id });
        NativeShell { router, clock }
    }

    /// Takes an [`Event`] of the shell.
    pub fn update(&self, event: &[u8]) -> Result<(), BorderError> {
        self.router.routes.border.update(event)
    }

    /// Opens what the URL names, such as `/pages/notes` or `/today`. The core asks for this
    /// with a navigation, and the shell with the first page.
    pub fn open(&self, url: &str) -> Result<(), NothingThere> {
        let route = Route::parse_url(url).ok_or_else(|| NothingThere(url.to_string()))?;
        let (local, _) = self.clock.now();
        let today = format!("{:04}-{:02}-{:02}", local.year, local.month, local.day);
        self.router.update(Event::Route { route, today });
        Ok(())
    }

    /// Takes the answer of the shell to the request `id`: the output of its operation. A request
    /// that only tells the shell something, such as a render, has no answer.
    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<(), BorderError> {
        self.router.routes.border.resolve(EffectId(id), output)
    }

    /// The whole [`ViewModel`](client_core::ViewModel), to draw after a render request.
    pub fn view(&self) -> Vec<u8> {
        let view = self.router.routes.border.view();
        view.expect("the view model is always written")
    }
}
