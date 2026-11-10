//! [`NativeShell`]: the core under Crux's `EffectRouter`, which sends each effect either to this
//! side or, as bytes, to the shell.

use crux_core::bridge::{BincodeFfiFormat, BridgeError};
use crux_core::macros::effect;
use crux_core::render::RenderOperation;
use peta_core::effects::{ApiOp, ApiOutput, ApiResult};
use peta_core::rest::ClientId;

/// Why bytes could not cross the border.
pub type BorderError = BridgeError<BincodeFfiFormat>;

/// What this side needs from the world outside the process.
pub trait Host: Send + Sync + 'static {
    /// Performs one operation on the journal server. It may take as long as the server does.
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput>;
}

/// The effects the shell performs. They cross as bytes.
#[effect(typegen)]
pub enum ShellEffect {
    Render(RenderOperation),
}

pub struct NativeShell;

impl NativeShell {
    /// Starts the core. `to_shell` gets the requests for the shell, and may be called from any
    /// thread.
    pub fn new<H: Host>(
        _host: H,
        _client_id: ClientId,
        _to_shell: impl Fn(Vec<u8>) + Send + Sync + 'static,
    ) -> NativeShell {
        NativeShell
    }

    /// Takes an [`Event`](peta_core::Event) of the shell.
    pub fn update(&self, _event: &[u8]) -> Result<(), BorderError> {
        Ok(())
    }

    /// The whole [`ViewModel`](peta_core::ViewModel), to draw after a render request.
    pub fn view(&self) -> Vec<u8> {
        Vec::new()
    }
}
