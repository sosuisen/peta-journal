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

    /// Takes an image pasted into the item `item_id` at `cursor`, in the characters of the
    /// core. The shell wrote it to the file at `path`, which the core deletes once the server
    /// has answered (ADR 071).
    pub fn paste_image(&self, item_id: String, path: String, cursor: u64) {
        let cursor = usize::try_from(cursor).unwrap_or(usize::MAX);
        self.shell
            .paste_image(&item_id, std::path::Path::new(&path), cursor);
    }

    /// Takes a key of the shell: a `KeyPress` as bytes (ADR 052). False when the key means
    /// nothing to the outline there, and the shell leaves it to its editor.
    pub fn press(&self, key: &[u8]) -> Result<bool, String> {
        self.shell.press(key).map_err(|e| e.to_string())
    }

    /// Takes a change of the Markdown of an item in its text field: a `Typing` as bytes. The
    /// core adds the time.
    pub fn typed(&self, typing: &[u8]) -> Result<(), String> {
        self.shell.typed(typing).map_err(|e| e.to_string())
    }

    /// Takes a step of a drag of the mouse that started on the text of an item: a `DragMove`
    /// as bytes. True when it told the core something, and the shell draws the view again.
    pub fn drag(&self, step: &[u8]) -> Result<bool, String> {
        self.shell.drag(step).map_err(|e| e.to_string())
    }

    /// Takes the answer of the shell to the request `id`: the output of its operation.
    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<(), String> {
        self.shell.resolve(id, output).map_err(|e| e.to_string())
    }

    /// The shell closes, as a sticky that is closed or an app that ends: the edit ends, and
    /// the text typed and still waiting for its save is saved before this returns (#766).
    pub fn close(&self) {
        self.shell.close();
    }

    /// What changed on the screen since the last call, as bytes of `Changes`: the changes to
    /// the rows of the outline, and the panels that changed (ADR 076). The first call gives a
    /// reset of the rows and every panel. Call it from the main thread only.
    pub fn changes(&self) -> Vec<u8> {
        self.shell.changes()
    }
}

/// The HTML of a copy, from the AST that comes with the Markdown to copy (ADR 027). The links
/// to pages lead to `app_url`, where the web frontend is served.
#[boltffi::export]
pub fn copied_html(ast: String, app_url: String, tags_dir: String) -> String {
    client_native_bridge::copied_html(&ast, &app_url, &tags_dir)
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

/// How far the list scrolls in one step of a drag selection with the mouse at `y`, for a list
/// that can be seen from `top` to `bottom`. Below zero is up; zero when the mouse is not in the
/// band at an edge.
#[boltffi::export]
pub fn edge_step(y: f64, top: f64, bottom: f64) -> f64 {
    client_core::drag::edge_step(y, top, bottom)
}

/// How long one step of that scroll takes, in milliseconds.
#[boltffi::export]
pub fn edge_step_ms() -> u32 {
    client_core::drag::EDGE_STEP_MS
}

/// Where to look for the item under the mouse at `y` in a drag, in a list that can be seen from
/// `top` to `bottom`: a place beyond the list counts as the nearest place in it.
#[boltffi::export]
pub fn place_in_list(y: f64, top: f64, bottom: f64) -> f64 {
    client_core::drag::place_in_list(y, top, bottom)
}
