//! What the app makes of the answers of Dropbox. Dropbox itself is a Dropbox of the test.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use client_cloud_sync::{
    content_hash, Change, Cloud, CloudFile, DropboxCloud, DropboxEntry, DropboxError, DropboxFile,
    DropboxFiles, Listing, SyncError,
};
use tempfile::TempDir;

/// A Dropbox that answers what the test tells it, and keeps what it was asked.
#[derive(Default)]
struct FakeDropbox {
    /// The whole list of each folder, and the parts after each cursor.
    lists: HashMap<String, Listing>,
    after: HashMap<String, Listing>,
    /// The cursors that Dropbox has forgotten.
    forgotten: HashSet<String>,
    contents: HashMap<String, String>,
    /// What stops the next upload, if anything.
    refusal: Option<DropboxError>,
    uploads: Mutex<Vec<Upload>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Upload {
    path: String,
    text: String,
    over: Option<String>,
    modified_ms: i64,
}

/// The Dropbox of the test, shared with the cloud that asks it.
#[derive(Clone)]
struct Shared(Arc<FakeDropbox>);

impl DropboxFiles for Shared {
    fn list(&self, folder: &str) -> Result<Listing, DropboxError> {
        Ok(self.0.lists[folder].clone())
    }

    fn list_after(&self, cursor: &str) -> Result<Listing, DropboxError> {
        if self.0.forgotten.contains(cursor) {
            return Err(DropboxError::CursorForgotten);
        }
        Ok(self.0.after[cursor].clone())
    }

    fn download(&self, rev: &str, to: &Path) -> Result<(), DropboxError> {
        fs::write(to, &self.0.contents[rev])
            .map_err(|failure| DropboxError::Other(failure.to_string()))
    }

    fn upload(
        &self,
        path: &str,
        from: &Path,
        over: Option<&str>,
        modified_ms: i64,
    ) -> Result<DropboxFile, DropboxError> {
        if let Some(refusal) = &self.0.refusal {
            return Err(refusal.clone());
        }
        self.0.uploads.lock().unwrap().push(Upload {
            path: path.to_string(),
            text: fs::read_to_string(from).unwrap(),
            over: over.map(str::to_string),
            modified_ms,
        });
        Ok(DropboxFile {
            path: Some(path.to_string()),
            content_hash: Some("hash-of-upload".into()),
            rev: "r9".into(),
        })
    }
}

fn cloud(dropbox: FakeDropbox) -> (DropboxCloud, Arc<FakeDropbox>) {
    let shared = Arc::new(dropbox);
    (
        DropboxCloud::new(Shared(Arc::clone(&shared)), "/PetaJournal/"),
        shared,
    )
}

fn file(path: &str, rev: &str) -> DropboxEntry {
    DropboxEntry::File(DropboxFile {
        path: Some(path.to_string()),
        content_hash: Some(format!("hash-{rev}")),
        rev: rev.to_string(),
    })
}

fn listing(entries: Vec<DropboxEntry>, cursor: &str, has_more: bool) -> Listing {
    Listing {
        entries,
        cursor: cursor.to_string(),
        has_more,
    }
}

fn whole(entries: Vec<DropboxEntry>) -> FakeDropbox {
    FakeDropbox {
        lists: HashMap::from([("/PetaJournal".to_string(), listing(entries, "c1", false))]),
        ..FakeDropbox::default()
    }
}

fn paths(changes: &[Change]) -> Vec<&str> {
    changes
        .iter()
        .map(|change| match change {
            Change::File(file) => file.path.as_str(),
            Change::Gone(path) => path.as_str(),
        })
        .collect()
}

/// A page of the app, to upload.
fn page(dir: &TempDir, text: &str) -> PathBuf {
    let page = dir.path().join("a.md");
    fs::write(&page, text).unwrap();
    page
}

// カーソルがなければ、フォルダーの全部の一覧を取る。パスは、フォルダーからの相対になる。
#[test]
fn without_a_cursor_the_changes_are_the_whole_folder_with_paths_inside_it() {
    let (cloud, _) = cloud(whole(vec![file("/PetaJournal/docs/a.md", "r1")]));
    let changes = cloud.changes(None).unwrap();
    assert!(changes.whole);
    assert_eq!(changes.cursor, "c1");
    assert_eq!(
        changes.entries,
        vec![Change::File(CloudFile {
            path: "docs/a.md".into(),
            content_hash: "hash-r1".into(),
            remote: "rev:r1".into(),
        })]
    );
}

// カーソルがあれば、そのあとに変わったものだけを取る。
#[test]
fn with_a_cursor_the_changes_are_what_came_after_it() {
    let dropbox = FakeDropbox {
        after: HashMap::from([(
            "c1".to_string(),
            listing(vec![file("/PetaJournal/docs/b.md", "r2")], "c2", false),
        )]),
        ..FakeDropbox::default()
    };
    let (cloud, _) = cloud(dropbox);
    let changes = cloud.changes(Some("c1")).unwrap();
    assert!(!changes.whole);
    assert_eq!(changes.cursor, "c2");
    assert_eq!(paths(&changes.entries), vec!["docs/b.md"]);
}

// Dropboxがカーソルを忘れていれば、全部の一覧を取り直す。
#[test]
fn a_cursor_that_dropbox_forgot_gives_the_whole_folder_again() {
    let mut dropbox = whole(vec![file("/PetaJournal/docs/a.md", "r1")]);
    dropbox.forgotten.insert("old".into());
    let (cloud, _) = cloud(dropbox);
    let changes = cloud.changes(Some("old")).unwrap();
    assert!(changes.whole);
    assert_eq!(paths(&changes.entries), vec!["docs/a.md"]);
}

// Dropboxに届かなければ、カーソルを忘れたときとは違い、全部の一覧を取り直さずに、通信の失敗になる。
#[test]
fn a_list_that_cannot_reach_dropbox_fails_as_offline() {
    struct Unreachable;
    impl DropboxFiles for Unreachable {
        fn list(&self, _: &str) -> Result<Listing, DropboxError> {
            panic!("the whole list is not asked for")
        }
        fn list_after(&self, _: &str) -> Result<Listing, DropboxError> {
            Err(DropboxError::Offline("no connection".into()))
        }
        fn download(&self, _: &str, _: &Path) -> Result<(), DropboxError> {
            unreachable!()
        }
        fn upload(
            &self,
            _: &str,
            _: &Path,
            _: Option<&str>,
            _: i64,
        ) -> Result<DropboxFile, DropboxError> {
            unreachable!()
        }
    }
    let cloud = DropboxCloud::new(Unreachable, "/PetaJournal");
    assert_eq!(
        cloud.changes(Some("c1")),
        Err(SyncError::Offline("no connection".into()))
    );
}

// Dropboxは一覧を何回かに分けて答える。全部をつなぎ、最後のカーソルを返す。
#[test]
fn a_list_that_comes_in_parts_is_read_to_its_end() {
    let dropbox = FakeDropbox {
        lists: HashMap::from([(
            "/PetaJournal".to_string(),
            listing(vec![file("/PetaJournal/docs/a.md", "r1")], "part", true),
        )]),
        after: HashMap::from([(
            "part".to_string(),
            listing(vec![file("/PetaJournal/docs/b.md", "r1")], "end", false),
        )]),
        ..FakeDropbox::default()
    };
    let (cloud, _) = cloud(dropbox);
    let changes = cloud.changes(None).unwrap();
    assert_eq!(paths(&changes.entries), vec!["docs/a.md", "docs/b.md"]);
    assert_eq!(changes.cursor, "end");
    assert!(changes.whole);
}

// パスは、Dropboxが見せるとおりの大文字と小文字を保つ。ページの名前になるからである。
#[test]
fn a_path_keeps_the_case_that_dropbox_shows() {
    let (cloud, _) = cloud(whole(vec![file("/petajournal/Docs/Trip.md", "r1")]));
    assert_eq!(
        paths(&cloud.changes(None).unwrap().entries),
        vec!["Docs/Trip.md"]
    );
}

// あるフォルダーの知らせは、変更にならない。フォルダーは、その中のファイルで分かる。
#[test]
fn a_folder_that_is_there_is_no_change() {
    let (cloud, _) = cloud(whole(vec![
        DropboxEntry::Folder {
            path: Some("/PetaJournal".into()),
        },
        DropboxEntry::Folder {
            path: Some("/PetaJournal/docs".into()),
        },
        file("/PetaJournal/docs/a.md", "r1"),
    ]));
    assert_eq!(
        paths(&cloud.changes(None).unwrap().entries),
        vec!["docs/a.md"]
    );
}

// 消えたファイルやフォルダーは、「なくなった」という変更になる。
#[test]
fn what_was_deleted_in_dropbox_is_a_change_that_says_gone() {
    let (cloud, _) = cloud(whole(vec![DropboxEntry::Deleted {
        path: Some("/PetaJournal/docs".into()),
    }]));
    assert_eq!(
        cloud.changes(None).unwrap().entries,
        vec![Change::Gone("docs".into())]
    );
}

// パスやハッシュのない知らせでも、落ちない。パスがなければ捨て、ハッシュがなければ空にする。
#[test]
fn an_entry_without_a_path_is_left_out_and_one_without_a_hash_gets_an_empty_one() {
    let (cloud, _) = cloud(whole(vec![
        DropboxEntry::File(DropboxFile {
            path: None,
            content_hash: Some("h".into()),
            rev: "r1".into(),
        }),
        DropboxEntry::File(DropboxFile {
            path: Some("/PetaJournal/docs/a.md".into()),
            content_hash: None,
            rev: "r2".into(),
        }),
    ]));
    assert_eq!(
        cloud.changes(None).unwrap().entries,
        vec![Change::File(CloudFile {
            path: "docs/a.md".into(),
            content_hash: String::new(),
            remote: "rev:r2".into(),
        })]
    );
}

// ダウンロードは、一覧で見たそのバージョンを取る。
#[test]
fn a_download_takes_the_very_version_that_the_list_showed() {
    let mut dropbox = whole(vec![file("/PetaJournal/docs/a.md", "r7")]);
    dropbox.contents.insert("r7".into(), "- seventh".into());
    let (cloud, _) = cloud(dropbox);
    let Change::File(seen) = cloud.changes(None).unwrap().entries.remove(0) else {
        panic!("a file was listed")
    };
    let dir = TempDir::new().unwrap();
    let into = dir.path().join("into");
    cloud.download(&seen, &into).unwrap();
    assert_eq!(fs::read_to_string(into).unwrap(), "- seventh");
}

// Dropboxにないファイルは、置き換えるバージョンなしで上げる。フォルダーのパスを前に付ける。
#[test]
fn a_new_file_is_uploaded_over_no_version_into_the_folder() {
    let dir = TempDir::new().unwrap();
    let (cloud, dropbox) = cloud(FakeDropbox::default());
    let answer = cloud
        .upload("docs/a.md", &page(&dir, "- new"), None)
        .unwrap();
    let uploads = dropbox.uploads.lock().unwrap().clone();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].path, "/PetaJournal/docs/a.md");
    assert_eq!(uploads[0].over, None);
    assert_eq!(uploads[0].text, "- new");
    assert_eq!(
        answer,
        CloudFile {
            path: "docs/a.md".into(),
            content_hash: "hash-of-upload".into(),
            remote: "rev:r9".into(),
        }
    );
}

// Dropboxにあるファイルは、アプリが見たバージョンの上にだけ上げる。
#[test]
fn a_file_that_dropbox_has_is_uploaded_over_the_version_that_the_app_saw() {
    let dir = TempDir::new().unwrap();
    let (cloud, dropbox) = cloud(FakeDropbox::default());
    cloud
        .upload("docs/a.md", &page(&dir, "- a"), Some("rev:r3"))
        .unwrap();
    assert_eq!(
        dropbox.uploads.lock().unwrap()[0].over.as_deref(),
        Some("r3")
    );
}

// 上げるファイルの更新時刻は、秒までにする。Dropboxは、秒より細かい時刻を受け取らない。
#[test]
fn an_upload_tells_the_time_of_the_last_change_in_whole_seconds() {
    let dir = TempDir::new().unwrap();
    let page = page(&dir, "- a");
    let modified = SystemTime::UNIX_EPOCH + Duration::from_millis(1_700_000_000_999);
    fs::File::options()
        .write(true)
        .open(&page)
        .unwrap()
        .set_modified(modified)
        .unwrap();
    let (cloud, dropbox) = cloud(FakeDropbox::default());
    cloud.upload("docs/a.md", &page, None).unwrap();
    assert_eq!(
        dropbox.uploads.lock().unwrap()[0].modified_ms,
        1_700_000_000_000
    );
}

// Dropboxのファイルが別のバージョンなら、「クラウドで変わった」というエラーになる。
#[test]
fn an_upload_over_another_version_fails_as_changed_in_the_cloud() {
    let dir = TempDir::new().unwrap();
    let (cloud, _) = cloud(FakeDropbox {
        refusal: Some(DropboxError::UploadConflict),
        ..FakeDropbox::default()
    });
    assert_eq!(
        cloud.upload("docs/a.md", &page(&dir, "- a"), Some("rev:r3")),
        Err(SyncError::ChangedInCloud("docs/a.md".into()))
    );
}

// 通信の失敗は、通信の失敗として伝わる。ほかの失敗は、その理由を持って伝わる。
#[test]
fn an_upload_that_fails_for_another_reason_fails_with_that_reason() {
    let dir = TempDir::new().unwrap();
    let (offline, _) = cloud(FakeDropbox {
        refusal: Some(DropboxError::Offline("the network is gone".into())),
        ..FakeDropbox::default()
    });
    let (other, _) = cloud(FakeDropbox {
        refusal: Some(DropboxError::Other("too many requests".into())),
        ..FakeDropbox::default()
    });
    let page = page(&dir, "- a");
    assert_eq!(
        offline.upload("docs/a.md", &page, None),
        Err(SyncError::Offline("the network is gone".into()))
    );
    assert_eq!(
        other.upload("docs/a.md", &page, None),
        Err(SyncError::Other("too many requests".into()))
    );
}

// ファイルのハッシュは、Dropboxのコンテンツハッシュである。
#[test]
fn the_content_hash_of_a_file_is_the_one_of_dropbox() {
    let dir = TempDir::new().unwrap();
    let (cloud, _) = cloud(FakeDropbox::default());
    let page = page(&dir, "- a");
    assert_eq!(
        cloud.content_hash(&page).unwrap(),
        content_hash(&page).unwrap()
    );
}

// 一覧の続きを読む途中でDropboxがカーソルを忘れたら、通信ではない失敗になる。
#[test]
fn a_cursor_forgotten_in_the_middle_of_a_list_fails_apart_from_the_network() {
    let mut dropbox = whole(vec![]);
    dropbox
        .lists
        .insert("/PetaJournal".into(), listing(vec![], "part", true));
    dropbox.forgotten.insert("part".into());
    let (cloud, _) = cloud(dropbox);
    assert!(matches!(cloud.changes(None), Err(SyncError::Other(_))));
}
