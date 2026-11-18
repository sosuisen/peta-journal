//! Monotonic HMTID generators for pages, items and uploaded files.

use hmtid::{Generator, Hmtid};

#[derive(Debug)]
pub struct IdGenerator {
    pages: Generator,
    items: Generator,
    assets: Generator,
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator {
    pub fn new() -> IdGenerator {
        IdGenerator {
            pages: Generator::for_pages_and_items(),
            items: Generator::for_pages_and_items(),
            assets: Generator::for_files(),
        }
    }

    pub fn generate_page_id(&self) -> Hmtid {
        self.pages.generate()
    }

    pub fn generate_item_id(&self) -> Hmtid {
        self.items.generate()
    }

    /// The ID an uploaded file is saved under. It is written without `-` between the parts
    /// of the time, to keep the file name short. See ADR-022.
    pub fn generate_asset_id(&self) -> Hmtid {
        self.assets.generate()
    }

    /// The ID of a file that was saved at the given time, in milliseconds since 1970. For
    /// the files that were saved before they got an ID. The times must come oldest first:
    /// an older time than the one before gets the time of the one before.
    pub fn generate_asset_id_at(&self, saved_ms: u64) -> Hmtid {
        self.assets.generate_at(saved_ms)
    }
}
