//! The entrance of the sync, called here from Rust as Kotlin and Swift call it. Dropbox is a
//! Dropbox of the test, in place of the SDK of the shell.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tempfile::TempDir;

use super::*;

/// A Dropbox of the test, as the SDK of a shell answers.
#[derive(Default)]
struct FakeSdk {
    /// What `list` answers.
    whole: Vec<PetaDropboxEntry>,
    /// The content of each version.
    contents: HashMap<String, String>,
    /// What `list` and `upload` fail with, if anything.
    failure: Option<PetaDropboxError>,
    /// The paths that were uploaded, as Dropbox names them.
    uploads: Mutex<Vec<String>>,
    lists: Mutex<u32>,
    /// What `list_after` answers, if not that nothing changed.
    after: Option<Result<PetaListing, PetaDropboxError>>,
}

impl PetaDropboxSdk for FakeSdk {
    fn list(&self, folder: String) -> Result<PetaListing, PetaDropboxError> {
        assert_eq!(folder, "/PetaJournal");
        *self.lists.lock().unwrap() += 1;
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        Ok(PetaListing {
            entries: self.whole.clone(),
            cursor: "c1".into(),
            has_more: false,
        })
    }

    fn list_after(&self, _cursor: String) -> Result<PetaListing, PetaDropboxError> {
        self.after.clone().unwrap_or(Ok(PetaListing {
            entries: vec![],
            cursor: "c1".into(),
            has_more: false,
        }))
    }

    fn download(&self, rev: String, to: String) -> Result<(), PetaDropboxError> {
        fs::write(to, &self.contents[&rev]).unwrap();
        Ok(())
    }

    fn upload(
        &self,
        path: String,
        _from: String,
        _over: Option<String>,
        _modified_ms: i64,
    ) -> Result<PetaDropboxFile, PetaDropboxError> {
        if let Some(failure) = &self.failure {
            return Err(failure.clone());
        }
        self.uploads.lock().unwrap().push(path.clone());
        Ok(PetaDropboxFile {
            path: Some(path),
            content_hash: Some("uploaded".into()),
            rev: "r9".into(),
        })
    }
}

fn file(path: &str, rev: &str, content: &str) -> PetaDropboxEntry {
    let temp = tempfile::NamedTempFile::new().unwrap();
    fs::write(temp.path(), content).unwrap();
    let hash = client_cloud_sync::content_hash(temp.path()).unwrap();
    PetaDropboxEntry::File {
        file: PetaDropboxFile {
            path: Some(format!("/PetaJournal/{path}")),
            content_hash: Some(hash),
            rev: rev.into(),
        },
    }
}

/// A shell that asks nothing of what the core requests.
struct NoScreen;

impl crate::PetaShell for NoScreen {
    fn process_effects(&self, _bytes: Vec<u8>) {}
}

/// The app of the test: its folders, its core, and what the sync asked of it.
struct App {
    core: PetaCore,
    dir: TempDir,
    told: Receiver<(PetaSyncState, bool)>,
    logged: Arc<Mutex<Vec<String>>>,
    taken_in: Arc<Mutex<u32>>,
    /// What the core writes when it takes the files in, as a page that changed in the app.
    writes: Arc<Mutex<Option<(String, String)>>>,
}

struct Host {
    told: Sender<(PetaSyncState, bool)>,
    logged: Arc<Mutex<Vec<String>>>,
    taken_in: Arc<Mutex<u32>>,
    markdown: PathBuf,
    writes: Arc<Mutex<Option<(String, String)>>>,
}

impl PetaSyncHost for Host {
    fn take_in(&self) {
        *self.taken_in.lock().unwrap() += 1;
        if let Some((path, text)) = self.writes.lock().unwrap().take() {
            fs::write(self.markdown.join(path), text).unwrap();
        }
    }

    fn told(&self, state: PetaSyncState, in_front: bool) {
        let _ = self.told.send((state, in_front));
    }

    fn log(&self, failure: String) {
        self.logged.lock().unwrap().push(failure);
    }
}

fn app() -> (App, PetaCloudSync) {
    app_waiting(60_000)
}

/// The app, with a sync that starts `after_change_ms` after the last change of a page.
fn app_waiting(after_change_ms: u64) -> (App, PetaCloudSync) {
    let dir = TempDir::new().unwrap();
    let markdown = dir.path().join("markdown");
    fs::create_dir_all(markdown.join("docs")).unwrap();
    let (told_tx, told) = channel();
    let logged = Arc::new(Mutex::new(Vec::new()));
    let taken_in = Arc::new(Mutex::new(0));
    let writes = Arc::new(Mutex::new(None));
    let host = Host {
        told: told_tx,
        logged: Arc::clone(&logged),
        taken_in: Arc::clone(&taken_in),
        markdown: markdown.clone(),
        writes: Arc::clone(&writes),
    };
    let path = |name: &str| dir.path().join(name).to_string_lossy().into_owned();
    let core = PetaCore::new(path("data"), path("markdown"), Arc::new(NoScreen)).unwrap();
    let sync = PetaCloudSync::new(
        &core,
        after_change_ms,
        path("downloads"),
        path("dropbox-cursor"),
        path("dropbox-record"),
        true,
        Arc::new(host),
    );
    (
        App {
            core,
            dir,
            told,
            logged,
            taken_in,
            writes,
        },
        sync,
    )
}

impl App {
    /// What the sync told until it rested.
    fn until_rest(&self) -> Vec<(PetaSyncState, bool)> {
        let mut seen = Vec::new();
        loop {
            let told = self.told.recv_timeout(Duration::from_secs(5)).unwrap();
            seen.push(told);
            if matches!(told.0, PetaSyncState::Idle | PetaSyncState::Failed { .. }) {
                return seen;
            }
        }
    }
}

// 同期は、シェルのSDKを通してDropboxのファイルを取り、コアに取り込ませ、アプリで変わったページを上げる。シェルには、その進み具合が伝わる。
#[test]
fn a_sync_downloads_through_the_sdk_of_the_shell_lets_the_core_take_in_and_uploads() {
    let (app, sync) = app();
    let sdk = FakeSdk {
        whole: vec![
            PetaDropboxEntry::Folder {
                path: Some("/PetaJournal/docs".into()),
            },
            file("docs/notes.md", "r1", "- a"),
        ],
        contents: HashMap::from([("r1".to_string(), "- a".to_string())]),
        ..FakeSdk::default()
    };
    *app.writes.lock().unwrap() = Some(("docs/new.md".into(), "- made on the phone".into()));
    let sdk = Arc::new(sdk);
    sync.start(sdk.clone(), "/PetaJournal/".into(), true);
    let seen = app.until_rest();
    assert_eq!(
        seen,
        vec![
            (PetaSyncState::Downloading { done: 0, total: 0 }, true),
            (PetaSyncState::Downloading { done: 0, total: 1 }, true),
            (PetaSyncState::Downloading { done: 1, total: 1 }, true),
            (PetaSyncState::Uploading { done: 0, total: 1 }, true),
            (PetaSyncState::Uploading { done: 1, total: 1 }, true),
            (PetaSyncState::Idle, false),
        ]
    );
    assert_eq!(
        fs::read_to_string(app.dir.path().join("markdown/docs/notes.md")).unwrap(),
        "- a"
    );
    assert_eq!(*app.taken_in.lock().unwrap(), 1);
    assert_eq!(
        *sdk.uploads.lock().unwrap(),
        vec!["/PetaJournal/docs/new.md".to_string()]
    );
    assert_eq!(sync.state(), PetaSyncState::Idle);
}

// シェルが通信の失敗を返すと、同期は通信の失敗で止まる。SDKの言葉は、画面ではなくログに渡る。
#[test]
fn an_offline_failure_of_the_shell_rests_the_sync_as_offline_and_goes_to_the_log() {
    let (app, sync) = app();
    let sdk = FakeSdk {
        failure: Some(PetaDropboxError::Offline {
            why: "urlSessionError".into(),
        }),
        ..FakeSdk::default()
    };
    sync.start(Arc::new(sdk), "/PetaJournal".into(), false);
    assert_eq!(
        app.until_rest().last(),
        Some(&(PetaSyncState::Failed { offline: true }, false))
    );
    let logged = app.logged.lock().unwrap().clone();
    assert_eq!(logged.len(), 1);
    assert!(logged[0].contains("urlSessionError"));
}

// シェルがほかの失敗を返すと、同期は通信の失敗ではない失敗で止まる。
#[test]
fn another_failure_of_the_shell_rests_the_sync_apart_from_the_network() {
    let (app, sync) = app();
    let sdk = FakeSdk {
        failure: Some(PetaDropboxError::Other {
            why: "too many requests".into(),
        }),
        ..FakeSdk::default()
    };
    sync.start(Arc::new(sdk), "/PetaJournal".into(), false);
    assert_eq!(
        app.until_rest().last(),
        Some(&(PetaSyncState::Failed { offline: false }, false))
    );
}

// シェルが宣言していない例外を投げたときは、ほかの失敗になる。
#[test]
fn an_undeclared_error_of_the_shell_is_another_failure() {
    assert_eq!(
        PetaDropboxError::from(boltffi::UnexpectedFfiCallbackError("NSError".into())),
        PetaDropboxError::Other {
            why: "NSError".into()
        }
    );
}

// シェルが版の衝突を返すと、ダウンロードからやり直す。3回続けば、通信ではない失敗で止まる。
#[test]
fn an_upload_conflict_of_the_shell_starts_the_round_again() {
    let (app, sync) = app();
    *app.writes.lock().unwrap() = Some(("docs/new.md".into(), "- new".into()));
    struct Conflicts(FakeSdk);
    impl PetaDropboxSdk for Conflicts {
        fn list(&self, folder: String) -> Result<PetaListing, PetaDropboxError> {
            self.0.list(folder)
        }
        fn list_after(&self, cursor: String) -> Result<PetaListing, PetaDropboxError> {
            self.0.list_after(cursor)
        }
        fn download(&self, rev: String, to: String) -> Result<(), PetaDropboxError> {
            self.0.download(rev, to)
        }
        fn upload(
            &self,
            _: String,
            _: String,
            _: Option<String>,
            _: i64,
        ) -> Result<PetaDropboxFile, PetaDropboxError> {
            Err(PetaDropboxError::UploadConflict)
        }
    }
    sync.start(
        Arc::new(Conflicts(FakeSdk::default())),
        "/PetaJournal".into(),
        false,
    );
    assert_eq!(
        app.until_rest().last(),
        Some(&(PetaSyncState::Failed { offline: false }, false))
    );
    assert_eq!(*app.taken_in.lock().unwrap(), 3);
}

// サインアウトすると、サインインしていない状態になり、カーソルのファイルが消える。
#[test]
fn a_sign_out_rests_signed_out_and_deletes_the_cursor() {
    let (app, sync) = app();
    sync.start(Arc::new(FakeSdk::default()), "/PetaJournal".into(), false);
    app.until_rest();
    let cursor = app.dir.path().join("dropbox-cursor");
    assert!(cursor.exists());
    sync.sign_out();
    assert_eq!(sync.state(), PetaSyncState::SignedOut);
    assert!(!cursor.exists());
    assert_eq!(
        app.told.recv_timeout(Duration::from_secs(1)).unwrap(),
        (PetaSyncState::SignedOut, false)
    );
}

// シェルは、同期の今の状態と、ユーザーが待っているかを、知らせを待たずに読める。休むと、待っている印は消える。
#[test]
fn the_shell_reads_where_the_sync_stands_now_and_whether_the_user_waits() {
    /// A Dropbox that answers the listing only when the test lets it.
    struct Held {
        go: Mutex<std::sync::mpsc::Receiver<()>>,
        inner: FakeSdk,
    }
    impl PetaDropboxSdk for Held {
        fn list(&self, folder: String) -> Result<PetaListing, PetaDropboxError> {
            self.go.lock().unwrap().recv().unwrap();
            self.inner.list(folder)
        }
        fn list_after(&self, cursor: String) -> Result<PetaListing, PetaDropboxError> {
            self.inner.list_after(cursor)
        }
        fn download(&self, rev: String, to: String) -> Result<(), PetaDropboxError> {
            self.inner.download(rev, to)
        }
        fn upload(
            &self,
            path: String,
            from: String,
            over: Option<String>,
            modified_ms: i64,
        ) -> Result<PetaDropboxFile, PetaDropboxError> {
            self.inner.upload(path, from, over, modified_ms)
        }
    }
    let (app, sync) = app();
    let (go, held) = channel();
    let sdk = Held {
        go: Mutex::new(held),
        inner: FakeSdk::default(),
    };
    assert!(!sync.in_front());
    sync.start(Arc::new(sdk), "/PetaJournal".into(), true);
    assert_eq!(
        sync.state(),
        PetaSyncState::Downloading { done: 0, total: 0 }
    );
    assert!(sync.in_front());
    go.send(()).unwrap();
    app.until_rest();
    assert_eq!(sync.state(), PetaSyncState::Idle);
    assert!(!sync.in_front());
}

/// A Dropbox of the test that holds the notes, and answers `after` for the changes since.
fn with_notes(after: Option<Result<PetaListing, PetaDropboxError>>) -> FakeSdk {
    FakeSdk {
        whole: vec![file("docs/notes.md", "r1", "- a")],
        contents: HashMap::from([("r1".to_string(), "- a".to_string())]),
        after,
        ..FakeSdk::default()
    }
}

// シェルが消えたと伝えたファイルは、アプリのフォルダーからも消える。
#[test]
fn a_file_that_the_shell_tells_has_gone_is_deleted_here() {
    let (app, sync) = app();
    sync.start(Arc::new(with_notes(None)), "/PetaJournal".into(), false);
    app.until_rest();
    let gone = Ok(PetaListing {
        entries: vec![PetaDropboxEntry::Deleted {
            path: Some("/PetaJournal/docs/notes.md".into()),
        }],
        cursor: "c2".into(),
        has_more: false,
    });
    sync.start(
        Arc::new(with_notes(Some(gone))),
        "/PetaJournal".into(),
        false,
    );
    app.until_rest();
    assert!(!app.dir.path().join("markdown/docs/notes.md").exists());
}

// シェルがカーソルを忘れたと伝えると、Dropboxの全部を尋ね直す。
#[test]
fn a_cursor_that_the_shell_tells_was_forgotten_asks_for_everything_again() {
    let (app, sync) = app();
    sync.start(Arc::new(with_notes(None)), "/PetaJournal".into(), false);
    app.until_rest();
    let sdk = Arc::new(with_notes(Some(Err(PetaDropboxError::CursorForgotten))));
    sync.start(sdk.clone(), "/PetaJournal".into(), false);
    assert_eq!(app.until_rest().last(), Some(&(PetaSyncState::Idle, false)));
    assert_eq!(*sdk.lists.lock().unwrap(), 1);
}

/// A Dropbox of the test that holds nothing.
fn empty() -> Arc<FakeSdk> {
    Arc::new(FakeSdk::default())
}

impl App {
    /// True when the sync tells nothing more for a while.
    fn stays_quiet(&self) -> bool {
        self.told.recv_timeout(Duration::from_millis(500)).is_err()
    }
}

// アプリでページが変わると、待ち時間のあとに、ユーザーが待たない同期が始まる。ページを作る操作でも同じである（ADR 055）。
#[test]
fn a_page_made_in_the_app_starts_a_quiet_sync_after_the_wait() {
    let (app, sync) = app_waiting(100);
    sync.start(empty(), "/PetaJournal".into(), true);
    app.until_rest();

    app.core.open_or_make("/pages/plan".into()).unwrap();

    assert_eq!(app.until_rest().last(), Some(&(PetaSyncState::Idle, false)));
    assert_eq!(*app.taken_in.lock().unwrap(), 2);
}

// 同期がページの変更のあとで待つ間に、シェルがまだ保存していない入力を伝えると、同期はその入力から待ち直す。
#[test]
fn typing_told_by_the_shell_moves_the_waiting_sync_later() {
    let (app, sync) = app_waiting(500);
    sync.start(empty(), "/PetaJournal".into(), true);
    app.until_rest();
    app.core.open_or_make("/pages/plan".into()).unwrap();
    std::thread::sleep(Duration::from_millis(300));

    sync.typing();
    std::thread::sleep(Duration::from_millis(300));

    assert_eq!(*app.taken_in.lock().unwrap(), 1);
    assert_eq!(app.until_rest().last(), Some(&(PetaSyncState::Idle, false)));
    assert_eq!(*app.taken_in.lock().unwrap(), 2);
}

// ページを読むだけの操作では、同期は始まらない。
#[test]
fn opening_a_page_starts_no_sync() {
    let (app, sync) = app_waiting(100);
    fs::write(app.dir.path().join("markdown/docs/notes.md"), "- a\n").unwrap();
    app.core.sync();
    sync.start(empty(), "/PetaJournal".into(), true);
    app.until_rest();

    app.core.open("/pages/notes".into()).unwrap();

    assert!(app.stays_quiet());
    assert_eq!(*app.taken_in.lock().unwrap(), 1);
}

// ダウンロードしたファイルの取り込みでは、同期は始まらない。取り込みが書き直したページは、同じ回の同期が上げる。
#[test]
fn taking_in_the_downloads_starts_no_sync() {
    let (app, sync) = app_waiting(100);
    sync.start(empty(), "/PetaJournal".into(), true);
    app.until_rest();

    fs::write(app.dir.path().join("markdown/docs/notes.md"), "- a\n").unwrap();
    app.core.sync();

    assert!(app.stays_quiet());
}

// ダウンロードの取り込みと同時に画面がページを読んでも、同期は始まらない。取り込みが書き直すページは、読み取りの変更ではない。
#[test]
fn reading_a_page_while_the_downloads_are_taken_in_starts_no_sync() {
    let (app, sync) = app_waiting(100);
    fs::write(app.dir.path().join("markdown/docs/notes.md"), "- a\n").unwrap();
    app.core.sync();
    sync.start(empty(), "/PetaJournal".into(), true);
    app.until_rest();
    for n in 0..100 {
        let file = format!("markdown/docs/sample-{n}.md");
        fs::write(app.dir.path().join(file), "- a\n").unwrap();
    }

    std::thread::scope(|threads| {
        threads.spawn(|| app.core.sync());
        threads.spawn(|| {
            for _ in 0..100 {
                app.core.open("/pages/notes".into()).unwrap();
            }
        });
    });

    assert!(app.stays_quiet());
}

// アプリを開くと、Markdownフォルダーに、ページの2つのフォルダーと、画像のフォルダーができる（ADR 030）。
#[test]
fn the_app_makes_the_folders_of_the_pages_and_the_images() {
    let (app, _sync) = app();
    for name in ["docs", "journals", "assets"] {
        assert!(
            app.dir.path().join("markdown").join(name).is_dir(),
            "{name}"
        );
    }
}
