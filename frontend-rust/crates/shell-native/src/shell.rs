//! [`NativeShell`]: the core under Crux's `EffectRouter`, which sends each effect either to this
//! side or, as bytes, to the shell.

use crux_core::bridge::{BincodeFfiFormat, BridgeError};
use crux_core::capability::Operation;
use crux_core::effects::routes::Serialized;
use crux_core::effects::{EffectRouter, ResolveSink, Routes};
use crux_core::macros::effect;
use crux_core::render::RenderOperation;
use crux_core::{Core, Request};
use peta_core::effects::{ApiOp, ApiOutput, ApiResult, SseEvent, UiOp};
use peta_core::rest::ClientId;
use peta_core::{Effect, Event, PetaApp};
use std::sync::{Arc, Weak};
use std::thread;

/// Why bytes could not cross the border.
pub type BorderError = BridgeError<BincodeFfiFormat>;

/// What this side needs from the world outside the process.
pub trait Host: Send + Sync + 'static {
    /// Performs one operation on the journal server. It may take as long as the server does.
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput>;
    /// Reads the event stream of the server, and returns when it has ended.
    fn events(&self, send: &mut dyn FnMut(SseEvent));
}

/// The effects the shell performs. They cross as bytes.
#[effect(typegen)]
pub enum ShellEffect {
    Render(RenderOperation),
    Ui(UiOp),
}

type Router = EffectRouter<PetaApp, Lanes>;

/// Where the effects go: over the border as bytes, or back to the core from this side.
#[derive(Clone)]
struct Lanes {
    router: Weak<Router>,
    border: Arc<Serialized<PetaApp, Lanes, BincodeFfiFormat>>,
}

impl Routes<PetaApp> for Lanes {
    fn new(router: Weak<Router>) -> Lanes {
        Lanes {
            border: Arc::new(Serialized::new(router.clone())),
            router,
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

    fn call<H: Host>(&self, host: &Arc<H>, mut request: Request<ApiOp>) {
        let (lanes, host) = (self.clone(), host.clone());
        thread::spawn(move || {
            let result = host.call(request.operation.clone());
            lanes.answer(&mut request, result);
        });
    }
}

pub struct NativeShell {
    router: Arc<Router>,
}

impl NativeShell {
    /// Starts the core. `to_shell` gets the requests for the shell, and may be called from any
    /// thread.
    pub fn new<H: Host>(
        host: H,
        client_id: ClientId,
        to_shell: impl Fn(Vec<u8>) + Send + Sync + 'static,
    ) -> NativeShell {
        let host = Arc::new(host);
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
                Effect::Render(request) => send(ShellEffect::Render(request)),
                Effect::Ui(request) => send(ShellEffect::Ui(request)),
                _ => {}
            }
        });
        router.update(Event::Start { client_id });
        NativeShell { router }
    }

    /// Takes an [`Event`] of the shell.
    pub fn update(&self, event: &[u8]) -> Result<(), BorderError> {
        self.router.routes.border.update(event)
    }

    /// The whole [`ViewModel`](peta_core::ViewModel), to draw after a render request.
    pub fn view(&self) -> Vec<u8> {
        let view = self.router.routes.border.view();
        view.expect("the view model is always written")
    }
}
