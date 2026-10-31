//! An atlas shared by two devices through their markdown folders, as a cloud folder keeps
//! them alike. Each device has a server state and a folder of its own. A file reaches the
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
        let root = dir.join("markdown");
        let config_dir = dir.join("config");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&config_dir).unwrap();
        std::fs::write(
            config_dir.join("config.json"),
            json!({ "markdownDir": root.to_string_lossy() }).to_string(),
        )
        .unwrap();
        let state =
            AppState::build(Settings::under(dir, 0), BuildOptions { interactive: false }).unwrap();
        Device { state, root }
    }

    fn export(&self) {
        self.state.export.process_export_queue();
    }

    /// Takes the file as the other device wrote it, as the cloud folder brings it.
    fn receive(&self, from: &Device, rel_path: &str) -> Outcome {
        from.export();
        let content = std::fs::read(from.root.join(rel_path))
            .unwrap_or_else(|e| panic!("{rel_path}: {e} in {:?}", walk(&from.root)));
        let target = self.root.join(rel_path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, content).unwrap();
        self.state.import.sync_file(&self.root, rel_path)
    }

    fn file(&self, rel_path: &str) -> String {
        self.export();
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
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
    let a = Device::new(&dir.join("a"));
    let b = Device::new(&dir.join("b"));
    // The index page of a folder made for a page of it has no file until it is written; both
    // devices are to have the same one
    let index = a.doc("index");
    a.state.export.queue_export(&index);
    a.doc("a");
    a.doc("index_atlas");
    for rel_path in ["docs/index.md", INDEX, "docs/d/a.md", ATLAS] {
        b.receive(&a, rel_path);
    }
    (a, b)
}

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

#[test]
fn a_page_deleted_on_both_devices_leaves_both_atlases_alike() {
    let dir = tempfile::tempdir().unwrap();
    let (a, b) = two_devices(dir.path());
    let c = a.doc("c");
    b.receive(&a, "docs/d/c.md");
    b.receive(&a, ATLAS);

    a.state.page_service.delete_page(&c).unwrap();
    b.receive(&a, ATLAS);

    // B has the page until it deletes it too, as the question about the missing file asks
    assert_eq!(b.outline(), ["1:[[d/a]]", "1:[[d/c]]"]);
    b.state.page_service.delete_page(&c).unwrap();
    a.receive(&b, ATLAS);

    assert_eq!(a.outline(), ["1:[[d/a]]"]);
    assert_eq!(a.file(ATLAS), b.file(ATLAS));
    // A change made on A afterwards is not older than the atlas of B
    a.doc("e");
    assert_eq!(b.receive(&a, ATLAS), Outcome::Imported);
    assert!(a.conflicts().is_empty() && b.conflicts().is_empty());
}

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
