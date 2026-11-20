//! The files of the assets folder: the images and the other files that items link to.

use std::path::Path;

use anyhow::Context;
use serde::Serialize;

use crate::domain::asset_name::saved_name;
use crate::error::AppError;

use super::Journal;

/// The largest file saved among the assets. The default of 2MB of the server refused large
/// screenshots. A file is held in memory while it is saved, so the limit stays.
pub const ASSET_LIMIT_BYTES: usize = 50 * 1024 * 1024;

/// A file saved among the assets: the name it has there, and the name it came with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedAsset {
    pub file_name: String,
    pub original_name: String,
}

impl Journal {
    /// Saves a file into `{markdownDir}/assets` under a new name (ADR 022).
    pub fn save_asset(&self, original_name: &str, bytes: &[u8]) -> Result<SavedAsset, AppError> {
        let markdown_dir = self.services.config.markdown_dir();
        if markdown_dir.trim().is_empty() {
            return Err(AppError::internal("Markdown directory not configured"));
        }
        let assets_dir = Path::new(&markdown_dir).join("assets");
        std::fs::create_dir_all(&assets_dir).context("Failed to create assets directory")?;
        let file_name = saved_name(&self.services.ids.generate_asset_id(), original_name);
        let target = assets_dir.join(&file_name);
        std::fs::write(&target, bytes).context("Failed to save file")?;
        tracing::info!("Saved asset: {}", target.display());
        Ok(SavedAsset {
            file_name,
            original_name: original_name.to_string(),
        })
    }

    /// Saves a file of this device among the assets, for a caller that names the file and
    /// does not send it. The path is absolute: the folder the journal runs in means nothing
    /// to a caller.
    pub fn save_asset_file(&self, path: &str) -> Result<SavedAsset, AppError> {
        let path = Path::new(path);
        let name = path.file_name().and_then(|name| name.to_str());
        let (true, Some(original_name)) = (path.is_absolute(), name) else {
            return Err(AppError::bad_request(
                "filePath must be the absolute path of a file",
            ));
        };
        let size = std::fs::metadata(path)
            .ok()
            .filter(|file| file.is_file())
            .map(|file| file.len())
            .ok_or_else(|| AppError::not_found(format!("No file at {}", path.display())))?;
        if size > ASSET_LIMIT_BYTES as u64 {
            return Err(AppError::bad_request("The file is larger than 50MB"));
        }
        let bytes = std::fs::read(path).context("Failed to read the file")?;
        self.save_asset(original_name, &bytes)
    }
}
