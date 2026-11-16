use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::cloud::CloudFile;

/// What the app knows of the files in the cloud: the version and the content hash of each, as
/// the last download or upload left it (ADR 031). A file here whose hash differs from it has
/// changed in the app. An upload names the version, and the cloud refuses when it has another.
///
/// A path is looked up without regard to case, as the cloud does.
///
/// The file is JSON: an object whose keys are the paths in lower case, and whose values are
/// [`CloudFile`]s. The iOS shell wrote this form before ADR 054. A file in another form, such as
/// the `Properties` of the Android shell before ADR 054, counts as no record.
pub struct CloudRecord {
    file: PathBuf,
    known: Mutex<Known>,
}

struct Known {
    files: BTreeMap<String, CloudFile>,
    /// Whether the file holds a record: see [`CloudRecord::is_kept`].
    kept: bool,
}

impl CloudRecord {
    /// `file` keeps the record between two runs of the app.
    pub fn new(file: impl Into<PathBuf>) -> Self {
        let file = file.into();
        let read = fs::read(&file)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<BTreeMap<String, CloudFile>>(&bytes).ok());
        let known = match read {
            Some(files) => Known { files, kept: true },
            None => Known {
                files: BTreeMap::new(),
                kept: false,
            },
        };
        CloudRecord {
            file,
            known: Mutex::new(known),
        }
    }

    /// The file as the cloud had it when the app last heard of it. `None` for a file it has not.
    pub fn get(&self, path: &str) -> Option<CloudFile> {
        self.lock().files.get(&path.to_lowercase()).cloned()
    }

    pub fn put(&self, version: CloudFile) {
        self.lock()
            .files
            .insert(version.path.to_lowercase(), version);
    }

    /// Forgets the file, or the folder with all that is in it: it has gone from the cloud.
    pub fn remove(&self, path: &str) {
        let key = path.to_lowercase();
        self.lock()
            .files
            .retain(|known, _| !is_at_or_under(known, &key));
    }

    /// Forgets every file whose path is not in `paths`, which are in lower case.
    pub fn keep_only(&self, paths: &HashSet<String>) {
        self.lock().files.retain(|known, _| paths.contains(known));
    }

    /// False when the app has written no record that it can read: nothing is known of the
    /// versions of its files, and every file here is taken for a copy of the cloud.
    pub fn is_kept(&self) -> bool {
        self.lock().kept
    }

    /// Writes the record to its file. The file is whole or as it was.
    pub fn save(&self) -> io::Result<()> {
        let mut known = self.lock();
        let json = serde_json::to_vec(&known.files)?;
        let part = self.file.with_extension("part");
        fs::write(&part, json)?;
        fs::rename(&part, &self.file)?;
        known.kept = true;
        Ok(())
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Known> {
        self.known.lock().unwrap()
    }
}

/// True when `path` is `place` itself or a path in the folder `place`.
pub(crate) fn is_at_or_under(path: &str, place: &str) -> bool {
    path == place
        || path
            .strip_prefix(place)
            .is_some_and(|rest| rest.starts_with('/'))
}
