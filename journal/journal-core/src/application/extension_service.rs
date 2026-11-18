//! Calls the extension functions when something happens to a page, and writes what they ask
//! for (see ADR 009).

use std::sync::Arc;

use hmtid::Hmtid;

use crate::domain::model::{Item, Page};
use crate::error::AppError;

use super::extension_functions::ExtensionFunctionService;
use super::extension_runtime::{ExtensionEvent, ExtensionRuntime, PageInfo};
use super::item_service::ItemService;
use super::page_service::PageService;

/// Tells the clients that extensions changed a page after its request was answered.
pub trait ExtensionEvents: Send + Sync {
    fn page_changed(&self, page_id: &Hmtid);
}

pub struct ExtensionService {
    runtime: ExtensionRuntime,
    functions: Arc<ExtensionFunctionService>,
    items: Arc<ItemService>,
    pages: Arc<PageService>,
    events: Arc<dyn ExtensionEvents>,
}

impl ExtensionService {
    pub fn new(
        runtime: ExtensionRuntime,
        functions: Arc<ExtensionFunctionService>,
        items: Arc<ItemService>,
        pages: Arc<PageService>,
        events: Arc<dyn ExtensionEvents>,
    ) -> ExtensionService {
        ExtensionService {
            runtime,
            functions,
            items,
            pages,
            events,
        }
    }

    /// Lets each extension answer a page made over the API, in the order of their folder
    /// names. The items of each extension are written before the next one runs. Returns the
    /// items the page has then, which the caller answers with.
    pub fn page_created(&self, page: &Page) -> Result<Vec<Item>, AppError> {
        self.dispatch(page, &ExtensionEvent::PageCreate(page_info(page)));
        self.pages.get_page_items(&page.id)
    }

    /// Lets each extension answer the tags in `after` that are not in `before`. The extensions
    /// run on a thread of their own after the caller has answered, since one may wait for the
    /// user. The clients hear of the page when they are done.
    pub fn tags_changed(self: &Arc<Self>, page: &Page, before: &[String], after: &[String]) {
        let added: Vec<String> = after
            .iter()
            .filter(|tag| !before.contains(tag))
            .cloned()
            .collect();
        if added.is_empty() {
            return;
        }
        let service = Arc::clone(self);
        let page = page.clone();
        std::thread::spawn(move || {
            service.dispatch(&page, &ExtensionEvent::PageTagAdd(page_info(&page), added));
            service.events.page_changed(&page.id);
        });
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
