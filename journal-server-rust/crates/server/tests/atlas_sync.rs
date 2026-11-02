//! Pages and their atlas shared by two devices through their markdown folders, as a cloud
//! folder keeps them alike. Each device has a server state and a folder of its own. A file reaches the
//! other device when the test copies it, so the test decides the order the files arrive in.

use std::path::{Path, PathBuf};

use hmtid::Hmtid;
use journal_server::config::Settings;
use journal_server::infrastructure::export::import_service::Outcome;
use journal_server::state::{AppState, BuildOptions, SharedState};
use serde_json::json;

struct Device {
    state: SharedState,
    root: PathBuf,
}

impl Device {
    fn new(dir: &Path) -> Device {
        Device::with(dir, Settings::under(dir, 0))
    }

    /// A device that does not wait: it takes a file with the id of a page still elsewhere as
    /// a copy at once, and writes a page over a file not taken in yet at once
    fn not_waiting(dir: &Path) -> Device {
        let settings = Settings {
            import_copy_wait_secs: 0,
            export_overwrite_wait_secs: 0,
            ..Settings::under(dir, 0)
        };
        Device::with(dir, settings)
    }

    fn with(dir: &Path, settings: Settings) -> Device {
        // A scan that finds missing files opens the browser, unless it did within the
        // cooldown; one longer than the time since 1970 keeps it shut
        let settings = Settings {
            import_delete_prompt_cooldown_secs: 10_000_000_000,
            ..settings
        };
        let root = dir.join("markdown");
        let config_dir = dir.join("config");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            json!({ "markdownDir": root.to_string_lossy() }).to_string(),
        )
        .unwrap();
        let state = AppState::build(settings, BuildOptions { interactive: false }).unwrap();
        Device { state, root }
    }

    fn export(&self) {
        self.state.export.process_export_queue();
    }

    /// Takes the file as the other device wrote it, as the cloud folder brings it.
    fn receive(&self, from: &Device, rel_path: &str) -> Outcome {
        self.deliver(from, rel_path);
        self.sync(rel_path)
    }

    /// Puts the file the other device wrote into the folder, before the server takes it in
    fn deliver(&self, from: &Device, rel_path: &str) {
        from.export();
        let content = std::fs::read(from.root.join(rel_path))
            .unwrap_or_else(|e| panic!("{rel_path}: {e} in {:?}", walk(&from.root)));
        let target = self.root.join(rel_path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, content).unwrap();
    }

    /// The file as it is on disk now, without writing the queued pages first
    fn on_disk(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
    }

    /// Gives the first item of the page a property, as an edit in the app
    fn edit(&self, page_id: &Hmtid, value: &str) {
        let items = self.state.page_service.get_page_items(page_id).unwrap();
        self.state
            .item_service
            .set_property(&items[0].id, "note", "text", value)
            .unwrap();
    }

    fn file(&self, rel_path: &str) -> String {
        self.export();
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
    }

    /// The paths the scan holds as deleted files, to ask the user about
    fn missing(&self) -> Vec<String> {
        self.state.reconcile.scan();
        self.state
            .file_sync_state
            .find_missing()
            .unwrap()
            .into_iter()
            .map(|s| s.rel_path)
            .collect()
    }

    fn sync(&self, rel_path: &str) -> Outcome {
        self.state.import.sync_file(&self.root, rel_path)
    }

    /// The id of the page in the directory, if there is one
    fn page_in(&self, dir_path: &str, name: &str) -> Option<Hmtid> {
        self.state
            .pages
            .find_by_type_dir_path_file_name("doc", dir_path, name)
            .unwrap()
            .map(|p| p.id)
    }

    fn doc(&self, name: &str) -> Hmtid {
        self.state
            .page_service
            .get_or_create_doc(Some(name), Some("d/"))
            .unwrap()
            .page
            .id
    }

    fn atlas_id(&self) -> Hmtid {
        self.state
            .pages
            .find_by_type_dir_path_file_name("doc", "d/", "index_atlas")
            .unwrap()
            .unwrap()
            .id
    }

    /// The outline of the atlas, each item as `level:body`
    fn outline(&self) -> Vec<String> {
        self.state
            .page_service
            .get_page_items(&self.atlas_id())
            .unwrap()
            .iter()
            .map(|i| format!("{}:{}", i.level, i.body))
            .collect()
    }

    /// The item of the card of a page on the atlas, and its properties
    fn card_bounds(&self, page_id: &Hmtid) -> Option<String> {
        let items = self
            .state
            .page_service
            .get_page_items(&self.atlas_id())
            .unwrap();
        let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        let props = self
            .state
            .properties
            .find_scalar_by_targets("item", &ids)
            .unwrap();
        props
            .values()
            .find(|p| p.get("atlas_page").map(String::as_str) == Some(page_id.as_str()))
            .and_then(|p| p.get("atlas_bounds").cloned())
    }

    fn card_item(&self, page_id: &Hmtid) -> Hmtid {
        let items = self
            .state
            .page_service
            .get_page_items(&self.atlas_id())
            .unwrap();
        let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        let props = self
            .state
            .properties
            .find_scalar_by_targets("item", &ids)
            .unwrap();
        let (id, _) = props
            .iter()
            .find(|(_, p)| p.get("atlas_page").map(String::as_str) == Some(page_id.as_str()))
            .unwrap();
        Hmtid::parse(id).unwrap()
    }

    /// The backups of files that lost to the database
    fn conflicts(&self) -> Vec<String> {
        walk(&self.root)
            .into_iter()
            .filter(|p| p.contains("conflict"))
            .collect()
    }
}

fn walk(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(walk(&path));
        } else {
            found.push(path.to_string_lossy().into_owned());
        }
    }
    found
}

const INDEX: &str = "docs/d/index.md";
const ATLAS: &str = "docs/d/index_atlas.md";

/// Two devices that share the directory `d/` with the page a and its atlas, made on A
fn two_devices(dir: &Path) -> (Device, Device) {
    two_devices_with(dir, Device::new)
}

fn two_devices_with(dir: &Path, make_b: fn(&Path) -> Device) -> (Device, Device) {
    let a = Device::new(&dir.join("a"));
    let b = make_b(&dir.join("b"));
    a.doc("a");
    a.doc("index_atlas");
    for rel_path in ["docs/index.md", INDEX, "docs/d/a.md", ATLAS] {
        b.receive(&a, rel_path);
    }
    (a, b)
}

// 2台のデバイスは、同じページから同じアトラスを書く。
#[test]
fn two_devices_write_the_same_atlas_from_the_same_pages() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    assert_eq!(a.file(ATLAS), b.file(ATLAS));

    a.doc("c");
    // B writes its atlas again when the new page comes, before the atlas of A comes
    b.receive(&a, "docs/d/c.md");

    assert_eq!(b.outline(), ["1:[[d/a]]", "1:[[d/c]]"]);
    assert_eq!(a.file(ATLAS), b.file(ATLAS));
    assert_eq!(b.receive(&a, ATLAS), Outcome::Unchanged);
    assert_eq!(a.receive(&b, ATLAS), Outcome::Unchanged);
    assert!(a.conflicts().is_empty() && b.conflicts().is_empty());
}

// 一方のデバイスで置いたカードは、アトラスがページより先に届いても位置を保つ。
#[test]
fn a_card_placed_on_one_device_keeps_its_place_when_the_atlas_comes_before_the_page() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = a.doc("c");
    a.state
        .item_service
        .set_property(&a.card_item(&c), "atlas_bounds", "bounds", "1,2,0,40")
        .unwrap();

    assert_eq!(b.receive(&a, ATLAS), Outcome::Imported);
    b.receive(&a, "docs/d/c.md");

    assert_eq!(b.card_bounds(&c).as_deref(), Some("1,2,0,40"));
    assert_eq!(b.outline(), ["1:[[d/a]]", "1:[[d/c]]"]);
    assert_eq!(a.file(ATLAS), b.file(ATLAS));
    assert!(b.conflicts().is_empty());
}

// 両方のデバイスで削除したページは、両方のアトラスを同じに保つ。
#[test]
fn a_page_deleted_on_both_devices_leaves_both_atlases_alike() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = a.doc("c");
    b.receive(&a, "docs/d/c.md");
    b.receive(&a, ATLAS);

    a.state.page_service.delete_page(&c).unwrap();
    b.receive(&a, ATLAS);

    // B takes the atlas as it came, while it has the page until it deletes it too, as the
    // question about the missing file asks
    assert_eq!(b.outline(), ["1:[[d/a]]"]);
    b.state.page_service.delete_page(&c).unwrap();
    a.receive(&b, ATLAS);

    assert_eq!(a.outline(), ["1:[[d/a]]"]);
    assert_eq!(a.file(ATLAS), b.file(ATLAS));
    // A change made on A afterwards is not older than the atlas of B
    a.doc("e");
    assert_eq!(b.receive(&a, ATLAS), Outcome::Imported);
    assert!(a.conflicts().is_empty() && b.conflicts().is_empty());
}

// 旧形式のアトラスを移した2台のデバイスは、同じアトラスを書く。
#[test]
fn two_devices_moving_the_atlas_of_old_write_the_same_atlas() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = a.doc("c");
    b.receive(&a, "docs/d/c.md");
    b.receive(&a, ATLAS);
    let page_a = a
        .state
        .pages
        .find_by_type_dir_path_file_name("doc", "d/", "a")
        .unwrap()
        .unwrap()
        .id;
    for device in [&a, &b] {
        let props = &device.state.properties;
        props
            .upsert_scalar(
                "page",
                page_a.as_str(),
                "atlas_bounds",
                "bounds",
                "1,2,0,40",
            )
            .unwrap();
        props
            .upsert_scalar(
                "page",
                device.atlas_id().as_str(),
                "atlas_callout_x",
                "text",
                "10,20,1.5,top:Hi",
            )
            .unwrap();
        let conn = device.state.db.conn().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO property_definitions (key, value_type, is_system, for_page) \
             VALUES ('atlas_group_x', 'list', 1, 1)",
            [],
        )
        .unwrap();
        for (i, id) in [&page_a, &c].iter().enumerate() {
            conn.execute(
                "INSERT INTO property_list_items (target_type, target_id, key, sort_order, value) \
                 VALUES ('page', ?, 'atlas_group_x', ?, ?)",
                rusqlite::params![device.atlas_id().as_str(), i as i64, id.as_str()],
            )
            .unwrap();
        }
    }

    a.state.atlas.move_old_properties().unwrap();
    b.state.atlas.move_old_properties().unwrap();

    assert_eq!(
        a.outline(),
        ["1:", "2:[[d/a]]", "2:[[d/c]]", "1:callout", "2:Hi"]
    );
    assert_eq!(a.file(ATLAS), b.file(ATLAS));
    assert_eq!(b.receive(&a, ATLAS), Outcome::Unchanged);
}

/// Delivers the atlas back and forth a few times; each device writes what it takes in, as
/// the cloud folder sends every change on
fn exchange(a: &Device, b: &Device) {
    for _ in 0..3 {
        b.receive(a, ATLAS);
        a.receive(b, ATLAS);
    }
}

// 改名より先に届いたアトラスは、届いたまま保たれる。
#[test]
fn an_atlas_that_comes_before_a_rename_is_kept_as_it_came() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = a.doc("c");
    b.receive(&a, "docs/d/c.md");
    b.receive(&a, ATLAS);

    a.state
        .page_service
        .update_page(&c, Some("c2"), None, false)
        .unwrap();
    let renamed = a.file(ATLAS);
    exchange(&a, &b);

    // B does not write its own atlas from the old name while the rename is on its way
    assert_eq!(b.file(ATLAS), renamed);
    assert_eq!(a.file(ATLAS), renamed);
    std::fs::remove_file(b.root.join("docs/d/c.md")).unwrap();
    assert_eq!(b.receive(&a, "docs/d/c2.md"), Outcome::Imported);
    assert_eq!(b.outline(), ["1:[[d/a]]", "1:[[d/c2]]"]);
    assert_eq!(b.file(ATLAS), renamed);
    assert!(a.conflicts().is_empty() && b.conflicts().is_empty());
}

// 移動より先に届いたアトラスは、届いたまま保たれる。
#[test]
fn an_atlas_that_comes_before_a_move_is_kept_as_it_came() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = a.doc("c");
    b.receive(&a, "docs/d/c.md");
    b.receive(&a, ATLAS);

    a.state
        .page_service
        .update_page(&c, None, Some("e/"), false)
        .unwrap();
    let moved = a.file(ATLAS);
    exchange(&a, &b);

    assert_eq!(b.file(ATLAS), moved);
    assert_eq!(a.file(ATLAS), moved);
    assert_eq!(a.outline(), ["1:[[d/a]]"]);
    // The cloud folder takes the file away from where it was, then brings it where it went
    std::fs::remove_file(b.root.join("docs/d/c.md")).unwrap();
    assert_eq!(b.receive(&a, "docs/e/c.md"), Outcome::Imported);
    assert_eq!(b.file(ATLAS), moved);
    assert!(a.conflicts().is_empty() && b.conflicts().is_empty());
}

/// The page c made on A and sent to B, then moved to `e/` on A
fn moved_on_a(a: &Device, b: &Device) -> Hmtid {
    let c = a.doc("c");
    b.receive(a, "docs/d/c.md");
    a.state
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
    a.state
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
    let old = b.state.file_sync_state.find_by_path("docs/d/c.md").unwrap();
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
    let sync_state = &b.state.file_sync_state;
    sync_state
        .upsert_synced("docs/d/c.md", &c, "old", 1, 1)
        .unwrap();
    sync_state.mark_missing("docs/d/c.md").unwrap();

    assert!(b.missing().is_empty());
}
