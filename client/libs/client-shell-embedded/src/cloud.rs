//! The sync with Dropbox for the shells (ADR 054). The rules are in `client-cloud-sync`; this is
//! their entrance. The shell gives the calls of its Dropbox SDK ([`PetaDropboxSdk`]) and what the
//! sync asks of the app ([`PetaSyncHost`]).

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::PetaCore;

use client_cloud_sync::{
    CloudRecord, CloudSync, DropboxCloud, DropboxEntry, DropboxError, DropboxFile, DropboxFiles,
    Listing, Local, MarkdownFolder, SyncError, SyncListener, SyncState,
};

/// A file as Dropbox tells of it. The path is as Dropbox shows it, with the case of the names.
/// The SDK may give no path and no hash.
#[boltffi::data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PetaDropboxFile {
    pub path: Option<String>,
    pub content_hash: Option<String>,
    pub rev: String,
}

/// What Dropbox tells of one path.
#[boltffi::data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PetaDropboxEntry {
    File { file: PetaDropboxFile },
    Deleted { path: Option<String> },
    Folder { path: Option<String> },
}

/// One part of a list. `cursor` names the state after it. `has_more` says that a part follows.
#[boltffi::data]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PetaListing {
    pub entries: Vec<PetaDropboxEntry>,
    pub cursor: String,
    pub has_more: bool,
}

/// Why Dropbox did not do what it was asked. The shell tells these apart from the errors of
/// its SDK.
#[boltffi::error]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PetaDropboxError {
    /// Dropbox forgot a cursor that was too old.
    CursorForgotten,
    /// Dropbox has another version than the one that an upload was to replace, or a file where
    /// none was expected.
    UploadConflict,
    /// Dropbox could not be reached, as with no connection.
    Offline { why: String },
    /// Any other failure, in the words of the SDK, for the log.
    Other { why: String },
}

/// An error that the shell threw and did not declare, such as one of its SDK that it let out.
impl From<boltffi::UnexpectedFfiCallbackError> for PetaDropboxError {
    fn from(failure: boltffi::UnexpectedFfiCallbackError) -> Self {
        PetaDropboxError::Other { why: failure.0 }
    }
}

/// The calls of the Dropbox SDK of the shell. The sync calls them on its own threads, several
/// at a time, and never on the main thread. Each waits for the answer of Dropbox.
#[boltffi::export]
pub trait PetaDropboxSdk: Send + Sync {
    /// The first part of all that is in the folder at the path, and in the folders in it.
    fn list(&self, folder: String) -> Result<PetaListing, PetaDropboxError>;

    /// The part after `cursor`: the next part of a list, or what changed since a list ended.
    fn list_after(&self, cursor: String) -> Result<PetaListing, PetaDropboxError>;

    /// Writes the content of the version `rev` of a file to the file at the path `to`.
    fn download(&self, rev: String, to: String) -> Result<(), PetaDropboxError>;

    /// Puts the content of the file at the path `from` at the path in Dropbox, with no other
    /// name. `over` is the version that this content replaces, and none for a file that
    /// Dropbox has not. `modified_ms` is the time of the last change, in milliseconds since
    /// 1970.
    fn upload(
        &self,
        path: String,
        from: String,
        over: Option<String>,
        modified_ms: i64,
    ) -> Result<PetaDropboxFile, PetaDropboxError>;
}

/// Where the sync with Dropbox stands, for the screen.
#[boltffi::data]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PetaSyncState {
    /// The user has not signed in to Dropbox yet.
    SignedOut,
    /// Nothing runs.
    Idle,
    Downloading {
        done: u32,
        total: u32,
    },
    Uploading {
        done: u32,
        total: u32,
    },
    /// The last sync failed. `offline` says that Dropbox could not be reached. The screen
    /// tells only that; the reason went to the log.
    Failed {
        offline: bool,
    },
}

/// What the sync asks of the app.
#[boltffi::export]
pub trait PetaSyncHost: Send + Sync {
    /// Lets the core take in the files of the Markdown folder and write the pages that changed
    /// in the app, and returns when the core is done. It is called on a thread of the sync,
    /// never on the main thread.
    fn take_in(&self);

    /// The sync stands at `state` now. `in_front` says that the user waits for the sync that
    /// runs. It is called on a thread of the sync, one call at a time, in order. It must not
    /// call [`PetaCloudSync`]: it hands the state to the main thread and returns.
    fn told(&self, state: PetaSyncState, in_front: bool);

    /// Keeps why a sync failed, for the one who looks into it.
    fn log(&self, failure: String);
}

/// The syncs of the Markdown folder of the app with a folder in Dropbox, one at a time (ADR 030,
/// ADR 031, ADR 054).
#[boltffi::export]
pub struct PetaCloudSync {
    sync: CloudSync,
}

#[boltffi::export]
impl PetaCloudSync {
    /// The sync puts the downloads in the Markdown folder of `core`. `temp_dir` is where a
    /// download is written before it is whole; it is not under the Markdown folder.
    /// `cursor_file` keeps where the last download ended, and `record_file` the versions of
    /// the files in Dropbox that the app has seen.
    ///
    /// `core` tells the sync when a page changed in the app: the sync starts when no page has
    /// changed for `after_change_ms` (ADR 055).
    pub fn new(
        core: &PetaCore,
        after_change_ms: u64,
        temp_dir: String,
        cursor_file: String,
        record_file: String,
        signed_in: bool,
        host: Arc<dyn PetaSyncHost>,
    ) -> Self {
        let local = Local {
            folder: MarkdownFolder::new(&core.markdown, temp_dir),
            cursor: cursor_file.into(),
            record: CloudRecord::new(record_file),
        };
        let taker = Arc::clone(&host);
        let sync = CloudSync::new(
            local,
            signed_in,
            Duration::from_millis(after_change_ms),
            move || {
                taker.take_in();
                Ok(())
            },
            Listener { host },
        );
        *core.door.on_change.write().unwrap() = Some(Box::new(sync.on_change()));
        PetaCloudSync { sync }
    }

    /// Starts a sync with the folder at the path `folder` in Dropbox, such as `/PetaJournal`.
    /// While one runs, another starts after it, once. `in_front` says that the user waits for
    /// it: the app came to the front.
    pub fn start(&self, dropbox: Arc<dyn PetaDropboxSdk>, folder: String, in_front: bool) {
        let cloud = DropboxCloud::new(Sdk { dropbox }, &folder);
        self.sync.start(Arc::new(cloud), in_front);
    }

    /// Where the sync stands now. What `PetaSyncHost.told` tells may be old by the time the
    /// shell reads it on its main thread: a sync may have started since.
    pub fn state(&self) -> PetaSyncState {
        state(self.sync.state())
    }

    /// Whether the user waits for the sync that runs now.
    pub fn in_front(&self) -> bool {
        self.sync.in_front()
    }

    /// The user has signed out of Dropbox. The cursor goes, and the downloaded files and the
    /// record stay. Not for the time a sync runs.
    pub fn sign_out(&self) {
        self.sync.sign_out();
    }
}

struct Listener {
    host: Arc<dyn PetaSyncHost>,
}

impl SyncListener for Listener {
    fn told(&self, state_now: SyncState, in_front: bool) {
        self.host.told(state(state_now), in_front);
    }

    fn log(&self, failure: &SyncError) {
        self.host.log(format!("{failure:?}"));
    }
}

fn state(state: SyncState) -> PetaSyncState {
    match state {
        SyncState::SignedOut => PetaSyncState::SignedOut,
        SyncState::Idle => PetaSyncState::Idle,
        SyncState::Downloading { done, total } => PetaSyncState::Downloading { done, total },
        SyncState::Uploading { done, total } => PetaSyncState::Uploading { done, total },
        SyncState::Failed { offline } => PetaSyncState::Failed { offline },
    }
}

/// The Dropbox SDK of the shell, as the sync calls it.
struct Sdk {
    dropbox: Arc<dyn PetaDropboxSdk>,
}

impl DropboxFiles for Sdk {
    fn list(&self, folder: &str) -> Result<Listing, DropboxError> {
        self.dropbox
            .list(folder.to_string())
            .map(listing)
            .map_err(error)
    }

    fn list_after(&self, cursor: &str) -> Result<Listing, DropboxError> {
        self.dropbox
            .list_after(cursor.to_string())
            .map(listing)
            .map_err(error)
    }

    fn download(&self, rev: &str, to: &Path) -> Result<(), DropboxError> {
        self.dropbox
            .download(rev.to_string(), text(to))
            .map_err(error)
    }

    fn upload(
        &self,
        path: &str,
        from: &Path,
        over: Option<&str>,
        modified_ms: i64,
    ) -> Result<DropboxFile, DropboxError> {
        self.dropbox
            .upload(
                path.to_string(),
                text(from),
                over.map(str::to_string),
                modified_ms,
            )
            .map(file)
            .map_err(error)
    }
}

fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn listing(part: PetaListing) -> Listing {
    Listing {
        entries: part.entries.into_iter().map(entry).collect(),
        cursor: part.cursor,
        has_more: part.has_more,
    }
}

fn entry(entry: PetaDropboxEntry) -> DropboxEntry {
    match entry {
        PetaDropboxEntry::File { file: found } => DropboxEntry::File(file(found)),
        PetaDropboxEntry::Deleted { path } => DropboxEntry::Deleted { path },
        PetaDropboxEntry::Folder { path } => DropboxEntry::Folder { path },
    }
}

fn file(file: PetaDropboxFile) -> DropboxFile {
    DropboxFile {
        path: file.path,
        content_hash: file.content_hash,
        rev: file.rev,
    }
}

fn error(failure: PetaDropboxError) -> DropboxError {
    match failure {
        PetaDropboxError::CursorForgotten => DropboxError::CursorForgotten,
        PetaDropboxError::UploadConflict => DropboxError::UploadConflict,
        PetaDropboxError::Offline { why } => DropboxError::Offline(why),
        PetaDropboxError::Other { why } => DropboxError::Other(why),
    }
}

#[cfg(test)]
#[path = "cloud_tests.rs"]
mod tests;
