use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

use crate::cloud::{Change, Cloud, CloudFile, SyncError};
use crate::folder::MarkdownFolder;
use crate::record::is_at_or_under;
use crate::Local;

/// Brings the Markdown folder of the app up to date with the folder in the cloud. It only
/// reads the cloud (ADR 030): what the cloud has is right, and what it lost is deleted here.
/// It writes down the versions it saw, for the upload (ADR 031).
///
/// A file that the cloud lost stays here when it holds what the cloud never had: a change of
/// the app that is not uploaded yet, or a page that was made in the app. The upload brings it
/// to the cloud as a new file. What was written on the phone is not lost in silence.
pub struct Downloader<'a> {
    cloud: &'a dyn Cloud,
    local: &'a Local,
    /// How many files are downloaded at the same time. One after the other, a file takes
    /// about a second.
    threads: usize,
}

impl<'a> Downloader<'a> {
    pub fn new(cloud: &'a dyn Cloud, local: &'a Local) -> Self {
        Downloader {
            cloud,
            local,
            threads: 8,
        }
    }

    /// Runs once. `progress` gets how many files are downloaded, of how many: first with 0,
    /// then after each file, in order but from other threads.
    ///
    /// When the cloud or a download fails, the error comes out and the cursor stays. The next
    /// run asks for the same changes again, and downloads only what is still missing.
    pub fn run(&self, progress: &(dyn Fn(u32, u32) + Sync)) -> Result<(), SyncError> {
        let record = &self.local.record;
        // Without a record, the changes since the cursor would leave most versions unknown
        let cursor = record
            .is_kept()
            .then(|| fs::read_to_string(&self.local.cursor).ok())
            .flatten();
        let changes = self.cloud.changes(cursor.as_deref())?;
        let mut files = Files::default();
        for change in changes.entries {
            match change {
                Change::Gone(path) => {
                    files.remove(&crate::fold(&path));
                    self.local.folder.remove(&path, |here| self.is_unsent(here));
                    record.remove(&path);
                }
                Change::File(file) => {
                    if MarkdownFolder::keeps(&crate::fold(&file.path)) {
                        files.put(file);
                    }
                }
            }
        }
        let files = files.in_order();
        if changes.whole {
            let paths: HashSet<String> = files.iter().map(|file| crate::fold(&file.path)).collect();
            self.local
                .folder
                .keep_only(&paths, |here| self.is_unsent(here));
            record.keep_only(&paths);
        }
        let missing: Vec<&CloudFile> = files
            .iter()
            .filter(|file| {
                self.hash_here(&file.path).as_deref() != Some(file.content_hash.as_str())
            })
            .collect();
        self.download(&missing, progress)?;
        for file in files {
            record.put(file);
        }
        record.save()?;
        fs::write(&self.local.cursor, changes.cursor)?;
        Ok(())
    }

    /// True for a file here whose content the cloud never had. Without a record, every file is
    /// a copy of the cloud: a version of the app that could not write downloaded it.
    fn is_unsent(&self, path: &str) -> bool {
        let record = &self.local.record;
        record.is_kept() && self.hash_here(path) != record.get(path).map(|file| file.content_hash)
    }

    fn hash_here(&self, path: &str) -> Option<String> {
        self.local
            .folder
            .content_hash(path, |file| self.cloud.content_hash(file))
    }

    /// Downloads the files on a few threads. Every file is tried; the first failure in the
    /// order of the files comes out.
    fn download(
        &self,
        files: &[&CloudFile],
        progress: &(dyn Fn(u32, u32) + Sync),
    ) -> Result<(), SyncError> {
        let total = count(files.len());
        progress(0, total);
        let next = AtomicUsize::new(0);
        let done = Mutex::new(0);
        let failures = Mutex::new(Vec::new());
        thread::scope(|scope| {
            for _ in 0..self.threads.min(files.len()) {
                scope.spawn(|| {
                    while let Some(file) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                        let put = self
                            .local
                            .folder
                            .put(&file.path, |to| self.cloud.download(file, to));
                        match put {
                            Ok(()) => {
                                let mut done = done.lock().unwrap();
                                *done += 1;
                                progress(*done, total);
                            }
                            Err(failure) => {
                                failures.lock().unwrap().push((file.path.clone(), failure))
                            }
                        }
                    }
                });
            }
        });
        let failures = failures.into_inner().unwrap();
        match files
            .iter()
            .find_map(|file| failures.iter().find(|(path, _)| *path == file.path))
        {
            Some((_, failure)) => Err(failure.clone()),
            None => Ok(()),
        }
    }
}

/// The files that a list names, by their path in lower case, as the cloud tells no case apart.
/// A file keeps the place where its path came first.
#[derive(Default)]
struct Files {
    order: Vec<String>,
    by_path: HashMap<String, CloudFile>,
}

impl Files {
    fn put(&mut self, file: CloudFile) {
        let key = crate::fold(&file.path);
        if !self.by_path.contains_key(&key) {
            self.order.push(key.clone());
        }
        self.by_path.insert(key, file);
    }

    /// Forgets the file, or the folder with all that is in it.
    fn remove(&mut self, key: &str) {
        self.by_path.retain(|path, _| !is_at_or_under(path, key));
        self.order.retain(|path| !is_at_or_under(path, key));
    }

    fn in_order(mut self) -> Vec<CloudFile> {
        self.order
            .iter()
            .filter_map(|key| self.by_path.remove(key))
            .collect()
    }
}

pub(crate) fn count(files: usize) -> u32 {
    u32::try_from(files).unwrap_or(u32::MAX)
}
