//! Monotonic HMTID generators for pages, items and uploaded files.

use std::sync::Arc;

use hmtid::{Generator, Hmtid};

use crate::clock::{Clock, SystemClock};

#[derive(Debug)]
pub struct IdGenerator {
    pages: Generator,
    items: Generator,
    assets: Generator,
    /// The clock of the journal: an ID holds the time it was made at
    clock: Arc<dyn Clock>,
}

/// With the clock of the system.
impl Default for IdGenerator {
    fn default() -> Self {
        Self::new(Arc::new(SystemClock))
    }
}

impl IdGenerator {
    pub fn new(clock: Arc<dyn Clock>) -> IdGenerator {
        IdGenerator {
            pages: Generator::for_pages_and_items(),
            items: Generator::for_pages_and_items(),
            assets: Generator::for_files(),
            clock,
        }
    }

    /// The time now by the clock of the journal, for what is dated together with a new ID.
    pub fn now_millis(&self) -> i64 {
        self.clock.now_millis()
    }

    fn now(&self) -> u64 {
        self.now_millis().unsigned_abs()
    }

    pub fn generate_page_id(&self) -> Hmtid {
        self.pages.generate_at(self.now())
    }

    pub fn generate_item_id(&self) -> Hmtid {
        self.items.generate_at(self.now())
    }

    /// The ID an uploaded file is saved under. It is written without `-` between the parts
    /// of the time, to keep the file name short. See ADR-022.
    pub fn generate_asset_id(&self) -> Hmtid {
        self.assets.generate_at(self.now())
    }

    /// The ID of a file that was saved at the given time, in milliseconds since 1970. For
    /// the files that were saved before they got an ID. The times must come oldest first:
    /// an older time than the one before gets the time of the one before.
    pub fn generate_asset_id_at(&self, saved_ms: u64) -> Hmtid {
        self.assets.generate_at(saved_ms)
    }
}
