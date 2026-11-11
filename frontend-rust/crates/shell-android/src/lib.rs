//! The core for the Android shell: the Crux core, and a journal core inside the app instead
//! of a journal server (ADR 029). BoltFFI makes the Kotlin side of the entrance.

boltffi::scaffolding!();

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use chrono::{Datelike, Local, Timelike};
use journal_core::assembly::{CoreHost, CoreSettings, JournalCore};
use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use journal_core::journal::FolderWatch;
use peta_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, LocalDateTimeDto, SseEvent};
use peta_core::rest::ClientId;
use peta_core::screens::page::split_doc_path;
use peta_core::util::filename::is_journal_file_name;
use peta_core::Route;
use peta_embedded_journal::{EmbeddedJournal, EventQueue};
use peta_shell_native::{Host, NativeShell};

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
}

impl Door {
    fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        match &*self.journal.read().unwrap() {
            Some(journal) => journal.call(op),
            None => Err(ApiError("the journal is closed".into())),
        }
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

    fn now(&self) -> (LocalDateTimeDto, u64) {
        let now = Local::now();
        let local = LocalDateTimeDto {
            year: now.year(),
            month: now.month(),
            day: now.day(),
            hour: now.hour(),
            minute: now.minute(),
        };
        (local, now.timestamp_millis().unsigned_abs())
    }

    fn sleep(&self, ms: u32) {
        thread::sleep(Duration::from_millis(ms.into()));
    }

    fn events(&self, send: &mut dyn FnMut(SseEvent)) {
        self.door.events.read(send);
    }
}

/// What the shell gives the core, to hear from it.
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
        let config_dir = data_dir.join("config");
        std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        let events = Arc::new(EventQueue::default());
        let core = JournalCore::build(
            &CoreSettings {
                db_path: data_dir.join("journal.db"),
                index_dir: data_dir.join("index"),
                config_dir,
                import_enabled: true,
                import_copy_wait: Duration::from_secs(60),
                export_overwrite_wait: Duration::from_secs(60),
            },
            CoreHost {
                events: events.clone(),
                missing_files: Arc::new(Quiet),
                folder_watch: Arc::new(Quiet),
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
        });
        let host = Embedded { door: door.clone() };
        Ok(PetaCore {
            shell: NativeShell::new(host, client_id, move |bytes| shell.process_effects(bytes)),
            core,
            door,
        })
    }

    /// Takes in the files of the markdown folder, and makes them found by the search. The
    /// pages of the files that have gone are deleted. The shell calls this after a download
    /// from Dropbox. Answers with the number of files taken in.
    pub fn sync(&self) -> u32 {
        let summary = self.core.reconcile.scan();
        self.delete_pages_of_missing_files();
        self.core.search.flush();
        u32::try_from(summary.imported).unwrap_or(0)
    }

    /// Deletes the pages whose files have gone, without asking: the data of the app is a
    /// copy of Dropbox (ADR 030).
    ///
    /// The page of a folder cannot go before the pages under it, so the files are gone
    /// through again while a page went. A page that cannot go stays, for the next sync.
    fn delete_pages_of_missing_files(&self) {
        loop {
            let missing = self.core.file_sync_state.find_missing();
            let deleted = missing
                .unwrap_or_default()
                .iter()
                .filter(|file| self.core.page_service.delete_page(&file.page_id).is_ok())
                .count();
            if deleted == 0 {
                break;
            }
        }
    }

    /// Opens what the URL names, such as `/pages/notes` or `/today`. A page that is not there
    /// is an error, and it is not made: the app only reads (ADR 030).
    pub fn open(&self, url: String) -> Result<(), String> {
        let nothing = format!("nothing to open at {url:?}");
        match Route::parse_url(&url) {
            Some(route) if self.is_there(&route) => self.shell.open(&url).ok().ok_or(nothing),
            _ => Err(nothing),
        }
    }

    /// True when the route shows without a new page. The core makes the page of a route
    /// that has none, as the web app does for a link to a new page.
    fn is_there(&self, route: &Route) -> bool {
        let Route::Page { name, .. } = route else {
            return true;
        };
        let today = Local::now().format("%Y-%m-%d").to_string();
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

/// The HTML of an item, from its AST as the JSON that the view model carries (ADR 025). A
/// link to a page and a tag are links to the URL of their page, such as `/pages/notes`, for
/// `PetaCore::open`. A tag page is under `tags_dir`. The URL is encoded as a web address is.
/// An AST that cannot be read gives no HTML.
#[boltffi::export]
pub fn item_html(ast: String, tags_dir: String) -> String {
    // The app has no address: the URLs are paths
    peta_shell_native::copied_html(&ast, "", &tags_dir)
}

#[cfg(test)]
mod tests;
