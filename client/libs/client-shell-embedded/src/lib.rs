//! The core for the Android and the iOS shell: the Crux core, and a journal core inside the app instead
//! of a journal server (ADR 029, ADR 034). BoltFFI makes the Kotlin and the Swift side of the entrance.

boltffi::scaffolding!();

mod cloud;
mod screen;

pub use cloud::{
    PetaCloudSync, PetaDropboxEntry, PetaDropboxError, PetaDropboxFile, PetaDropboxSdk,
    PetaListing, PetaSyncHost, PetaSyncState,
};
pub use screen::PetaScreen;

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use client_api_embedded::{EmbeddedJournal, EventQueue};
use client_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, SseEvent};
use client_core::rest::ClientId;
use client_core::screens::page::split_doc_path;
use client_core::util::filename::is_journal_file_name;
use client_core::Route;
use client_native_bridge::{Clock, Host, NativeShell, SystemClock};
use journal_core::assembly::{CoreHost, CoreSettings, JournalCore};
use journal_core::infrastructure::export::export_queue_repository::ExportQueueRepository;
use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use journal_core::journal::FolderWatch;

/// The app has no watcher, and it asks nothing about missing files: `PetaCore::sync` deletes
/// their pages (ADR 030).
struct Quiet;

impl MissingFilesPrompt for Quiet {
    fn notify_new_missing(&self) {}
}

impl FolderWatch for Quiet {
    fn restart(&self) {}
}

/// The way to the journal of the app. It closes with the core: the threads of the shell may
/// live a little longer, and they must not keep the database and the index open.
struct Door {
    journal: RwLock<Option<EmbeddedJournal>>,
    events: Arc<EventQueue>,
    /// The pages that changed in the app and that their files do not hold yet.
    exports: ExportQueueRepository,
    /// Told when a page changed in the app: the sync starts a moment later (ADR 055).
    on_change: RwLock<Option<Box<dyn Fn() + Send + Sync>>>,
    /// Held by each operation for reading, and by the take-in of the downloads for writing.
    take_in: RwLock<()>,
}

impl Door {
    /// Does what the frontend core asks of the journal. When the operation put a page in the
    /// queue of the pages to write to their files, a page changed in the app, whatever the
    /// operation was (ADR 055). A page that the take-in of the downloads puts in the queue
    /// does not count: no operation runs while the take-in runs, so none can take its pages
    /// for its own (#1166).
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        let _apart = self.take_in.read().unwrap();
        let before = self.exports.last_queued_at().unwrap_or(None);
        let done = match &*self.journal.read().unwrap() {
            Some(journal) => journal.call(op),
            None => return Err(ApiError("the journal is closed".into())),
        };
        if self.exports.last_queued_at().unwrap_or(None) > before {
            if let Some(tell) = &*self.on_change.read().unwrap() {
                tell();
            }
        }
        done
    }

    /// Lets go of the journal. It waits for an operation that runs to end.
    fn close(&self) {
        self.events.close();
        self.journal.write().unwrap().take();
    }
}

struct Embedded {
    door: Arc<Door>,
}

impl Host for Embedded {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        self.door.call(op)
    }

    fn events(&self, send: &mut dyn FnMut(SseEvent)) {
        self.door.events.read(send);
    }

    fn upload(&self, path: &std::path::Path) -> ApiResult<ApiOutput> {
        match &*self.door.journal.read().unwrap() {
            Some(journal) => journal.upload(path),
            None => Err(ApiError("the journal is closed".into())),
        }
    }
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

#[boltffi::export]
pub trait PetaShell: Send + Sync {
    /// Takes requests for the shell: a list, each with its number and its effect. It may be
    /// called from any thread.
    fn process_effects(&self, bytes: Vec<u8>);
}

/// The core for one shell, with the journal of the app.
#[boltffi::export]
pub struct PetaCore {
    shell: NativeShell,
    core: JournalCore,
    door: Arc<Door>,
    /// The file with the URL of the screen opened last, for the next time the app opens.
    last: PathBuf,
    /// The folder the files from Dropbox are put in.
    markdown: PathBuf,
    /// What the screen tells besides the view model (ADR 056).
    screen: Mutex<PetaScreen>,
}

impl Drop for PetaCore {
    fn drop(&mut self) {
        self.door.close();
    }
}

#[boltffi::export]
impl PetaCore {
    /// Opens the journal of the app. `data_dir` keeps the database, the index and the
    /// settings. `markdown_dir` is the folder the files from Dropbox are put in.
    pub fn new(
        data_dir: String,
        markdown_dir: String,
        shell: Arc<dyn PetaShell>,
    ) -> Result<Self, String> {
        let data_dir = PathBuf::from(data_dir);
        // The folders of ADR 030. The core takes in nothing from a Markdown folder that has
        // neither of the folders of the pages
        for name in ["docs", "journals", "assets"] {
            std::fs::create_dir_all(PathBuf::from(&markdown_dir).join(name))
                .map_err(|e| e.to_string())?;
        }
        let config_dir = data_dir.join("config");
        std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        let events = Arc::new(EventQueue::default());
        let core = JournalCore::build(
            &CoreSettings {
                db_path: data_dir.join("journal.db"),
                index_dir: data_dir.join("index"),
                config_dir,
                confined_to: None,
                import_enabled: true,
                import_copy_wait: Duration::from_secs(60),
                export_overwrite_wait: Duration::from_secs(60),
            },
            CoreHost {
                events: events.clone(),
                missing_files: Arc::new(Quiet),
                folder_watch: Arc::new(Quiet),
                clock: Arc::new(journal_core::clock::SystemClock),
            },
        )
        .map_err(|e| e.to_string())?;
        core.config.set_markdown_dir(&markdown_dir);
        core.start_up();

        let client_id = ClientId::new();
        let door = Arc::new(Door {
            journal: RwLock::new(Some(EmbeddedJournal::new(
                Arc::clone(&core.journal),
                Arc::clone(&core.confirm),
                &client_id,
            ))),
            events,
            exports: ExportQueueRepository::new(Arc::clone(&core.db)),
            on_change: RwLock::new(None),
            take_in: RwLock::new(()),
        });
        let host = Embedded { door: door.clone() };
        Ok(PetaCore {
            shell: NativeShell::new(host, SystemClock, client_id, move |bytes| {
                shell.process_effects(bytes)
            }),
            core,
            door,
            last: data_dir.join("last-screen"),
            markdown: PathBuf::from(markdown_dir),
            screen: Mutex::default(),
        })
    }

    /// Takes in the files of the markdown folder, and makes them found by the search. The
    /// pages of the files that have gone are deleted. Then the pages that changed in the app
    /// are written to the folder (ADR 031). The shell calls this after a download from
    /// Dropbox, and it uploads after it. Answers with the number of files taken in.
    pub fn sync(&self) -> u32 {
        let _apart = self.door.take_in.write().unwrap();
        let summary = self.core.reconcile.scan();
        self.delete_pages_of_missing_files();
        // After the scan: a file is written over only when its changes are taken in
        self.core.export.process_whole_queue();
        self.core.search.flush();
        u32::try_from(summary.imported).unwrap_or(0)
    }

    /// Deletes the pages whose files have gone, without asking: the data of the app is a
    /// copy of Dropbox (ADR 030).
    ///
    /// A page that changed in the app, and whose file does not hold the change yet, stays
    /// (ADR 031). Its file is written again, and the shell uploads it as a new file.
    ///
    /// The page of a folder cannot go before the pages under it, so the files are gone
    /// through again while a page went. A page that cannot go stays, for the next sync.
    fn delete_pages_of_missing_files(&self) {
        loop {
            let missing = self.core.file_sync_state.find_missing();
            let deleted = missing
                .unwrap_or_default()
                .iter()
                .filter(|file| !self.core.export.waits(&file.page_id))
                .filter(|file| self.core.page_service.delete_page(&file.page_id).is_ok())
                .count();
            if deleted == 0 {
                break;
            }
        }
    }

    /// The app goes to the back, where Android or iOS may stop it: the edit ends, and the
    /// text typed and still waiting for its save is saved now (#766). The shell calls this
    /// before the sync, so that the sync writes the text to its file. It is not `close`, which
    /// the Kotlin side of a core has already, to let go of it.
    pub fn leave(&self) {
        self.shell.close();
    }

    /// Takes an event of the shell. The error says why the bytes could not be read.
    pub fn update(&self, event: &[u8]) -> Result<(), String> {
        self.shell.update(event).map_err(|e| e.to_string())
    }

    /// Takes a change of the Markdown of an item in its text field: a `Typing` as bytes. The
    /// core adds the time.
    pub fn typed(&self, typing: &[u8]) -> Result<(), String> {
        self.shell.typed(typing).map_err(|e| e.to_string())
    }

    /// Pastes the image in the file at `path` into the item, at `cursor` in characters. The
    /// shell wrote the file for this paste: it is deleted once the image is saved (ADR 071).
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

    /// Takes the answer of the shell to the request `id`: the output of its operation.
    pub fn resolve(&self, id: u32, output: &[u8]) -> Result<(), String> {
        self.shell.resolve(id, output).map_err(|e| e.to_string())
    }

    /// What changed on the screen since the last call, as bytes of `Changes`: the changes to
    /// the rows of the outline, and the panels that changed (ADR 076). The first call gives a
    /// reset of the rows and every panel. Call it from the main thread only.
    pub fn changes(&self) -> Vec<u8> {
        self.shell.changes()
    }
}

impl PetaCore {
    /// Opens what the URL names, such as `/pages/notes` or `/today`. A page that is not there
    /// is an error, and it is not made. It is for the screen of the last start, which may have
    /// gone since. The error is the name of the page that is not there.
    fn open(&self, url: String) -> Result<(), String> {
        match Route::parse_url(&url) {
            Some(route) if self.is_there(&route) => self.opened(&url),
            _ => Err(page_of(&url)),
        }
    }

    /// Opens what the URL names, and makes the page or the journal when it is not there. It is
    /// for a page the user asked for: by a link, a search result or the button of today, as in
    /// the other shells (ADR 083). The error, when the URL names nothing, is the name of the
    /// page as `open` gives it.
    fn open_or_make(&self, url: String) -> Result<(), String> {
        self.opened(&url)
    }

    /// The URL of the screen opened last, even before the app was closed. None when nothing
    /// was opened yet.
    fn last_screen(&self) -> Option<String> {
        std::fs::read_to_string(&self.last).ok()
    }

    /// Opens the URL, and keeps it as the last screen when it opened.
    fn opened(&self, url: &str) -> Result<(), String> {
        self.shell.open(url).map_err(|_| page_of(url))?;
        self.keep_last(url);
        Ok(())
    }

    /// Keeps the URL as the screen to open at the next start.
    fn keep_last(&self, url: &str) {
        // Not an error: the app opens the journal of today next time instead
        let _ = std::fs::write(&self.last, url);
    }

    /// True when the route shows without a new page. The core makes the page of a route
    /// that has none, as the web app does for a link to a new page.
    fn is_there(&self, route: &Route) -> bool {
        let Route::Page { name, .. } = route else {
            return true;
        };
        let today = SystemClock.now().0.day_name();
        let name = if name == "today" { &today } else { name };
        let page = if is_journal_file_name(name) {
            let journals = &self.core.pages;
            journals.find_by_type_dir_path_file_name("journal", "", name)
        } else {
            let (dir_path, title) = split_doc_path(name);
            self.core.page_service.find_doc(&title, &dir_path)
        };
        matches!(page, Ok(Some(_)))
    }
}

/// The name of the page that the URL names, decoded, without the item id. Another URL, such
/// as `/today`, is its own name.
fn page_of(url: &str) -> String {
    match journal_core::page_urls::read_url(url) {
        Some(address) => address.name,
        None => url.split('#').next().unwrap_or(url).to_string(),
    }
}

#[cfg(test)]
mod tests;
