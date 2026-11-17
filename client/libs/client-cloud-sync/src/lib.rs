//! The sync of the Markdown folder of the app with a folder in the cloud, for the Android and
//! the iOS shell (ADR 030, ADR 031, ADR 054).
//!
//! The rules are written here once. A shell gives only the calls of the Dropbox SDK
//! ([`DropboxFiles`]), the sign-in, and the screen.

mod cloud;
mod download;
mod dropbox;
mod folder;
mod record;
mod round;
mod sync;
mod upload;

pub use cloud::{Change, Changes, Cloud, CloudFile, SyncError};
pub use download::Downloader;
pub use dropbox::{
    content_hash, DropboxCloud, DropboxEntry, DropboxError, DropboxFile, DropboxFiles, Listing,
};
pub use folder::{find_without_case, MarkdownFolder};
pub use record::CloudRecord;
pub use round::SyncRound;
pub use sync::{CloudSync, SyncListener, SyncState};
pub use upload::Uploader;

use std::path::PathBuf;

/// What the sync keeps in the app: the Markdown folder, where the last download ended, and the
/// versions of the files in the cloud that the app has seen.
pub struct Local {
    pub folder: MarkdownFolder,
    /// The file that keeps the cursor of the last download, for the next one to ask only for
    /// what changed since.
    pub cursor: PathBuf,
    pub record: CloudRecord,
}

/// The path as the sync compares it: in lower case, as the cloud compares paths, and with the
/// letters composed (NFC). The file system of the iPhone and of the Mac takes a name with the
/// sound marks apart (NFD) for the same name; an app of the Mac may have written a name so,
/// and Dropbox may give it either way.
pub(crate) fn fold(path: &str) -> String {
    use unicode_normalization::UnicodeNormalization;
    path.nfc().collect::<String>().to_lowercase()
}
