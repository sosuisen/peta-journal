use crate::cloud::{Cloud, SyncError};
use crate::download::Downloader;
use crate::upload::Uploader;
use crate::Local;

/// One sync with the cloud (ADR 031): the download, then the work of the core, then the
/// upload. The download comes first, so the core has taken in what the cloud changed before
/// it writes its pages.
pub struct SyncRound<'a> {
    cloud: &'a dyn Cloud,
    local: &'a Local,
    /// Lets the core take in the files of the Markdown folder and write the pages that changed
    /// in the app. It returns when the core is done.
    take_in: &'a dyn Fn() -> Result<(), SyncError>,
}

/// How often a round runs when the cloud refuses an upload.
const TRIES: usize = 3;

impl<'a> SyncRound<'a> {
    pub fn new(
        cloud: &'a dyn Cloud,
        local: &'a Local,
        take_in: &'a dyn Fn() -> Result<(), SyncError>,
    ) -> Self {
        SyncRound {
            cloud,
            local,
            take_in,
        }
    }

    /// Runs once. When the cloud has a version of a file that the app has not seen, the round
    /// starts again: the download brings the version, and the core puts the two together.
    /// [`SyncError::ChangedInCloud`] comes out when the cloud refused in every try.
    pub fn run(
        &self,
        on_download: &(dyn Fn(u32, u32) + Sync),
        on_upload: &dyn Fn(u32, u32),
    ) -> Result<(), SyncError> {
        for _ in 1..TRIES {
            match self.once(on_download, on_upload) {
                // The next try downloads what changed
                Err(SyncError::ChangedInCloud(_)) => {}
                done => return done,
            }
        }
        self.once(on_download, on_upload)
    }

    fn once(
        &self,
        on_download: &(dyn Fn(u32, u32) + Sync),
        on_upload: &dyn Fn(u32, u32),
    ) -> Result<(), SyncError> {
        Downloader::new(self.cloud, self.local).run(on_download)?;
        (self.take_in)()?;
        Uploader::new(self.cloud, self.local).run(on_upload)
    }
}
