//! Monotonic HMTID generators for pages and items.

use hmtid::{Generator, Hmtid};

#[derive(Debug)]
pub struct IdGenerator {
    pages: Generator,
    items: Generator,
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
        }
    }

    pub fn generate_page_id(&self) -> Hmtid {
        self.pages.generate()
    }

    pub fn generate_item_id(&self) -> Hmtid {
        self.items.generate()
    }
}
