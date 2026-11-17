//! The journal core on a folder of its own, as an app builds it (ADR 058). Each test makes
//! its own core, so no test sees the pages of another and no setting needs a lock.

pub mod devices;
#[cfg(feature = "extensions")]
pub mod extensions;
pub mod items;
pub mod pages;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use journal_core::assembly::{CoreHost, CoreSettings, JournalCore};
use journal_core::events::EventSink;
use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use journal_core::journal::FolderWatch;

/// How long a core waits, as the server does, before it takes a file with the id of a page
/// still elsewhere as a copy, and before it writes a page over a file not taken in yet
pub const WAIT: Duration = Duration::from_secs(300);

/// A journal core with its data and its markdown folder in a temporary folder, which goes
/// with it.
pub struct Core {
    pub core: JournalCore,
    /// The markdown folder
    pub root: PathBuf,
    /// What the core told its clients
    pub told: Arc<Recorded>,
    _dir: tempfile::TempDir,
}

impl Core {
    /// A core with a markdown folder, started up as an app starts it.
    pub fn new() -> Core {
        Core::made(Recorded::default(), |_| {})
    }

    /// A core with a markdown folder and the extension functions that `install` puts into
    /// its config folder, which the core reads only when it is built. A client listens: it
    /// gets the questions of the extensions, and the test answers them.
    #[cfg(feature = "extensions")]
    pub fn with_extensions(install: impl FnOnce(&Path)) -> Core {
        Core::made(Recorded::listening(), install)
    }

    fn made(told: Recorded, install: impl FnOnce(&Path)) -> Core {
        let dir = tempfile::Builder::new()
            .prefix("peta-journal-core-")
            .tempdir()
            .unwrap();
        let told = Arc::new(told);
        let core = build(dir.path(), WAIT, told.clone(), install);
        let root = start_up_with_markdown_folder(&core, dir.path());
        Core {
            core,
            root,
            told,
            _dir: dir,
        }
    }

    /// A core whose user has not chosen a markdown folder yet, started up as an app starts
    /// it, and the temporary folder that holds its data.
    pub fn without_markdown_folder() -> (JournalCore, tempfile::TempDir) {
        let dir = tempfile::Builder::new()
            .prefix("peta-journal-core-")
            .tempdir()
            .unwrap();
        let core = build(dir.path(), WAIT, Arc::new(Quiet), |_| {});
        core.start_up();
        (core, dir)
    }
}

/// A core with its data and its markdown folder under the folder, started up as an app
/// starts it, and its markdown folder. The core waits for the time given before it takes a
/// copied file as a page of its own and before it writes over a file not taken in yet. Its
/// messages to the clients go to the sink.
pub fn start(dir: &Path, wait: Duration, events: Arc<dyn EventSink>) -> (JournalCore, PathBuf) {
    let core = build(dir, wait, events, |_| {});
    let root = start_up_with_markdown_folder(&core, dir);
    (core, root)
}

/// Gives the core the markdown folder under the folder and starts it up as an app starts
/// it. Returns the markdown folder.
fn start_up_with_markdown_folder(core: &JournalCore, dir: &Path) -> PathBuf {
    let root = dir.join("markdown");
    std::fs::create_dir_all(&root).unwrap();
    core.config.set_markdown_dir(&path_text(&root));
    core.start_up();
    root
}

/// A core with its data under the folder, without a markdown folder and not started up.
/// `install` gets the config folder before the core is built.
fn build(
    dir: &Path,
    wait: Duration,
    events: Arc<dyn EventSink>,
    install: impl FnOnce(&Path),
) -> JournalCore {
    let config_dir = dir.join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
    install(&config_dir);
    JournalCore::build(
        &CoreSettings {
            db_path: dir.join("data").join("journal.db"),
            index_dir: dir.join("index"),
            config_dir,
            import_enabled: true,
            import_copy_wait: wait,
            export_overwrite_wait: wait,
        },
        CoreHost {
            events,
            missing_files: Arc::new(Quiet),
            folder_watch: Arc::new(Quiet),
        },
    )
    .unwrap()
}

/// The messages a core gives its clients, kept as they were sent. Unless a client listens,
/// no one is there to answer a question of an extension.
#[derive(Default)]
pub struct Recorded {
    messages: Mutex<Vec<String>>,
    listening: bool,
}

impl Recorded {
    /// The messages of a core that a client listens to, so it asks its questions
    #[cfg(feature = "extensions")]
    pub fn listening() -> Recorded {
        Recorded {
            listening: true,
            ..Recorded::default()
        }
    }

    /// The first message the test looks for, waiting for it up to five seconds, as a
    /// thread of the core may send it a little later
    #[cfg(feature = "extensions")]
    pub fn wait_for(&self, wanted: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(found) = self.messages().into_iter().find(&wanted) {
                return found;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "no such message in {:?}",
                self.messages()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// The messages sent so far, each read as JSON, the first first
    pub fn messages(&self) -> Vec<serde_json::Value> {
        self.messages
            .lock()
            .unwrap()
            .iter()
            .map(|m| serde_json::from_str(m).unwrap())
            .collect()
    }
}

impl EventSink for Recorded {
    fn send(&self, message: String) -> bool {
        self.messages.lock().unwrap().push(message);
        self.listening
    }
}

fn path_text(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The parts of the app around the core that the tests do not look at. A scan that finds
/// missing files asks no one.
struct Quiet;

impl EventSink for Quiet {
    fn send(&self, _message: String) -> bool {
        false
    }
}

impl MissingFilesPrompt for Quiet {
    fn notify_new_missing(&self) {}
}

impl FolderWatch for Quiet {
    fn restart(&self) {}
}
