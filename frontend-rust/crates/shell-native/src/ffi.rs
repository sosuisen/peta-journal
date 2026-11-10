//! The entrance for a shell in another language. BoltFFI makes the Swift and the C# side of it.

use crate::server::connect;
use crate::shell::NativeShell;
use std::sync::Arc;

/// What the shell gives the core, to hear from it.
#[boltffi::export]
pub trait PetaShell: Send + Sync {
    /// Takes requests for the shell: a list, each with its number and its effect. It may be
    /// called from any thread.
    fn process_effects(&self, bytes: Vec<u8>);
}

/// The core for one shell.
#[boltffi::export]
pub struct PetaCore {
    shell: NativeShell,
}

#[boltffi::export]
impl PetaCore {
    /// Starts the core for the journal server at `server`, such as `http://localhost:8080`.
    /// The shell may get its first requests before this returns.
    pub fn new(server: String, shell: Arc<dyn PetaShell>) -> Self {
        PetaCore {
            shell: connect(server, move |bytes| shell.process_effects(bytes)),
        }
    }

    /// Takes an event of the shell. The error says why the bytes could not be read.
    pub fn update(&self, event: &[u8]) -> Result<(), String> {
        self.shell.update(event).map_err(|e| e.to_string())
    }

    /// Takes the answer of the shell to the request `id`: the output of its operation.
    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<(), String> {
        self.shell.resolve(id, output).map_err(|e| e.to_string())
    }

    /// The whole view model, to draw after a render request.
    pub fn view(&self) -> Vec<u8> {
        self.shell.view()
    }
}
