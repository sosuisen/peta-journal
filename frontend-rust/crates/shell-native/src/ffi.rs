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

/// The HTML of an item, from its AST as the JSON that the view model carries (ADR 025). It
/// needs no core: a shell asks for the items it shows.
#[boltffi::export]
pub fn item_html(ast: String) -> Result<String, String> {
    crate::item_html(&ast)
}

/// The HTML of a copy, from the AST that comes with the Markdown to copy (ADR 027). The links
/// to pages lead to `app_url`, where the web frontend is served.
#[boltffi::export]
pub fn copied_html(ast: String, app_url: String, tags_dir: String) -> String {
    crate::copied_html(&ast, &app_url, &tags_dir)
}

/// The URL that opens the page of a wikilink in the web frontend, served at `app_url`. A
/// native shell opens the links of its items there.
#[boltffi::export]
pub fn wikilink_url(app_url: String, page: String) -> String {
    crate::wikilink_url(&app_url, &page)
}

/// The URL that opens the page of a tag in the web frontend.
#[boltffi::export]
pub fn hashtag_url(app_url: String, tags_dir: String, tag: String) -> String {
    crate::hashtag_url(&app_url, &tags_dir, &tag)
}
