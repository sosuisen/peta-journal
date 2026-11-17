//! The journal core on a folder of its own, as an app builds it (ADR 058). Each test makes
//! its own core, so no test sees the pages of another and no setting needs a lock.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use journal_core::assembly::{CoreHost, CoreSettings, JournalCore};
use journal_core::events::EventSink;
use journal_core::infrastructure::export::reconcile_service::MissingFilesPrompt;
use journal_core::journal::FolderWatch;

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
        let root = dir.path().join("markdown");
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        let core = JournalCore::build(
            &CoreSettings {
                db_path: dir.path().join("data").join("journal.db"),
                index_dir: dir.path().join("index"),
                config_dir,
                import_enabled: true,
                import_copy_wait: Duration::from_secs(300),
                export_overwrite_wait: Duration::from_secs(300),
            },
            CoreHost {
                events: Arc::new(Quiet),
                missing_files: Arc::new(Quiet),
                folder_watch: Arc::new(Quiet),
            },
        )
        .unwrap();
        core.config.set_markdown_dir(&path_text(&root));
        core.start_up();
        Core {
            core,
            root,
            _dir: dir,
        }
    }
}

fn path_text(path: &std::path::Path) -> String {
    path.to_string_lossy().into_owned()
}

/// The parts of the app around the core that the tests do not look at.
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
