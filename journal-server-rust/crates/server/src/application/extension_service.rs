//! Calls the extension functions when something happens to a page, and writes what they ask
//! for (see ADR 009).

use std::sync::Arc;

use crate::domain::model::Page;

use super::extension_functions::ExtensionFunctionService;
use super::extension_runtime::{ExtensionEvent, ExtensionRuntime, PageInfo};
use super::item_service::ItemService;

pub struct ExtensionService {
    runtime: ExtensionRuntime,
    functions: Arc<ExtensionFunctionService>,
    items: Arc<ItemService>,
}

impl ExtensionService {
    pub fn new(
        runtime: ExtensionRuntime,
        functions: Arc<ExtensionFunctionService>,
        items: Arc<ItemService>,
    ) -> ExtensionService {
        ExtensionService {
            runtime,
            functions,
            items,
        }
    }

    /// Lets each extension answer a page made over the API, in the order of their folder
    /// names. The items of each extension are written before the next one runs, so it sees
    /// them.
    pub fn page_created(&self, page: &Page) {
        self.dispatch(page, &ExtensionEvent::PageCreate(page_info(page)));
    }

    /// Runs every extension for `event`. A failed extension writes nothing, and the next ones
    /// still run: one broken extension must not keep a page from being made.
    fn dispatch(&self, page: &Page, event: &ExtensionEvent) {
        for function in self.functions.list_functions() {
            let dir = self.functions.functions_dir().join(&function.dir_name);
            let written = self
                .runtime
                .run(&dir, event)
                .and_then(|requests| Ok(self.items.apply_extension_items(&page.id, &requests)?));
            if let Err(e) = written {
                tracing::warn!(
                    "Extension '{}' failed on {}: {e:#}",
                    function.name,
                    page.name
                );
            }
        }
    }
}

fn page_info(page: &Page) -> PageInfo {
    PageInfo {
        id: page.id.to_string(),
        file_name: page.name.clone(),
        dir_path: page.dir_path.clone(),
        page_type: page.page_type.clone(),
    }
}
