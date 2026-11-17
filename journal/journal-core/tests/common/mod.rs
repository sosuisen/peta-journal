//! The journal core on a folder of its own, as an app builds it (ADR 058). Each test makes
//! its own core, so no test sees the pages of another and no setting needs a lock.

pub mod devices;

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
    _dir: tempfile::TempDir,
}

impl Core {
    /// A core with a markdown folder, started up as an app starts it.
    pub fn new() -> Core {
        let dir = tempfile::Builder::new()
            .prefix("peta-journal-core-")
            .tempdir()
            .unwrap();
        let (core, root) = start(dir.path(), WAIT, Arc::new(Quiet));
        Core {
            core,
            root,
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
        let core = build(dir.path(), WAIT, Arc::new(Quiet));
        core.start_up();
        (core, dir)
    }
}

/// A core with its data and its markdown folder under the folder, started up as an app
/// starts it, and its markdown folder. The core waits for the time given before it takes a
/// copied file as a page of its own and before it writes over a file not taken in yet. Its
/// messages to the clients go to the sink.
pub fn start(dir: &Path, wait: Duration, events: Arc<dyn EventSink>) -> (JournalCore, PathBuf) {
    let root = dir.join("markdown");
    std::fs::create_dir_all(&root).unwrap();
    let core = build(dir, wait, events);
    core.config.set_markdown_dir(&path_text(&root));
    core.start_up();
    (core, root)
}

/// A core with its data under the folder, without a markdown folder and not started up
fn build(dir: &Path, wait: Duration, events: Arc<dyn EventSink>) -> JournalCore {
    let config_dir = dir.join("config");
    std::fs::create_dir_all(&config_dir).unwrap();
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

/// The messages a core gives its clients, kept as they were sent. No client answers a
/// question of an extension.
#[derive(Default)]
pub struct Recorded {
    messages: Mutex<Vec<String>>,
}

impl Recorded {
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
        false
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
