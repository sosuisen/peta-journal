use crate::cloud::{Cloud, SyncError};
use crate::download::count;
use crate::Local;

/// Brings the pages that changed in the app to the folder in the cloud (ADR 031). It uploads
/// the Markdown files, and the definitions of the properties, whose content is not what the
/// cloud had when the app last heard of it. It deletes and moves nothing in the cloud.
pub struct Uploader<'a> {
    cloud: &'a dyn Cloud,
    local: &'a Local,
}

impl<'a> Uploader<'a> {
    pub fn new(cloud: &'a dyn Cloud, local: &'a Local) -> Self {
        Uploader { cloud, local }
    }

    /// Runs once, after a download and after the core wrote its pages. `progress` gets how
    /// many files are uploaded, of how many: first with 0, then after each file.
    ///
    /// [`SyncError::ChangedInCloud`] comes out when the cloud has a version of a file that the
    /// app has not seen: the file there is not written over, and the sync is to start again
    /// with a download. What was uploaded before that, or before another failure, stays
    /// uploaded.
    pub fn run(&self, progress: &dyn Fn(u32, u32)) -> Result<(), SyncError> {
        let folder = &self.local.folder;
        let record = &self.local.record;
        let changed: Vec<String> = folder
            .written()
            .into_iter()
            .filter(|path| {
                folder.content_hash(path, |file| self.cloud.content_hash(file))
                    != record.get(path).map(|known| known.content_hash)
            })
            .collect();
        let total = count(changed.len());
        progress(0, total);
        let uploaded =
            changed
                .iter()
                .enumerate()
                .try_for_each(|(done, path)| -> Result<(), SyncError> {
                    let over = record.get(path).map(|known| known.remote);
                    record.put(
                        self.cloud
                            .upload(path, &folder.file(path), over.as_deref())?,
                    );
                    progress(count(done + 1), total);
                    Ok(())
                });
        let saved = record.save();
        uploaded?;
        saved?;
        Ok(())
    }
}
