//! The folder watcher takes in what an editor or a cloud folder does to the markdown files,
//! without waiting for the folder scan. Each test watches a folder of its own with a server
//! state of its own. Only the watcher runs there: no export scheduler and no folder scan.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use hmtid::Hmtid;
use journal_core::infrastructure::export::import_service::{MarkdownImportService, Outcome};
use journal_core::journal::UpdateConfigInput;
use journal_server::config::Settings;
use journal_server::infrastructure::pending_delete_prompt::PendingDeletePrompt;
use journal_server::infrastructure::watch::{FileImport, MarkdownFolderWatcher};
use journal_server::state::{AppState, BuildOptions, SharedState};
use serde_json::json;

struct Folder {
    state: SharedState,
    root: PathBuf,
    _dir: tempfile::TempDir,
}

/// An import that fails the first times, as when an editor or a cloud folder still holds the
/// file locked. Then it takes the file in. It counts how often the watcher tried.
struct FailingImport {
    real: Arc<MarkdownImportService>,
    failures_left: AtomicI32,
    tries: AtomicI32,
}

impl FailingImport {
    fn tries(&self) -> i32 {
        self.tries.load(Ordering::SeqCst)
    }

    /// Fails the next tries, as when the file is locked again
    fn fail_next(&self, failures: i32) {
        self.failures_left.store(failures, Ordering::SeqCst);
    }
}

impl FileImport for FailingImport {
    fn sync_file(&self, root: &Path, rel_path: &str) -> Outcome {
        self.tries.fetch_add(1, Ordering::SeqCst);
        if self.failures_left.fetch_sub(1, Ordering::SeqCst) > 0 {
            return Outcome::Error;
        }
        self.real.sync_file(root, rel_path)
    }
}

impl Folder {
    /// A markdown folder with a docs/ folder, watched from now on
    fn watched() -> Folder {
        let folder = Folder::new();
        folder.state.watcher.start();
        // start() attaches the watcher only after a few seconds; restart() does it now
        folder.state.watcher.restart();
        folder
    }

    /// A watched folder whose import fails the given number of times
    fn watched_failing(failures: i32) -> (Folder, Arc<FailingImport>) {
        let folder = Folder::new();
        let import = Arc::new(FailingImport {
            real: Arc::clone(&folder.state.import),
            failures_left: AtomicI32::new(failures),
            tries: AtomicI32::new(0),
        });
        let state = &folder.state;
        let watcher = Arc::new(MarkdownFolderWatcher::new(
            true,
            state.settings.import_debounce_ms,
            Arc::clone(&state.export),
            import.clone(),
            Arc::clone(&state.reconcile),
            state.file_sync_state.clone(),
            Arc::new(PendingDeletePrompt::new(
                0,
                state.settings.import_delete_prompt_cooldown_secs,
            )),
            Arc::clone(&state.search),
        ));
        // The tasks the watcher starts keep it alive
        watcher.start();
        watcher.restart();
        (folder, import)
    }

    /// A markdown folder with a docs/ folder, not watched yet
    fn new() -> Folder {
        let dir = tempfile::Builder::new()
            .prefix("peta-journal-watch-")
            .tempdir()
            .unwrap();
        let settings = Settings {
            // A short quiet time keeps the tests short
            import_debounce_ms: 100,
            // A deleted file opens the browser to ask the user, unless it did within the
            // cooldown; one longer than the time since 1970 keeps it shut
            import_delete_prompt_cooldown_secs: 10_000_000_000,
            ..Settings::under(dir.path(), 0)
        };
        let root = dir.path().join("markdown");
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            json!({ "markdownDir": root.to_string_lossy() }).to_string(),
        )
        .unwrap();
        let state = AppState::build(settings, BuildOptions { interactive: false }).unwrap();
        Folder {
            state,
            root,
            _dir: dir,
        }
    }

    /// Writes the file as an editor or a cloud folder does
    fn write(&self, rel_path: &str, text: &str) {
        std::fs::write(self.root.join(rel_path), text).unwrap();
    }

    /// A page that is already in, with its file, as the scan or an earlier import left it
    fn page_with_file(&self, name: &str, text: &str) -> Hmtid {
        let rel_path = format!("docs/{name}.md");
        self.write(&rel_path, text);
        assert_eq!(
            self.state.import.sync_file(&self.root, &rel_path),
            Outcome::Imported
        );
        self.page(name).unwrap()
    }

    fn page(&self, name: &str) -> Option<Hmtid> {
        self.state
            .pages
            .find_by_type_dir_path_file_name("doc", "", name)
            .unwrap()
            .map(|p| p.id)
    }

    /// The items of the page, each as `level:body`
    fn outline(&self, page_id: &Hmtid) -> Vec<String> {
        self.state
            .page_service
            .get_page_items(page_id)
            .unwrap()
            .iter()
            .map(|i| format!("{}:{}", i.level, i.body))
            .collect()
    }

    /// Whether a search for the word finds the page
    fn finds(&self, page_id: &Hmtid, word: &str) -> bool {
        self.state
            .journal
            .search_combined(word, Some(10))
            .unwrap()
            .results
            .iter()
            .any(|r| r.page_id == page_id.as_str())
    }

    fn is_missing(&self, rel_path: &str) -> bool {
        self.state
            .file_sync_state
            .find_by_path(rel_path)
            .unwrap()
            .is_some_and(|s| s.is_missing())
    }
}

/// Lets the watcher work until the condition holds. The watcher takes a file in within a
/// second. The limit stays below the 20 seconds after which cargo-mutants stops a test, so a
/// mutant that stops the watcher counts as caught, not as a timeout.
async fn eventually(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting until {what}");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

// 外部で編集したファイルは、スキャンを待たずにページに取り込まれ、検索で見つかる。
#[tokio::test]
async fn a_file_edited_outside_is_taken_into_its_page_and_found_without_a_scan() {
    let folder = Folder::watched();
    let page = folder.page_with_file("edited", "before the edit\n");

    folder.write("docs/edited.md", "edited outside with rhubarb\n");

    eventually("the edit is in the page", || {
        folder.outline(&page) == ["1:edited outside with rhubarb"]
    })
    .await;
    eventually("the edit is found", || folder.finds(&page, "rhubarb")).await;
}

// フォルダに置いた新しいファイルは、スキャンを待たずにページになる。
#[tokio::test]
async fn a_new_file_in_the_folder_becomes_a_page_without_a_scan() {
    let folder = Folder::watched();

    folder.write("docs/new.md", "hello from disk\n\nsecond block\n");

    eventually("the file is a page", || folder.page("new").is_some()).await;
    let page = folder.page("new").unwrap();
    eventually("the page has the blocks of the file", || {
        folder.outline(&page) == ["1:hello from disk", "1:second block"]
    })
    .await;
}

// ほかの端末が書いたsettings/properties.jsonは、スキャンを待たずに取り込まれる。settings/がまだない端末でも、監視を始めるときに作って見る。
#[tokio::test]
async fn a_file_of_definitions_from_another_device_is_taken_in_without_a_scan() {
    let folder = Folder::watched();

    folder.write(
        "settings/properties.json",
        r#"{ "definitions": { "score": { "type": "number", "page": false, "modified": "2025-11-15T01:00:00Z" } } }"#,
    );

    eventually("the definition is in", || {
        folder
            .state
            .properties
            .find_definition("score")
            .unwrap()
            .is_some()
    })
    .await;
}

// 外部で消したファイルは、消えたものとして記録され、ページはユーザーが決めるまで残る。
#[tokio::test]
async fn a_file_deleted_outside_is_held_as_missing_and_its_page_stays() {
    let folder = Folder::watched();
    let page = folder.page_with_file("deleted", "still here\n");

    std::fs::remove_file(folder.root.join("docs/deleted.md")).unwrap();

    eventually("the file is held as missing", || {
        folder.is_missing("docs/deleted.md")
    })
    .await;
    assert_eq!(folder.outline(&page), ["1:still here"]);
}

// Markdownフォルダーの設定を変えると、すぐに新しいフォルダーが見張られる。
#[tokio::test]
async fn a_new_markdown_directory_is_watched_at_once() {
    let folder = Folder::watched();
    let moved = folder._dir.path().join("moved");
    std::fs::create_dir_all(moved.join("docs")).unwrap();
    // The watcher looks at the setting by itself 3 seconds after its start, and then every
    // 15 seconds. After the first look, only the change of the setting can move it in time
    tokio::time::sleep(Duration::from_millis(3500)).await;

    folder.state.journal.update_config(UpdateConfigInput {
        markdown_dir: Some(moved.to_string_lossy().into_owned()),
        ..UpdateConfigInput::default()
    });
    std::fs::write(moved.join("docs/moved.md"), "in the new folder\n").unwrap();

    eventually("the file is a page", || folder.page("moved").is_some()).await;
}

// 空のファイルが先に作られても、ページは中身が書かれてから作られる。
#[tokio::test]
async fn an_empty_file_becomes_a_page_only_once_its_content_is_written() {
    let folder = Folder::watched();

    // Sync tools and some editors create the file first and fill it in later
    folder.write("docs/later.md", "");
    // Long enough for the watcher to try the empty file after its quiet time
    tokio::time::sleep(Duration::from_millis(1000)).await;
    assert_eq!(folder.page("later"), None);

    folder.write("docs/later.md", "written later\n");

    eventually("the file is a page", || folder.page("later").is_some()).await;
    let page = folder.page("later").unwrap();
    eventually("the page has the content", || {
        folder.outline(&page) == ["1:written later"]
    })
    .await;
}

// 読み込みに失敗したファイルは、新しい変更を待たずに取り込み直される。
#[tokio::test]
async fn a_file_that_failed_to_read_is_taken_in_again_without_a_new_change() {
    let (folder, import) = Folder::watched_failing(2);

    folder.write("docs/locked.md", "read at last\n");

    eventually("the file is a page", || folder.page("locked").is_some()).await;
    let page = folder.page("locked").unwrap();
    // The page exists a moment before its items do
    eventually("the page has the content", || {
        folder.outline(&page) == ["1:read at last"]
    })
    .await;
    assert_eq!(import.tries(), 3);
}

// 読み込みの失敗が続くファイルは、4回試したところで諦められ、スキャンが取り込む。
#[tokio::test]
async fn a_file_that_keeps_failing_is_left_to_the_scan_after_four_tries() {
    let (folder, import) = Folder::watched_failing(i32::MAX);

    folder.write("docs/stuck.md", "for the scan\n");

    eventually("the watcher tried four times", || import.tries() >= 4).await;
    // Long enough for two more tries, if the watcher had not given up
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(import.tries(), 4);
    assert_eq!(folder.page("stuck"), None);

    folder.state.reconcile.scan();
    let page = folder.page("stuck").unwrap();
    assert_eq!(folder.outline(&page), ["1:for the scan"]);
}

// 一度失敗してから取り込めたファイルは、あとでまた失敗しても、また4回試される。
#[tokio::test]
async fn a_file_taken_in_after_failing_gets_four_tries_again_when_it_fails_later() {
    let (folder, import) = Folder::watched_failing(2);
    folder.write("docs/twice.md", "first save\n");
    eventually("the file is a page", || folder.page("twice").is_some()).await;
    assert_eq!(import.tries(), 3);

    import.fail_next(i32::MAX);
    folder.write("docs/twice.md", "second save\n");

    eventually("the watcher tried four more times", || import.tries() >= 7).await;
    // Long enough for two more tries, if the watcher had not given up
    tokio::time::sleep(Duration::from_millis(1500)).await;
    assert_eq!(import.tries(), 7);
}
