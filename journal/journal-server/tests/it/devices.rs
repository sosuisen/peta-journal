//! Two devices that share their pages through their markdown folders, as a cloud folder keeps
//! them alike. Each device has a server state and a folder of its own. A file reaches the
//! other device when the test copies it, so the test decides the order the files arrive in.
//! The tests that still use them move to journal-core, which has its own devices (ADR 058).

use std::path::{Path, PathBuf};

use hmtid::Hmtid;
use journal_core::infrastructure::export::import_service::Outcome;
use journal_core::journal::AddPropertyDefinitionInput;
use journal_server::config::Settings;
use journal_server::state::{AppState, BuildOptions, SharedState};
use serde_json::json;

pub(super) struct Device {
    pub(super) state: SharedState,
    pub(super) root: PathBuf,
}

impl Device {
    fn new(dir: &Path) -> Device {
        // A scan that finds missing files opens the browser, unless it did within the
        // cooldown; one longer than the time since 1970 keeps it shut
        let settings = Settings {
            import_delete_prompt_cooldown_secs: 10_000_000_000,
            ..Settings::under(dir, 0)
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

    pub(super) fn export(&self) {
        self.state.export.process_export_queue();
    }

    /// Takes the file as the other device wrote it, as the cloud folder brings it.
    pub(super) fn receive(&self, from: &Device, rel_path: &str) -> Outcome {
        self.deliver(from, rel_path);
        self.sync(rel_path)
    }

    /// Puts the file the other device wrote into the folder, before the server takes it in
    pub(super) fn deliver(&self, from: &Device, rel_path: &str) {
        from.export();
        let content = std::fs::read(from.root.join(rel_path))
            .unwrap_or_else(|e| panic!("{rel_path}: {e} in {:?}", walk(&from.root)));
        let target = self.root.join(rel_path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, content).unwrap();
    }

    /// Defines a key, as the user does in Settings
    pub(super) fn define(&self, key: &str, value_type: &str, page: bool) {
        self.state
            .journal
            .add_property_definition(AddPropertyDefinitionInput {
                key: Some(key.to_string()),
                value_type: Some(value_type.to_string()),
                page: Some(page),
            })
            .unwrap();
    }

    /// Defines a key with the type its name gives, unless it is defined already
    pub(super) fn ensure_defined(&self, key: &str) {
        if self
            .state
            .properties
            .find_definition(key)
            .unwrap()
            .is_none()
        {
            self.define(key, property_names::type_of(key).as_str(), true);
        }
    }

    /// Gives the first item of the page a property, as an edit in the app
    pub(super) fn edit(&self, page_id: &Hmtid, value: &str) {
        self.ensure_defined("note");
        let items = self.state.page_service.get_page_items(page_id).unwrap();
        self.state
            .item_service
            .set_property(&items[0].id, "note", "text", value)
            .unwrap();
    }

    pub(super) fn file(&self, rel_path: &str) -> String {
        self.export();
        std::fs::read_to_string(self.root.join(rel_path)).unwrap()
    }

    pub(super) fn sync(&self, rel_path: &str) -> Outcome {
        self.state.import.sync_file(&self.root, rel_path)
    }

    /// The id of the page in the directory, if there is one
    pub(super) fn page_in(&self, dir_path: &str, name: &str) -> Option<Hmtid> {
        self.state
            .pages
            .find_by_type_dir_path_file_name("doc", dir_path, name)
            .unwrap()
            .map(|p| p.id)
    }

    pub(super) fn doc(&self, name: &str) -> Hmtid {
        self.state
            .page_service
            .get_or_create_doc(Some(name), Some("d/"))
            .unwrap()
            .page
            .id
    }

    /// The backups of files that lost to the database
    pub(super) fn conflicts(&self) -> Vec<String> {
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
pub(super) const ATLAS: &str = "docs/d/index_atlas.md";

/// Two devices that share the directory `d/` with the page a and its atlas, made on A
pub(super) fn two_devices(dir: &Path) -> (Device, Device) {
    let a = Device::new(&dir.join("a"));
    let b = Device::new(&dir.join("b"));
    a.doc("a");
    a.doc("index_atlas");
    for rel_path in ["docs/index.md", INDEX, "docs/d/a.md", ATLAS] {
        b.receive(&a, rel_path);
    }
    (a, b)
}
