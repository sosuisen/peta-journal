//! The folder watcher takes in what an editor or a cloud folder does to the markdown files,
//! without waiting for the folder scan. Each test watches a folder of its own with a server
//! state of its own. Only the watcher runs there: no export scheduler and no folder scan.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use hmtid::Hmtid;
use journal_server::config::Settings;
use journal_server::infrastructure::export::import_service::Outcome;
use journal_server::state::{AppState, BuildOptions, SharedState};
use serde_json::json;

struct Folder {
    state: SharedState,
    root: PathBuf,
    _dir: tempfile::TempDir,
}

impl Folder {
    /// A markdown folder with a docs/ folder, watched from now on
    fn watched() -> Folder {
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
        state.watcher.start();
        // start() attaches the watcher only after a few seconds; restart() does it now
        state.watcher.restart();
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
            .combined_search
            .search(word, 10)
            .unwrap()
            .iter()
            .any(|r| &r.page_id == page_id)
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
