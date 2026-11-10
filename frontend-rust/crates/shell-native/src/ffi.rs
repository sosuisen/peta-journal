//! The entrance for a shell in another language. BoltFFI makes the Swift and the C# side of it.

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
pub struct PetaCore;

#[boltffi::export]
impl PetaCore {
    /// Starts the core for the journal server at `server`, such as `http://localhost:8080`.
    pub fn new(server: String, shell: Arc<dyn PetaShell>) -> Self {
        let _ = (server, shell);
        PetaCore
    }

    /// Takes an event of the shell.
    pub fn update(&self, event: &[u8]) -> Result<(), String> {
        let _ = event;
        Ok(())
    }

    /// Takes the answer of the shell to the request `id`.
    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<(), String> {
        let _ = (id, output);
        Ok(())
    }

    /// The whole view model, to draw after a render request.
    pub fn view(&self) -> Vec<u8> {
        Vec::new()
    }
}
