//! Pages and their atlas shared by two devices through their markdown folders, as a cloud
//! folder keeps them alike. Each device has a core and a folder of its own. A file reaches the
//! other device when the test copies it, so the test decides the order the files arrive in.

use hmtid::Hmtid;
use journal_core::infrastructure::export::import_service::Outcome;

use crate::common::devices::{two_devices, two_devices_with, Device, INDEX};

impl Device {
    pub(crate) fn atlas_id(&self) -> Hmtid {
        self.core
            .pages
            .find_by_type_dir_path_file_name("doc", "d/", "index_atlas")
            .unwrap()
            .unwrap()
            .id
    }

    /// The outline of the atlas, each item as `level:body`
    pub(crate) fn outline(&self) -> Vec<String> {
        self.core
            .page_service
            .get_page_items(&self.atlas_id())
            .unwrap()
            .iter()
            .map(|i| format!("{}:{}", i.level, i.body))
            .collect()
    }
}

/// The page c made on A and sent to B, then moved to `e/` on A
fn moved_on_a(a: &Device, b: &Device) -> Hmtid {
    let c = a.doc("c");
    b.receive(a, "docs/d/c.md");
    a.core
        .page_service
        .update_page(&c, None, Some("e/"), false)
        .unwrap();
    c
}

// 新しいファイルが先に届いた移動は、古いファイルが消えたときにページを移動する。
#[test]
fn a_move_that_brings_the_new_file_first_moves_the_page_when_the_old_file_goes() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = moved_on_a(&a, &b);

    // The cloud folder brings the file where it went before it takes the old one away
    assert_eq!(b.receive(&a, "docs/e/c.md"), Outcome::Waiting);
    std::fs::remove_file(b.root.join("docs/d/c.md")).unwrap();
    assert_eq!(b.sync("docs/d/c.md"), Outcome::Missing);

    assert_eq!(b.page_in("e/", "c"), Some(c));
    assert_eq!(b.page_in("d/", "c"), None);
    assert_eq!(b.file("docs/e/c.md"), a.file("docs/e/c.md"));
    assert!(b.conflicts().is_empty());
}

// 待機中のファイルは、古いファイルが消えたあとの走査で取り込まれる。
#[test]
fn a_file_that_waits_comes_in_on_the_scan_after_the_old_file_went() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = moved_on_a(&a, &b);
    assert_eq!(b.receive(&a, "docs/e/c.md"), Outcome::Waiting);

    // The old file goes while the server does not see it, as when it was not running
    std::fs::remove_file(b.root.join("docs/d/c.md")).unwrap();
    assert_eq!(b.sync("docs/e/c.md"), Outcome::Imported);

    assert_eq!(b.page_in("e/", "c"), Some(c));
    assert_eq!(b.page_in("d/", "c"), None);
}

// 複製されたファイルは、待機が終わると独立したページになる。
#[test]
fn a_copied_file_becomes_a_page_of_its_own_once_the_wait_is_over() {
    let dir = tempfile::tempdir().unwrap();
    let device = Device::not_waiting(dir.path());
    let a = device.doc("a");
    device.export();
    std::fs::copy(
        device.root.join("docs/d/a.md"),
        device.root.join("docs/d/a2.md"),
    )
    .unwrap();

    assert_eq!(device.sync("docs/d/a2.md"), Outcome::Imported);

    let copy = device.page_in("d/", "a2").unwrap();
    assert_ne!(copy, a);
    assert_eq!(device.page_in("d/", "a"), Some(a));
    // The copy is written again with its own id
    assert!(device.file("docs/d/a2.md").contains(copy.as_str()));
}

// まだ取り込んでいないファイルは上書きされない。
#[test]
fn a_file_not_taken_in_yet_is_not_written_over() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let page = a.page_in("d/", "a").unwrap();
    a.edit(&page, "from A");
    b.deliver(&a, "docs/d/a.md");

    // B changes the page before it takes in the file that came
    b.edit(&page, "from B");
    b.export();
    assert!(b.on_disk("docs/d/a.md").contains("from A"));

    // The import decides: the change on B is later, so the file of A is kept as a backup
    assert_eq!(b.sync("docs/d/a.md"), Outcome::Conflict);
    assert!(b.file("docs/d/a.md").contains("from B"));
    let backups = b.conflicts();
    assert_eq!(backups.len(), 1);
    assert!(std::fs::read_to_string(&backups[0])
        .unwrap()
        .contains("from A"));
}

// 待機が終わっても取り込まれなかったファイルは、バックアップを残して上書きされる。
#[test]
fn a_file_not_taken_in_after_the_wait_is_kept_as_a_backup_and_written_over() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices_with(dir.path(), Device::not_waiting);
    let page = a.page_in("d/", "a").unwrap();
    a.edit(&page, "from A");
    b.deliver(&a, "docs/d/a.md");

    b.edit(&page, "from B");
    b.export();

    assert!(b.on_disk("docs/d/a.md").contains("from B"));
    let backups = b.conflicts();
    assert_eq!(backups.len(), 1);
    assert!(std::fs::read_to_string(&backups[0])
        .unwrap()
        .contains("from A"));
}

// アプリで作られたフォルダのindexページは、すぐにファイルを得る。
#[test]
fn a_folder_index_made_in_the_app_gets_a_file_at_once() {
    let dir = tempfile::tempdir().unwrap();
    let device = Device::new(dir.path());
    device.doc("a");
    let index = device.page_in("d/", "index").unwrap();

    device.export();

    assert!(device.on_disk(INDEX).contains(index.as_str()));
}

// 取り込んだファイルのために作られたindexページは、他方のデバイスのindexファイルを待つ。
#[test]
fn a_folder_index_made_for_a_file_taken_in_waits_for_the_index_file_of_the_other_device() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    a.core
        .page_service
        .get_or_create_doc(Some("x"), Some("e/"))
        .unwrap();

    b.receive(&a, "docs/e/x.md");
    b.export();
    assert!(!b.root.join("docs/e/index.md").exists());

    // Both index pages have an id of the same second, so either may be the older one; the
    // older wins on both devices once each has the file of the other
    let older = a.page_in("e/", "index").min(b.page_in("e/", "index"));
    b.receive(&a, "docs/e/index.md");
    a.receive(&b, "docs/e/index.md");
    assert_eq!(a.page_in("e/", "index"), older);
    assert_eq!(b.page_in("e/", "index"), older);
    assert_eq!(b.file("docs/e/index.md"), a.file("docs/e/index.md"));
}

// 他方のデバイスで移動したページは、削除の候補にならない。
#[test]
fn a_page_moved_on_the_other_device_is_not_offered_for_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    moved_on_a(&a, &b);

    std::fs::remove_file(b.root.join("docs/d/c.md")).unwrap();
    assert_eq!(b.sync("docs/d/c.md"), Outcome::Missing);
    assert_eq!(b.receive(&a, "docs/e/c.md"), Outcome::Imported);

    // The import forgets the old path before any scan
    let old = b.core.file_sync_state.find_by_path("docs/d/c.md").unwrap();
    assert!(old.is_none());
    assert!(b.missing().is_empty());
}

// 以前に移動したページの古いパスは、もう削除の候補にならない。
#[test]
fn the_old_path_of_a_page_moved_before_is_no_longer_offered_for_deletion() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = moved_on_a(&a, &b);
    std::fs::remove_file(b.root.join("docs/d/c.md")).unwrap();
    b.receive(&a, "docs/e/c.md");
    // A move taken in by an older version left the old path as a deleted file
    let sync_state = &b.core.file_sync_state;
    sync_state
        .upsert_synced("docs/d/c.md", &c, "old", 1, 1, 1)
        .unwrap();
    sync_state.mark_missing("docs/d/c.md").unwrap();

    assert!(b.missing().is_empty());
}
