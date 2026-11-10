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
            pages: Generator::new('-', true),
            items: Generator::new('-', true),
            assets: Generator::new('_', false),
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
}
