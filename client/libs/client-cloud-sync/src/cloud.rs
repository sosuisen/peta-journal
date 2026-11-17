use std::io;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// What the sync needs of the cloud storage: the Markdown folder there. A path is relative to
/// that folder, with `/` between its parts and none at its start.
///
/// The trait follows what Dropbox offers. The cloud tells what changed since a cursor. It
/// takes an upload only over the version it is told. It does not tell names apart by upper
/// and lower case.
pub trait Cloud: Send + Sync {
    /// What changed in the folder since `cursor`, in the order it happened. Without a cursor,
    /// or when the cloud no longer knows the cursor, it is everything that is in the folder.
    fn changes(&self, cursor: Option<&str>) -> Result<Changes, SyncError>;

    /// Writes the content of the file to the path `to`, which is a file here.
    fn download(&self, file: &CloudFile, to: &Path) -> Result<(), SyncError>;

    /// Puts the content of the file `from` at the path. `over` names the version that this
    /// content replaces, as [`CloudFile::remote`] does, and it is `None` for a file that the
    /// cloud has not. Answers with the file as the cloud has it now.
    ///
    /// The error is [`SyncError::ChangedInCloud`] when the cloud has another version than
    /// `over`, or a file where none was expected. Nothing is written then.
    fn upload(&self, path: &str, from: &Path, over: Option<&str>) -> Result<CloudFile, SyncError>;

    /// The hash that this cloud makes of the content of `file`, which is a file here. It is
    /// what [`CloudFile::content_hash`] is for the same content, so it tells whether a file
    /// here has the content of the file in the cloud, with no download.
    fn content_hash(&self, file: &Path) -> io::Result<String>;
}

/// One version of a file in the cloud.
///
/// The record of the app keeps it as JSON with these names, as the iOS shell wrote it before
/// ADR 054.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudFile {
    pub path: String,
    /// The hash that the cloud makes of the content, as [`Cloud::content_hash`] does.
    pub content_hash: String,
    /// How the cloud finds this very version of the file.
    pub remote: String,
}

/// What changed at one path of the cloud.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// A file that is new or has new content.
    File(CloudFile),
    /// A file, or a folder with all that was in it, that is no longer there.
    Gone(String),
}

/// The answer of [`Cloud::changes`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changes {
    pub entries: Vec<Change>,
    /// Names the state after the entries, to ask for the changes after it.
    pub cursor: String,
    /// True when the entries are all that the folder holds, and not changes: a file that is
    /// not among them is not in the cloud.
    pub whole: bool,
}

/// Why a sync stopped. What it had downloaded and uploaded before stays. The words are for the
/// log, in its `Debug` form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SyncError {
    /// The cloud has a version of the file at the path that the app has not seen yet.
    ChangedInCloud(String),
    /// The cloud could not be reached, as with no connection.
    Offline(String),
    /// Any other failure, in words for the log.
    Other(String),
}

/// A failure of the files of the app, not of the cloud.
impl From<io::Error> for SyncError {
    fn from(failure: io::Error) -> Self {
        SyncError::Other(failure.to_string())
    }
}
