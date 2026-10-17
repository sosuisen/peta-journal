//! Page use cases.

use std::sync::{Arc, LazyLock};

use hmtid::Hmtid;
use regex::Regex;

use crate::domain::fractional_index;
use crate::domain::id_generator::IdGenerator;
use crate::domain::journal_name;
use crate::domain::model::{BacklinkEntry, Item, Page};
use crate::domain::search::SearchIndex;
use crate::error::{AppError, AppResult};
use crate::infrastructure::export::export_queue_repository::ExportQueueRepository;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::db::now_millis;
use crate::infrastructure::persistence::property_repository::{ListProps, ScalarProps};
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

const PAGE_TARGET: &str = "page";

/// Result of getting or creating a document page.
#[derive(Debug, Clone)]
pub struct DocResult {
    pub page: Page,
    pub items: Vec<Item>,
    pub created: bool,
}

pub struct PageService {
    pages: PageRepository,
    items: ItemRepository,
    search: Arc<dyn SearchIndex>,
    export: Arc<MarkdownExportService>,
    export_queue: ExportQueueRepository,
    properties: PropertyRepository,
    ids: Arc<IdGenerator>,
}

static FORBIDDEN_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"[<>:"/\\|?*\x00-\x1F\x7F]"#).unwrap());
static RESERVED_NAMES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$").unwrap());

pub fn validate_file_name(title: &str) -> AppResult<()> {
    if let Some(m) = FORBIDDEN_CHARS.find(title) {
        let ch = m.as_str().chars().next().unwrap_or(' ');
        if (ch as u32) < 0x20 || ch as u32 == 0x7F {
            return Err(AppError::bad_request(
                "Title contains a control character (not allowed in filenames)",
            ));
        }
        return Err(AppError::bad_request(format!(
            "Title contains '{ch}' (not allowed in filenames)"
        )));
    }
    if title.ends_with('.') || title.ends_with(' ') {
        return Err(AppError::bad_request("Title cannot end with a dot or space"));
    }
    if RESERVED_NAMES.is_match(title) {
        return Err(AppError::bad_request(format!(
            "Title '{title}' is a reserved name on Windows"
        )));
    }
    Ok(())
}

impl PageService {
    pub fn new(
        pages: PageRepository,
        items: ItemRepository,
        search: Arc<dyn SearchIndex>,
        export: Arc<MarkdownExportService>,
        export_queue: ExportQueueRepository,
        properties: PropertyRepository,
        ids: Arc<IdGenerator>,
    ) -> PageService {
        PageService {
            pages,
            items,
            search,
            export,
            export_queue,
            properties,
            ids,
        }
    }

    pub fn get_page(&self, page_id: &Hmtid) -> AppResult<Option<Page>> {
        self.pages.find_by_id(page_id)
    }

    pub fn get_page_items(&self, page_id: &Hmtid) -> AppResult<Vec<Item>> {
        self.items.find_by_page_id(page_id)
    }

    /// Gets an existing document page by title and directory, or creates a new one.
    pub fn get_or_create_doc(&self, title: Option<&str>, dir_path: Option<&str>) -> AppResult<DocResult> {
        let title = title.unwrap_or("");
        if title.trim().is_empty() {
            return Err(AppError::bad_request("Page title cannot be empty or whitespace-only"));
        }
        if title != title.trim() {
            return Err(AppError::bad_request("Page title cannot have leading or trailing whitespace"));
        }
        let dir = dir_path.unwrap_or("");
        let file_name = title;
        validate_file_name(file_name)?;

        // Reject journal-type names as doc titles only when dirPath is empty
        if dir.is_empty() && journal_name::is_journal_file_name(file_name) {
            return Err(AppError::bad_request(format!(
                "Cannot create a doc with a journal-type name: {title}"
            )));
        }

        self.ensure_ancestor_pages(dir)?;

        if let Some(page) = self.pages.find_by_type_dir_path_file_name("doc", dir, file_name)? {
            let items = self.items.find_by_page_id(&page.id)?;
            return Ok(DocResult { page, items, created: false });
        }

        // If no exact match, check if an index page exists at dir+fileName/
        let index_dir = format!("{dir}{file_name}/");
        if let Some(page) = self.pages.find_by_type_dir_path_file_name("doc", &index_dir, "index")? {
            let items = self.items.find_by_page_id(&page.id)?;
            return Ok(DocResult { page, items, created: false });
        }

        let id = self.ids.generate_page_id();
        let now = now_millis();
        let page = Page {
            id: id.clone(),
            parent_id: None,
            dir_path: dir.to_string(),
            name: file_name.to_string(),
            page_type: "doc".to_string(),
            created_date: now,
            modified_date: now,
        };
        let page = self.pages.save(&page)?;
        self.search.index_page(&page);

        let item = self.initial_item(&page, now);
        self.items.save(&item)?;
        self.export.queue_export(&id);
        Ok(DocResult {
            page,
            items: vec![item],
            created: true,
        })
    }

    fn initial_item(&self, page: &Page, now: i64) -> Item {
        Item {
            id: self.ids.generate_item_id(),
            page_id: page.id.clone(),
            parent_item_id: None,
            dir_path: page.dir_path.clone(),
            name: page.name.clone(),
            item_type: page.page_type.clone(),
            created_date: now,
            modified_date: now,
            idx: fractional_index::initial(),
            level: 1,
            body: String::new(),
            ast: String::new(),
            collapsed: false,
            types: Vec::new(),
            transclude_item_id: None,
        }
    }

    /// Updates a page's metadata. Only the given parameters are applied.
    pub fn update_page(&self, page_id: &Hmtid, title: Option<&str>, dir_path: Option<&str>) -> AppResult<Page> {
        let existing = self
            .pages
            .find_by_id(page_id)?
            .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))?;

        let mut file_name = existing.name.clone();
        if let Some(title) = title {
            if title.trim().is_empty() {
                return Err(AppError::bad_request("Page title cannot be empty or whitespace-only"));
            }
            if title != title.trim() {
                return Err(AppError::bad_request("Page title cannot have leading or trailing whitespace"));
            }
            validate_file_name(title)?;
            file_name = title.to_string();
        }

        // Prevent demoting an index page that still has children
        if existing.name == "index" && file_name != "index" {
            let children = self.pages.find_by_parent_id(page_id)?;
            if !children.is_empty() {
                return Err(AppError::bad_request(format!(
                    "Cannot rename index page with {} child page(s). Delete or move child pages first.",
                    children.len()
                )));
            }
        }

        let mut new_dir_path = existing.dir_path.clone();
        if let Some(dir_path) = dir_path {
            new_dir_path = dir_path.to_string();
            // Ensure ancestor index pages exist for the new dir_path
            if file_name != "index" {
                self.ensure_ancestor_pages(&new_dir_path)?;
            }
        }

        // Detect dir_path change for index pages to propagate to descendants
        let old_dir_path = existing.dir_path.clone();
        let dir_path_changed = existing.name == "index" && new_dir_path != old_dir_path;

        let updated = Page {
            id: existing.id.clone(),
            parent_id: None,
            dir_path: new_dir_path.clone(),
            name: file_name,
            page_type: existing.page_type.clone(),
            created_date: existing.created_date,
            modified_date: now_millis(),
        };
        let updated = self.pages.save(&updated)?;
        self.search.index_page(&updated);

        // Re-index all items with the new page title
        let new_title = updated.title();
        for item in self.items.find_by_page_id(page_id)? {
            self.search.index_item(&item, &new_title);
        }

        if dir_path_changed {
            self.propagate_dir_path_change(page_id, &old_dir_path, &new_dir_path)?;
        }

        self.export.queue_export(page_id);
        Ok(updated)
    }

    /// The scalar and list properties of one page.
    pub fn page_properties(&self, page_id: &Hmtid) -> AppResult<(ScalarProps, ListProps)> {
        let id = page_id.to_string();
        let scalars = self.properties.find_scalar_by_targets(PAGE_TARGET, std::slice::from_ref(&id))?;
        let lists = self.properties.find_list_by_targets(PAGE_TARGET, std::slice::from_ref(&id))?;
        Ok((scalars, lists))
    }

    /// Sets a property of a page. `created` and `modified` are the page's own dates, as an
    /// RFC 3339 date-time; `tag` takes a comma separated list; every other key must be
    /// defined for pages, and an unknown key is defined as a user key of the given type.
    /// The page file is rewritten.
    pub fn set_property(&self, page_id: &Hmtid, key: &str, value_type: &str, value: &str) -> AppResult<Page> {
        let page = self
            .pages
            .find_by_id(page_id)?
            .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))?;
        if key == "created" || key == "modified" {
            let millis = chrono::DateTime::parse_from_rfc3339(value.trim())
                .map(|dt| dt.timestamp_millis())
                .map_err(|_| {
                    AppError::bad_request(format!("Not a date-time: {value} (expected e.g. 2026-01-02T03:04:05Z)"))
                })?;
            let updated = self.pages.save(&Page {
                created_date: if key == "created" { millis } else { page.created_date },
                modified_date: if key == "modified" { millis } else { now_millis() },
                ..page.clone()
            })?;
            self.export.queue_export(&page.id);
            return Ok(updated);
        }
        match self.properties.find_definition(key)? {
            Some(def) if !def.page => {
                return Err(AppError::bad_request(format!("Property '{key}' cannot be set on a page")));
            }
            Some(def) if def.value_type == "tags" || def.value_type == "list" => {
                let tags: Vec<String> = value
                    .split(',')
                    .map(|t| t.trim().to_string())
                    .filter(|t| !t.is_empty())
                    .collect();
                self.properties.sync_tags(PAGE_TARGET, page_id.as_str(), &tags)?;
            }
            _ => self
                .properties
                .upsert_scalar(PAGE_TARGET, page_id.as_str(), key, value_type, value)?,
        }
        self.touch_and_export(&page)
    }

    /// Removes a property of a page. The page file is rewritten. The page's own dates stay.
    pub fn delete_property(&self, page_id: &Hmtid, key: &str) -> AppResult<Page> {
        if key == "created" || key == "modified" {
            return Err(AppError::bad_request(format!("Property '{key}' cannot be removed from a page")));
        }
        let page = self
            .pages
            .find_by_id(page_id)?
            .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))?;
        self.properties.delete_property(PAGE_TARGET, page_id.as_str(), key)?;
        self.touch_and_export(&page)
    }

    fn touch_and_export(&self, page: &Page) -> AppResult<Page> {
        let updated = self.pages.save(&Page {
            modified_date: now_millis(),
            ..page.clone()
        })?;
        self.export.queue_export(&page.id);
        Ok(updated)
    }

    /// Deletes a page with all its items and their properties.
    pub fn delete_page(&self, page_id: &Hmtid) -> AppResult<()> {
        let page = self
            .pages
            .find_by_id(page_id)?
            .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))?;

        if page.name == "index" && page.dir_path.is_empty() {
            return Err(AppError::bad_request("Cannot delete the root index page."));
        }
        if page.name == "index" {
            let children = self.pages.find_by_parent_id(page_id)?;
            if !children.is_empty() {
                return Err(AppError::bad_request(format!(
                    "Cannot delete index page with {} child page(s). Delete or move child pages first.",
                    children.len()
                )));
            }
        }

        self.search.remove_page(page_id.as_str());
        let item_ids: Vec<String> = self
            .items
            .find_by_page_id(page_id)?
            .iter()
            .map(|i| i.id.to_string())
            .collect();
        self.properties.delete_all_for_targets("item", &item_ids)?;
        self.properties
            .delete_all_for_targets(PAGE_TARGET, std::slice::from_ref(&page_id.to_string()))?;
        self.items.delete_by_page_id(page_id)?;
        self.pages.delete(page_id)?;
        self.export_queue.remove(page_id)?;
        // Remove the markdown file too, otherwise the folder scan would bring the page back
        self.export.delete_page_file(&page);
        Ok(())
    }

    pub fn get_recent_pages(&self, max_results: i64, page_type: Option<&str>) -> AppResult<Vec<Page>> {
        self.pages.find_recent(max_results, page_type)
    }

    /// All pages and items that link to the given page (backlinks).
    pub fn get_backlinks(&self, page_id: &Hmtid) -> AppResult<Vec<BacklinkEntry>> {
        let mut entries = Vec::new();
        let Some(target) = self.pages.find_by_id(page_id)? else {
            return Ok(entries);
        };
        let linking_items = self.items.find_by_page_id_containing_link(&target.name)?;
        for source in self.pages.find_backlink_sources(page_id)? {
            for item in &linking_items {
                if item.page_id == source.id {
                    entries.push(BacklinkEntry {
                        source_page: source.clone(),
                        source_item: Some(item.clone()),
                    });
                }
            }
        }
        Ok(entries)
    }

    /// Ensures that all ancestor directory pages exist for the given dirPath.
    /// For example, dirPath "dir1/dir2/" creates pages ("dir1/", "index") and ("dir1/dir2/", "index").
    pub fn ensure_ancestor_pages(&self, dir_path: &str) -> AppResult<()> {
        if dir_path.is_empty() {
            return Ok(());
        }
        let mut current_dir = String::new();
        for segment in dir_path.split('/') {
            if segment.is_empty() {
                continue;
            }
            let parent_dir = current_dir.clone();
            current_dir.push_str(segment);
            current_dir.push('/');
            let index_dir = current_dir.clone();

            if self.pages.find_by_type_dir_path_file_name("doc", &index_dir, "index")?.is_some() {
                continue;
            }
            // Check if a named page should be promoted to index
            if let Some(page) = self.pages.find_by_type_dir_path_file_name("doc", &parent_dir, segment)? {
                let renamed = Page {
                    id: page.id.clone(),
                    parent_id: None,
                    dir_path: index_dir.clone(),
                    name: "index".to_string(),
                    page_type: page.page_type.clone(),
                    created_date: page.created_date,
                    modified_date: now_millis(),
                };
                self.pages.save(&renamed)?;
                self.search.index_page(&renamed);
            } else {
                let now = now_millis();
                let page = Page {
                    id: self.ids.generate_page_id(),
                    parent_id: None,
                    dir_path: index_dir.clone(),
                    name: "index".to_string(),
                    page_type: "doc".to_string(),
                    created_date: now,
                    modified_date: now,
                };
                self.pages.save(&page)?;
                self.search.index_page(&page);
                let item = self.initial_item(&page, now);
                self.items.save(&item)?;
            }
        }
        Ok(())
    }

    /// Recursively propagates a dir_path change to all child pages.
    fn propagate_dir_path_change(&self, parent_page_id: &Hmtid, old_prefix: &str, new_prefix: &str) -> AppResult<()> {
        for child in self.pages.find_by_parent_id(parent_page_id)? {
            let suffix = child.dir_path.get(old_prefix.len()..).unwrap_or("");
            let child_new_dir = format!("{new_prefix}{suffix}");
            let updated = Page {
                id: child.id.clone(),
                parent_id: None,
                dir_path: child_new_dir.clone(),
                name: child.name.clone(),
                page_type: child.page_type.clone(),
                created_date: child.created_date,
                modified_date: child.modified_date,
            };
            let updated = self.pages.save(&updated)?;
            self.search.index_page(&updated);
            let child_title = updated.title();
            for item in self.items.find_by_page_id(&child.id)? {
                self.search.index_item(&item, &child_title);
            }
            self.export.queue_export(&child.id);
            if child.name == "index" {
                self.propagate_dir_path_change(&child.id, &child.dir_path, &child_new_dir)?;
            }
        }
        Ok(())
    }
}
