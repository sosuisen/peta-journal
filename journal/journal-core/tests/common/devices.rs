//! Two devices that share their pages through their markdown folders, as a cloud folder
//! keeps them alike (ADR 058). Each device has a core and a markdown folder of its own. A
//! file reaches the other device when the test copies it, so the test decides the order the
//! files arrive in.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use hmtid::Hmtid;
use journal_core::assembly::JournalCore;
use journal_core::clock::SteppedClock;
use journal_core::infrastructure::export::import_service::Outcome;
use journal_core::journal::AddPropertyDefinitionInput;

use super::{start, Recorded, WAIT};

pub struct Device {
    pub core: JournalCore,
    /// The markdown folder of the device
    pub root: PathBuf,
    /// What the device told its clients
    pub told: Arc<Recorded>,
    /// The clock of the device. The devices of a test each have their own
    pub clock: Arc<SteppedClock>,
}

/// A moment passes on the devices: what they change next is dated later than what they
/// changed before. Two devices take the newer of two dates, so a test that tells which
/// change was the later one says so with this.
pub fn time_passes(devices: &[&Device]) {
    for device in devices {
        device.clock.advance(5);
    }
}

impl Device {
    pub fn new(dir: &Path) -> Device {
        Device::with(dir, WAIT)
    }

    /// A device that does not wait: it takes a file with the id of a page still elsewhere as
    /// a copy at once, and writes a page over a file not taken in yet at once
    pub fn not_waiting(dir: &Path) -> Device {
        Device::with(dir, Duration::ZERO)
    }

    fn with(dir: &Path, wait: Duration) -> Device {
        let told = Arc::new(Recorded::default());
        let clock = Arc::new(SteppedClock::default());
        let (core, root) = start(dir, wait, told.clone(), clock.clone());
        Device {
            core,
            root,
            told,
            clock,
        }
    }

    pub fn export(&self) {
        self.core.export.process_export_queue();
    }

    /// Takes the file as the other device wrote it, as the cloud folder brings it.
    pub fn receive(&self, from: &Device, rel_path: &str) -> Outcome {
        self.deliver(from, rel_path);
        self.sync(rel_path)
    }

    /// Puts the file the other device wrote into the folder, before the device takes it in
    pub fn deliver(&self, from: &Device, rel_path: &str) {
        from.export();
        let content = std::fs::read(from.root.join(rel_path))
            .unwrap_or_else(|e| panic!("{rel_path}: {e} in {:?}", walk(&from.root)));
        let target = self.root.join(rel_path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, content).unwrap();
    }

    /// The file as it is on disk now, without writing the queued pages first
    pub fn on_disk(&self, rel_path: &str) -> String {
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
    }

    /// Defines a key, as the user does in Settings
    pub fn define(&self, key: &str, value_type: &str, page: bool) {
        self.core
            .journal
            .add_property_definition(AddPropertyDefinitionInput {
                key: Some(key.to_string()),
                value_type: Some(value_type.to_string()),
                page: Some(page),
            })
            .unwrap();
    }

    /// Defines a key with the type its name gives, unless it is defined already
    pub fn ensure_defined(&self, key: &str) {
        if self.core.properties.find_definition(key).unwrap().is_none() {
            self.define(key, property_names::type_of(key).as_str(), true);
        }
    }

    /// Gives the first item of the page a property, as an edit in the app
    pub fn edit(&self, page_id: &Hmtid, value: &str) {
        self.ensure_defined("note");
        let items = self.core.page_service.get_page_items(page_id).unwrap();
        self.core
            .item_service
            .set_property(&items[0].id, "note", "text", value)
            .unwrap();
    }

    pub fn file(&self, rel_path: &str) -> String {
        self.export();
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
    }

    /// The paths the scan holds as deleted files, to ask the user about
    pub fn missing(&self) -> Vec<String> {
        self.core.reconcile.scan();
        self.core
            .file_sync_state
            .find_missing()
            .unwrap()
            .into_iter()
            .map(|s| s.rel_path)
            .collect()
    }

    pub fn sync(&self, rel_path: &str) -> Outcome {
        self.core.import.sync_file(&self.root, rel_path)
    }

    /// The id of the page in the directory, if there is one
    pub fn page_in(&self, dir_path: &str, name: &str) -> Option<Hmtid> {
        self.core
            .pages
            .find_by_type_dir_path_file_name("doc", dir_path, name)
            .unwrap()
            .map(|p| p.id)
    }

    pub fn doc(&self, name: &str) -> Hmtid {
        self.core
            .page_service
            .get_or_create_doc(Some(name), Some("d/"))
            .unwrap()
            .page
            .id
    }

    /// The backups of files that lost to the database
    pub fn conflicts(&self) -> Vec<String> {
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

pub const INDEX: &str = "docs/d/index.md";
pub const ATLAS: &str = "docs/d/index_atlas.md";

/// Two devices that share the directory `d/` with the page a and its atlas, made on A
pub fn two_devices(dir: &Path) -> (Device, Device) {
    two_devices_with(dir, Device::new)
}

/// Two devices as `two_devices` makes them, with B made by the function given
pub fn two_devices_with(dir: &Path, make_b: fn(&Path) -> Device) -> (Device, Device) {
    let a = Device::new(&dir.join("a"));
    let b = make_b(&dir.join("b"));
    a.doc("a");
    a.doc("index_atlas");
    for rel_path in ["docs/index.md", INDEX, "docs/d/a.md", ATLAS] {
        b.receive(&a, rel_path);
    }
    (a, b)
}
