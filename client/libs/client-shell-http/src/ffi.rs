//! The entrance for a shell in another language. BoltFFI makes the Swift and the C# side of it.

use crate::server::connect;
use client_native_bridge::NativeShell;
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

    /// Opens what the URL names, such as `/pages/notes` or `/today`.
    pub fn open(&self, url: String) -> Result<(), String> {
        let opened = self.shell.open(&url);
        opened.map_err(|nothing| format!("nothing to open at {:?}", nothing.0))
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

/// The HTML of a copy, from the AST that comes with the Markdown to copy (ADR 027). The links
/// to pages lead to `app_url`, where the web frontend is served.
#[boltffi::export]
pub fn copied_html(ast: String, app_url: String, tags_dir: String) -> String {
    client_native_bridge::copied_html(&ast, &app_url, &tags_dir)
}

/// The address on the journal server at `app_url` of a URL of the app, such as
/// `/pages/notes#item`, for the browser.
#[boltffi::export]
pub fn page_address(app_url: String, url: String) -> String {
    client_native_bridge::page_address(&app_url, &url)
}

/// The address on the journal server at `app_url` of a file under the Markdown folder, such
/// as `assets/picture.png`, for the browser and for loading an image.
#[boltffi::export]
pub fn asset_address(app_url: String, path: String) -> String {
    client_native_bridge::asset_address(&app_url, &path)
}

/// The characters before the place `units` UTF-16 units into `text`, as the core counts a
/// cursor (ADR 043). A place inside a surrogate pair goes after its character.
#[boltffi::export]
pub fn chars_of_utf16(text: String, units: u64) -> u64 {
    client_core::util::utf16::chars_of_utf16(&text, units as usize) as u64
}

/// The UTF-16 units before the place `chars` characters into `text`, for a text field of the
/// shell (ADR 043). A place past the end is the end.
#[boltffi::export]
pub fn utf16_of_chars(text: String, chars: u64) -> u64 {
    client_core::util::utf16::utf16_of_chars(&text, usize::try_from(chars).unwrap_or(usize::MAX))
        as u64
}
