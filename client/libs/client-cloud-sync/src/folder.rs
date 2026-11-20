use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use crate::cloud::SyncError;

/// The Markdown folder of the app: a copy of the pages and their images in the folder in the
/// cloud (ADR 030).
///
/// The cloud does not tell `Notes.md` from `notes.md`, and the file system of a phone does. So
/// a path of the cloud is looked up here without regard to case, and a file never lies here
/// under two spellings.
pub struct MarkdownFolder {
    root: PathBuf,
    temp: PathBuf,
    /// One file is placed at a time: two downloads may need the same new folder.
    placing: Mutex<()>,
    /// Names the files in `temp` apart.
    parts: AtomicU64,
}

impl MarkdownFolder {
    /// The extension of a page.
    pub const MARKDOWN: &'static str = ".md";
    /// The folder of the images of the pages, with the `/` after it.
    pub const ASSETS: &'static str = "assets/";
    /// The definitions of the properties, which the core takes in (ADR 046).
    pub const DEFINITIONS: &'static str = "settings/properties.json";

    /// `root` is the folder the journal core takes the files in from. `temp` is where a file
    /// is written before it is whole. It is not under `root`: the core must never take in
    /// half a file.
    pub fn new(root: impl Into<PathBuf>, temp: impl Into<PathBuf>) -> Self {
        MarkdownFolder {
            root: root.into(),
            temp: temp.into(),
            placing: Mutex::new(()),
            parts: AtomicU64::new(0),
        }
    }

    /// True for a file that the app copies from the cloud: a page, an image of a page, or the
    /// definitions of the properties. `path` is in lower case. The cloud holds more, such as
    /// the settings of other apps.
    pub fn keeps(path: &str) -> bool {
        path.ends_with(Self::MARKDOWN)
            || path.starts_with(Self::ASSETS)
            || path == Self::DEFINITIONS
    }

    /// Deletes the file, or the folder with all that is in it, but the files that `keep`
    /// wants: it gets the path of each file as it is spelled here. Nothing there is fine.
    pub fn remove(&self, path: &str, keep: impl Fn(&str) -> bool) {
        let Some(found) = find_without_case(&self.root, path) else {
            return;
        };
        for place in bottom_up(&found) {
            // A folder that still holds a kept file does not go, and that is right
            if place.is_dir() {
                let _ = fs::remove_dir(&place);
            } else if !keep(&self.spelled(&place)) {
                let _ = fs::remove_file(&place);
            }
        }
    }

    /// The hash of the content of the file, as `of` makes it: each cloud has its own way.
    /// `None` when the file is not here, or cannot be read.
    pub fn content_hash(
        &self,
        path: &str,
        of: impl Fn(&Path) -> io::Result<String>,
    ) -> Option<String> {
        find_without_case(&self.root, path)
            .filter(|found| found.is_file())
            .and_then(|found| of(&found).ok())
    }

    /// The paths of the files that the app writes, as they are spelled here, in order: the
    /// files it copies from the cloud. The images are among them: the user pastes them in
    /// the app (ADR 071).
    pub fn written(&self) -> Vec<String> {
        let mut written: Vec<String> = bottom_up(&self.root)
            .into_iter()
            .filter(|place| place.is_file())
            .map(|file| self.spelled(&file))
            .filter(|path| Self::keeps(&crate::fold(path)))
            .collect();
        written.sort();
        written
    }

    /// The file at a path that [`MarkdownFolder::written`] gave.
    pub fn file(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    /// Puts a file at the path, with the spelling of the path. `write` gets the path of a file
    /// to write its content to. The file appears whole or not at all.
    pub fn put(
        &self,
        path: &str,
        write: impl FnOnce(&Path) -> Result<(), SyncError>,
    ) -> Result<(), SyncError> {
        fs::create_dir_all(&self.temp)?;
        let number = self.parts.fetch_add(1, Ordering::Relaxed);
        let part = self
            .temp
            .join(format!("download-{}-{number}.part", std::process::id()));
        let placed = write(&part).and_then(|()| self.place(&part, path));
        let _ = fs::remove_file(&part);
        placed
    }

    fn place(&self, part: &Path, path: &str) -> Result<(), SyncError> {
        let _one = self.placing.lock().unwrap();
        let mut folders: Vec<&str> = parts(path).collect();
        // A path without a name makes no file: the root is a folder
        let last = folders.pop().unwrap_or_default();
        let mut dir = self.root.clone();
        for name in folders {
            dir = match child(&dir, name) {
                Some(found) => found,
                None => {
                    let made = dir.join(name);
                    fs::create_dir(&made)?;
                    made
                }
            };
        }
        // The same file under another spelling, as after it was renamed in the cloud
        if let Some(old) = child(&dir, last) {
            let _ = fs::remove_file(old);
        }
        fs::rename(part, dir.join(last))?;
        Ok(())
    }

    /// Deletes every file that the app copies and whose path is not in `paths`, which are in
    /// lower case. It is for a list of all that the cloud holds: what is not in it has gone.
    /// A file that `keep` wants stays: it gets the path as it is spelled here.
    pub fn keep_only(&self, paths: &HashSet<String>, keep: impl Fn(&str) -> bool) {
        for file in bottom_up(&self.root)
            .into_iter()
            .filter(|place| place.is_file())
        {
            let path = self.spelled(&file);
            let lower = crate::fold(&path);
            if Self::keeps(&lower) && !paths.contains(&lower) && !keep(&path) {
                let _ = fs::remove_file(&file);
            }
        }
    }

    /// The path of a file under the root, with `/` between its parts.
    fn spelled(&self, file: &Path) -> String {
        let relative = file.strip_prefix(&self.root).unwrap_or(file);
        relative
            .components()
            .map(|part| part.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// The file or folder at the path under `root`, however its name is spelled there. The path of
/// the cloud, and the path that a page writes for its image, may differ from it in case.
///
/// Empty parts of the path are left out, so a path may start with `/` (ADR 054).
pub fn find_without_case(root: &Path, path: &str) -> Option<PathBuf> {
    parts(path).try_fold(root.to_path_buf(), |dir, name| child(&dir, name))
}

fn parts(path: &str) -> impl Iterator<Item = &str> {
    path.split('/').filter(|name| !name.is_empty())
}

fn child(dir: &Path, name: &str) -> Option<PathBuf> {
    let name = crate::fold(name);
    fs::read_dir(dir)
        .ok()?
        .flatten()
        .find(|entry| crate::fold(&entry.file_name().to_string_lossy()) == name)
        .map(|entry| entry.path())
}

/// The place, and all that is in it when it is a folder: the files and folders in a folder
/// come before the folder.
fn bottom_up(place: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if place.is_dir() {
        for entry in fs::read_dir(place).into_iter().flatten().flatten() {
            found.extend(bottom_up(&entry.path()));
        }
    }
    found.push(place.to_path_buf());
    found
}
