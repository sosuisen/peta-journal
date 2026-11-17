use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use client_cloud_sync::{
    Change, Changes, Cloud, CloudFile, CloudRecord, Downloader, Local, MarkdownFolder, SyncError,
    Uploader,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

/// A cloud of a test: the versions of its files, and the changes it tells of at each cursor.
#[derive(Default)]
pub struct FakeCloud {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    /// The content of each version of a file, by how the cloud finds the version.
    contents: HashMap<String, Vec<u8>>,
    /// The newest version of each file, by its path in lower case.
    newest: HashMap<String, CloudFile>,
    /// What `changes` answers, by the cursor it is asked with.
    answers: HashMap<Option<String>, Changes>,
    asked_with: Vec<Option<String>>,
    downloaded: Vec<String>,
    uploaded: Vec<String>,
    /// The path whose download or upload fails, as with no connection.
    broken: Option<String>,
}

impl FakeCloud {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap()
    }

    /// Gives the file of the cloud a new version, and answers with the change that tells of it.
    pub fn file(&self, path: &str, text: &str) -> Change {
        Change::File(self.lock().file(path, text.as_bytes()))
    }

    /// Takes the file, or the folder with all that is in it, out of the cloud.
    pub fn gone(&self, path: &str) -> Change {
        let key = path.to_lowercase();
        self.lock()
            .newest
            .retain(|known, _| *known != key && !known.starts_with(&format!("{key}/")));
        Change::Gone(path.to_string())
    }

    /// The text of the newest version of the file, or `None` when the cloud has no such file.
    pub fn text(&self, path: &str) -> Option<String> {
        let state = self.lock();
        let version = state.newest.get(&path.to_lowercase())?;
        Some(String::from_utf8(state.contents[&version.remote].clone()).unwrap())
    }

    /// Makes `changes` answer the entries when it is asked with `cursor`.
    pub fn answer(&self, cursor: Option<&str>, entries: Vec<Change>, next: &str, whole: bool) {
        self.lock().answers.insert(
            cursor.map(str::to_string),
            Changes {
                entries,
                cursor: next.to_string(),
                whole,
            },
        );
    }

    pub fn asked_with(&self) -> Vec<Option<String>> {
        self.lock().asked_with.clone()
    }

    pub fn downloaded(&self) -> Vec<String> {
        self.lock().downloaded.clone()
    }

    pub fn uploaded(&self) -> Vec<String> {
        self.lock().uploaded.clone()
    }

    pub fn forget_downloads(&self) {
        self.lock().downloaded.clear();
    }

    pub fn forget_uploads(&self) {
        self.lock().uploaded.clear();
    }

    /// Makes the download and the upload of the path fail, or none with `None`.
    pub fn break_at(&self, path: Option<&str>) {
        self.lock().broken = path.map(str::to_string);
    }
}

impl State {
    fn file(&mut self, path: &str, content: &[u8]) -> CloudFile {
        let version = CloudFile {
            path: path.to_string(),
            content_hash: hash_of(content),
            remote: format!("{path}#{}", self.contents.len()),
        };
        self.contents
            .insert(version.remote.clone(), content.to_vec());
        self.newest.insert(path.to_lowercase(), version.clone());
        version
    }

    fn check(&self, path: &str) -> Result<(), SyncError> {
        match &self.broken {
            Some(broken) if broken == path => Err(SyncError::Offline("the network is gone".into())),
            _ => Ok(()),
        }
    }
}

impl Cloud for FakeCloud {
    fn changes(&self, cursor: Option<&str>) -> Result<Changes, SyncError> {
        let mut state = self.lock();
        state.asked_with.push(cursor.map(str::to_string));
        Ok(state.answers[&cursor.map(str::to_string)].clone())
    }

    fn download(&self, file: &CloudFile, to: &Path) -> Result<(), SyncError> {
        let content = {
            let mut state = self.lock();
            state.check(&file.path)?;
            state.downloaded.push(file.path.clone());
            state.contents[&file.remote].clone()
        };
        fs::write(to, content)?;
        Ok(())
    }

    fn upload(&self, path: &str, from: &Path, over: Option<&str>) -> Result<CloudFile, SyncError> {
        let mut state = self.lock();
        state.check(path)?;
        let newest = state
            .newest
            .get(&path.to_lowercase())
            .map(|file| file.remote.as_str());
        if newest != over {
            return Err(SyncError::ChangedInCloud(path.to_string()));
        }
        state.uploaded.push(path.to_string());
        Ok(state.file(path, &fs::read(from)?))
    }

    fn content_hash(&self, file: &Path) -> io::Result<String> {
        Ok(hash_of(&fs::read(file)?))
    }
}

pub fn sha256(bytes: &[u8]) -> Vec<u8> {
    Sha256::digest(bytes).to_vec()
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The content hash of the cloud of a test. It is not the one of Dropbox: the sync asks the cloud.
pub fn hash_of(content: &[u8]) -> String {
    hex(&sha256(content))
}

/// What the tests of the sync start from: a cloud of a test, and the folder of the app.
pub struct Fixture {
    pub dir: TempDir,
    pub cloud: Arc<FakeCloud>,
    pub root: PathBuf,
    pub local: Local,
}

impl Fixture {
    pub fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let root = dir.path().join("markdown");
        fs::create_dir(&root).unwrap();
        let local = local(dir.path(), &root);
        Fixture {
            dir,
            cloud: Arc::new(FakeCloud::default()),
            root,
            local,
        }
    }

    /// Makes the parts of the sync again, as the app does when it opens.
    pub fn open(&mut self) {
        self.local = local(self.dir.path(), &self.root);
    }

    pub fn record_file(&self) -> PathBuf {
        self.dir.path().join("record")
    }

    /// Downloads the files, as the first sync of the app does.
    pub fn download_first(&self, files: &[(&str, &str)]) {
        let entries = files
            .iter()
            .map(|(path, text)| self.cloud.file(path, text))
            .collect();
        self.cloud.answer(None, entries, "c1", true);
        self.download().unwrap();
    }

    /// Downloads what changed in the cloud after the cursor `since`.
    pub fn download_since(&self, since: &str, next: &str, whole: bool, changes: Vec<Change>) {
        self.cloud.answer(Some(since), changes, next, whole);
        self.download().unwrap();
    }

    pub fn download(&self) -> Result<(), SyncError> {
        Downloader::new(self.cloud.as_ref(), &self.local).run(&|_, _| {})
    }

    pub fn upload(&self) -> Result<(), SyncError> {
        Uploader::new(self.cloud.as_ref(), &self.local).run(&|_, _| {})
    }

    /// Writes the file of a page, as the core does after an edit in the app.
    pub fn write(&self, path: &str, text: &str) {
        let file = self.root.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, text).unwrap();
    }

    /// The paths of the files under the Markdown folder, in order.
    pub fn here(&self) -> Vec<String> {
        self.texts().into_keys().collect()
    }

    /// The files under the Markdown folder, by their path, with their text.
    pub fn texts(&self) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        collect(&self.root, &self.root, &mut found);
        found
    }
}

fn local(dir: &Path, root: &Path) -> Local {
    Local {
        folder: MarkdownFolder::new(root, dir.join("temp")),
        cursor: dir.join("cursor"),
        record: CloudRecord::new(dir.join("record")),
    }
}

fn collect(root: &Path, dir: &Path, found: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(dir).unwrap().flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(root, &path, found);
        } else {
            let relative = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            found.insert(relative, fs::read_to_string(&path).unwrap());
        }
    }
}

/// The map of the paths and texts, for a comparison.
pub fn texts(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .collect()
}

pub fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}
