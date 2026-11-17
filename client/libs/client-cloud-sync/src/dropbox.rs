use std::fs::File;
use std::io::{self, Read};
use std::path::Path;
use std::time::UNIX_EPOCH;

use sha2::{Digest, Sha256};

use crate::cloud::{Change, Changes, Cloud, CloudFile, SyncError};

/// The calls of Dropbox that [`DropboxCloud`] makes, without the types of the Dropbox SDK. The
/// shell puts the SDK of its OS here, and a test its own Dropbox (ADR 054).
///
/// It is called from the threads of the sync, several at a time.
pub trait DropboxFiles: Send + Sync {
    /// The first part of all that is in the folder at the path, and in the folders in it.
    fn list(&self, folder: &str) -> Result<Listing, DropboxError>;

    /// The part after `cursor`: the next part of a list, or what changed since a list ended.
    /// The error is [`DropboxError::CursorForgotten`] when Dropbox no longer knows the cursor.
    fn list_after(&self, cursor: &str) -> Result<Listing, DropboxError>;

    /// Writes the content of the version `rev` of a file to the path `to`, which is a file here.
    fn download(&self, rev: &str, to: &Path) -> Result<(), DropboxError>;

    /// Puts the content of the file `from` at the path. `over` is the version that this content
    /// replaces, and it is `None` for a file that Dropbox has not. Dropbox gives the file no
    /// other name. `modified_ms` is the time of the last change, as the file has it here, in
    /// milliseconds since 1970.
    ///
    /// The error is [`DropboxError::UploadConflict`] when Dropbox has another version, or a
    /// file where none was expected.
    fn upload(
        &self,
        path: &str,
        from: &Path,
        over: Option<&str>,
        modified_ms: i64,
    ) -> Result<DropboxFile, DropboxError>;
}

/// One part of a list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Listing {
    pub entries: Vec<DropboxEntry>,
    /// Names the state after this part.
    pub cursor: String,
    /// Says that a part follows.
    pub has_more: bool,
}

/// What Dropbox tells of one path. The path is as Dropbox shows it, with the case of the
/// names. The SDK may give none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropboxEntry {
    File(DropboxFile),
    Deleted { path: Option<String> },
    Folder { path: Option<String> },
}

/// A file as Dropbox tells of it. `content_hash` is `None` when Dropbox gave none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DropboxFile {
    pub path: Option<String>,
    pub content_hash: Option<String>,
    pub rev: String,
}

/// Why Dropbox did not do what it was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DropboxError {
    /// Dropbox forgot a cursor that was too old. It wants the list made anew.
    CursorForgotten,
    /// Dropbox did not take an upload: the file there is not the version that was to be
    /// replaced.
    UploadConflict,
    /// Dropbox could not be reached, as with no connection. The shell tells it from the
    /// errors of its SDK.
    Offline(String),
    /// Any other failure, in the words of the SDK, for the log.
    Other(String),
}

impl From<DropboxError> for SyncError {
    fn from(failure: DropboxError) -> Self {
        match failure {
            DropboxError::Offline(why) => SyncError::Offline(why),
            DropboxError::Other(why) => SyncError::Other(why),
            // Not where Dropbox can answer it: the cloud turns each into what it means
            unexpected => SyncError::Other(format!("{unexpected:?}")),
        }
    }
}

/// The Markdown folder in Dropbox (ADR 030, ADR 031).
pub struct DropboxCloud {
    files: Box<dyn DropboxFiles>,
    folder: String,
}

/// Before the version of a file in [`CloudFile::remote`].
const REV: &str = "rev:";

impl DropboxCloud {
    /// `folder` is the path of the folder in Dropbox, such as `/PetaJournal`.
    pub fn new(files: impl DropboxFiles + 'static, folder: &str) -> Self {
        DropboxCloud {
            files: Box::new(files),
            folder: folder.trim_end_matches('/').to_string(),
        }
    }

    /// Dropbox answers in parts.
    fn read(&self, first: Listing, whole: bool) -> Result<Changes, SyncError> {
        let mut part = first;
        let mut entries = Vec::new();
        loop {
            entries.extend(part.entries.iter().filter_map(|entry| self.change(entry)));
            if !part.has_more {
                return Ok(Changes {
                    entries,
                    cursor: part.cursor,
                    whole,
                });
            }
            part = self.files.list_after(&part.cursor)?;
        }
    }

    /// Nothing for a folder that is there: its files tell of it.
    fn change(&self, entry: &DropboxEntry) -> Option<Change> {
        match entry {
            DropboxEntry::File(file) => Some(Change::File(self.version(file)?)),
            DropboxEntry::Deleted { path } => Some(Change::Gone(self.inside(path.as_deref())?)),
            DropboxEntry::Folder { .. } => None,
        }
    }

    fn version(&self, file: &DropboxFile) -> Option<CloudFile> {
        Some(CloudFile {
            path: self.inside(file.path.as_deref())?,
            content_hash: file.content_hash.clone().unwrap_or_default(),
            remote: format!("{REV}{}", file.rev),
        })
    }

    /// The path in the folder. The path as it is shown keeps the case of the name, which names
    /// the page; the folder may be spelled otherwise there.
    fn inside(&self, path: Option<&str>) -> Option<String> {
        let path: String = path?.chars().skip(self.folder.chars().count()).collect();
        let path = path.trim_start_matches('/');
        (!path.is_empty()).then(|| path.to_string())
    }
}

impl Cloud for DropboxCloud {
    fn changes(&self, cursor: Option<&str>) -> Result<Changes, SyncError> {
        if let Some(cursor) = cursor {
            match self.files.list_after(cursor) {
                Ok(listing) => return self.read(listing, false),
                // Dropbox wants the list made anew
                Err(DropboxError::CursorForgotten) => {}
                Err(failure) => return Err(failure.into()),
            }
        }
        self.read(self.files.list(&self.folder)?, true)
    }

    fn download(&self, file: &CloudFile, to: &Path) -> Result<(), SyncError> {
        let rev = file.remote.strip_prefix(REV).unwrap_or(&file.remote);
        Ok(self.files.download(rev, to)?)
    }

    fn upload(&self, path: &str, from: &Path, over: Option<&str>) -> Result<CloudFile, SyncError> {
        let over = over.map(|over| over.strip_prefix(REV).unwrap_or(over));
        let uploaded = self.files.upload(
            &format!("{}/{path}", self.folder),
            from,
            over,
            modified_ms(from)?,
        );
        match uploaded {
            Ok(file) => self
                .version(&file)
                .ok_or_else(|| SyncError::Other(format!("Dropbox gave no path for {path}"))),
            Err(DropboxError::UploadConflict) => Err(SyncError::ChangedInCloud(path.to_string())),
            Err(failure) => Err(failure.into()),
        }
    }

    fn content_hash(&self, file: &Path) -> io::Result<String> {
        content_hash(file)
    }
}

/// The time of the last change of the file, in whole seconds. The Dropbox app of the desk
/// gives its copy this time; Dropbox keeps no parts of a second.
fn modified_ms(file: &Path) -> io::Result<i64> {
    let since = file
        .metadata()?
        .modified()?
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?;
    Ok(i64::try_from(since.as_secs()).map_or(i64::MAX, |seconds| seconds.saturating_mul(1000)))
}

/// The size of the blocks that Dropbox hashes one by one.
const BLOCK: usize = 4 * 1024 * 1024;

/// The content hash of Dropbox: the SHA-256 of the SHA-256 of each block of 4 MiB, in hex.
/// Dropbox gives it for each file, so it tells whether a file here has the content of the file
/// in Dropbox, with no download.
pub fn content_hash(file: &Path) -> io::Result<String> {
    let mut reader = File::open(file)?;
    let mut blocks = Sha256::new();
    let mut block = Vec::with_capacity(BLOCK);
    loop {
        block.clear();
        (&mut reader).take(BLOCK as u64).read_to_end(&mut block)?;
        if block.is_empty() {
            break;
        }
        blocks.update(Sha256::digest(&block));
    }
    Ok(blocks
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
