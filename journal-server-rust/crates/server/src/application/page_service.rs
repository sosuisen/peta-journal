//! Page use cases.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, LazyLock};

use hmtid::Hmtid;
use regex::Regex;

use super::config_service::ConfigService;
use crate::domain::fractional_index;
use crate::domain::id_generator::IdGenerator;
use crate::domain::journal_name;
use crate::domain::model::{BacklinkEntry, Item, Page, ATLAS_NAME};
use crate::domain::search::SearchIndex;
use crate::domain::wikilink;
use crate::error::{AppError, AppResult};
use crate::infrastructure::export::export_queue_repository::ExportQueueRepository;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::db::now_millis;
use crate::infrastructure::persistence::property_repository::{is_atlas_group_key, ListProps, ScalarProps};
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
    config: Arc<ConfigService>,
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

/// The paths a wikilink may name the page by. An index page is named by its folder too, as
/// opening `[[a]]` shows `a/index`; a journal by its name alone.
fn link_paths_of(page: &Page) -> Vec<String> {
    if page.page_type != "doc" {
        return vec![page.name.clone()];
    }
    if page.name == "index" {
        let dir = page.dir_path.trim_end_matches('/');
        if dir.is_empty() {
            return vec!["index".to_string()];
        }
        return vec![dir.to_string(), format!("{dir}/index")];
    }
    vec![format!("{}{}", page.dir_path, page.name)]
}

/// The folder and name of a tag's page. A tag like `a/b` is `b` in the folder `a/`, as the
/// link of a hashtag is.
fn tag_page_path(tags_dir: &str, tag: &str) -> (String, String) {
    let path = format!("{tags_dir}{tag}");
    match path.rfind('/') {
        Some(slash) => (path[..=slash].to_string(), path[slash + 1..].to_string()),
        None => (String::new(), path),
    }
}

impl PageService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        pages: PageRepository,
        items: ItemRepository,
        search: Arc<dyn SearchIndex>,
        export: Arc<MarkdownExportService>,
        export_queue: ExportQueueRepository,
        properties: PropertyRepository,
        ids: Arc<IdGenerator>,
        config: Arc<ConfigService>,
    ) -> PageService {
        PageService {
            pages,
            items,
            search,
            export,
            export_queue,
            properties,
            ids,
            config,
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

        // A page whose path differs only in case has the same file on Windows and macOS: a
        // second page there would overwrite its file again and again
        let same_file = self
            .pages
            .find_by_type_dir_path_file_name_ignoring_case("doc", dir, file_name)?
            .into_iter()
            .chain(self.pages.find_by_type_dir_path_file_name_ignoring_case("doc", &index_dir, "index")?)
            .next();
        if let Some(page) = same_file {
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

        // The atlas page belongs to its directory: it moves only with the directory, and no
        // other page may take its name
        let dir_changes = dir_path.is_some_and(|d| d != existing.dir_path);
        if existing.is_atlas() && (file_name != existing.name || dir_changes) {
            return Err(AppError::bad_request("The atlas page cannot be renamed or moved"));
        }
        if !existing.is_atlas() && file_name == ATLAS_NAME && existing.page_type == "doc" {
            return Err(AppError::bad_request(format!("'{ATLAS_NAME}' is a reserved page name")));
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
    /// RFC 3339 date-time; `tags` takes a comma separated list; every other key must be
    /// defined for pages, and an unknown key is defined as a user key of the given type.
    /// The page file is rewritten.
    /// The tags keep their tag pages, as `update_tag_pages` says.
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
        if is_atlas_group_key(key) {
            let ids: Vec<String> = value
                .split(',')
                .map(|id| id.trim().to_string())
                .filter(|id| !id.is_empty())
                .collect();
            self.properties.sync_atlas_group(page_id.as_str(), key, &ids)?;
            return self.touch_and_export(&page);
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
                let before = self.properties.find_tags(PAGE_TARGET, page_id.as_str())?;
                self.properties.sync_tags(PAGE_TARGET, page_id.as_str(), &tags)?;
                self.update_tag_pages(&before, &tags);
            }
            _ => self
                .properties
                .upsert_scalar(PAGE_TARGET, page_id.as_str(), key, value_type, value)?,
        }
        self.touch_and_export(&page)
    }

    /// Sets the tags of a page as an imported file has them, and returns the tags it had.
    /// The tag pages are left to the caller, which knows when the import is complete.
    pub fn replace_page_tags(&self, page_id: &Hmtid, tags: &[String]) -> AppResult<Vec<String>> {
        let before = self.properties.find_tags(PAGE_TARGET, page_id.as_str())?;
        if before != tags {
            self.properties.sync_tags(PAGE_TARGET, page_id.as_str(), tags)?;
        }
        Ok(before)
    }

    /// Keeps the tag pages in step with a change of tags on one page or item: the added
    /// tags get their pages, and the removed ones lose theirs when nothing else uses them.
    pub fn update_tag_pages(&self, before: &[String], after: &[String]) {
        let added: Vec<String> = after.iter().filter(|t| !before.contains(t)).cloned().collect();
        let removed: Vec<String> = before.iter().filter(|t| !after.contains(t)).cloned().collect();
        self.ensure_tag_pages(&added);
        self.delete_unused_tag_pages(&removed);
    }

    /// Creates the tag pages under the tag directory that do not exist yet, so a tag link
    /// never leads to a missing page. A tag that cannot be a page name is skipped: the tag
    /// itself is already saved and should not be refused for it.
    /// Returns how many pages were created.
    fn ensure_tag_pages(&self, tags: &[String]) -> usize {
        let tags_dir = self.config.tags_dir();
        let mut created = 0;
        for tag in tags {
            let (dir, name) = tag_page_path(&tags_dir, tag);
            match self.get_or_create_doc(Some(&name), Some(&dir)) {
                Ok(result) if result.created => created += 1,
                Ok(_) => {}
                Err(e) => tracing::warn!("Tag page '{dir}{name}' was not created: {}", e),
            }
        }
        created
    }

    /// Creates the missing tag pages of every tag in use, for tags that were added before
    /// tag pages were made. Returns how many pages were created.
    pub fn ensure_all_tag_pages(&self) -> AppResult<usize> {
        Ok(self.ensure_tag_pages(&self.properties.find_all_tag_values()?))
    }

    /// Deletes the tag pages of tags that no page or item uses any more. A page with more
    /// than one empty item was written by the user, so it stays. A failure is only logged:
    /// the change of the tags itself is already saved.
    pub fn delete_unused_tag_pages(&self, tags: &[String]) {
        let tags_dir = self.config.tags_dir();
        for tag in tags {
            let (dir, name) = tag_page_path(&tags_dir, tag);
            if let Err(e) = self.delete_tag_page_if_unused(tag, &dir, &name) {
                tracing::warn!("Tag page '{dir}{name}' was not deleted: {}", e);
            }
        }
    }

    /// Tags that differ only in case share their tag page, as they share its file.
    fn delete_tag_page_if_unused(&self, tag: &str, dir: &str, name: &str) -> AppResult<()> {
        if self.properties.count_tag_uses_ignoring_case(tag)? > 0 {
            return Ok(());
        }
        for page in self.pages.find_by_type_dir_path_file_name_ignoring_case("doc", dir, name)? {
            if self.is_empty_page(&page)? {
                self.delete_page(&page.id)?;
            }
        }
        Ok(())
    }

    /// A page with one empty item, as a new page has.
    fn is_empty_page(&self, page: &Page) -> AppResult<bool> {
        let items = self.items.find_by_page_id(&page.id)?;
        Ok(matches!(items.as_slice(), [only] if only.body.trim().is_empty() && only.transclude_item_id.is_none()))
    }

    /// Deletes the tag pages that only repeat an older one in another case, like
    /// `NextJS` beside `Nextjs`. Such pages were made before tag pages were matched ignoring
    /// case, and they overwrite the file of the older page. Only empty ones go; the older page
    /// is written again at once, since its file was the one deleted. Returns how many went.
    pub fn delete_case_duplicate_tag_pages(&self) -> AppResult<usize> {
        let tags_dir = self.config.tags_dir();
        let mut deleted = 0;
        let mut done: Vec<String> = Vec::new();
        for tag in self.properties.find_all_tag_values()? {
            let (dir, name) = tag_page_path(&tags_dir, &tag);
            let key = format!("{dir}{name}").to_lowercase();
            if done.contains(&key) {
                continue;
            }
            done.push(key);
            let pages = self.pages.find_by_type_dir_path_file_name_ignoring_case("doc", &dir, &name)?;
            let Some((kept, others)) = pages.split_first() else { continue };
            let mut any = false;
            for page in others {
                if self.is_empty_page(page)? {
                    self.delete_page(&page.id)?;
                    deleted += 1;
                    any = true;
                } else {
                    tracing::warn!("{}{} repeats {}{} in another case but is not empty", page.dir_path, page.name, kept.dir_path, kept.name);
                }
            }
            if any {
                if let Err(e) = self.export.export_page(&kept.id) {
                    tracing::warn!("{}{} was not written again: {e}", kept.dir_path, kept.name);
                    self.export.queue_export(&kept.id);
                }
            }
        }
        Ok(deleted)
    }

    /// Removes a property of a page. The page file is rewritten. The page's own dates stay.
    /// Removing the tags deletes the tag pages that became unused, as `set_property` does.
    pub fn delete_property(&self, page_id: &Hmtid, key: &str) -> AppResult<Page> {
        if key == "created" || key == "modified" {
            return Err(AppError::bad_request(format!("Property '{key}' cannot be removed from a page")));
        }
        let page = self
            .pages
            .find_by_id(page_id)?
            .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))?;
        let before = if key == "tags" {
            self.properties.find_tags(PAGE_TARGET, page_id.as_str())?
        } else {
            Vec::new()
        };
        self.properties.delete_property(PAGE_TARGET, page_id.as_str(), key)?;
        if is_atlas_group_key(key) {
            self.properties.delete_unused_atlas_group(key)?;
        }
        self.delete_unused_tag_pages(&before);
        self.touch_and_export(&page)
    }

    /// Removes a user property key everywhere. Every page that lost a value, on itself or
    /// on one of its items, gets a new modified date and its file rewritten; their ids come
    /// back so the caller can tell the clients.
    pub fn delete_property_definition(&self, key: &str) -> AppResult<Vec<Hmtid>> {
        let mut touched = Vec::new();
        for id in self.properties.delete_definition(key)? {
            let Ok(page_id) = Hmtid::parse(&id) else { continue };
            if let Some(page) = self.pages.find_by_id(&page_id)? {
                self.touch_and_export(&page)?;
                touched.push(page_id);
            }
        }
        Ok(touched)
    }

    fn touch_and_export(&self, page: &Page) -> AppResult<Page> {
        let updated = self.pages.save(&Page {
            modified_date: now_millis(),
            ..page.clone()
        })?;
        self.export.queue_export(&page.id);
        Ok(updated)
    }

    /// Deletes a page with all its items and their properties. The tags of the page and of
    /// its items go with them, so their tag pages are deleted when nothing else uses them.
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
        let mut tags = self.properties.find_tags(PAGE_TARGET, page_id.as_str())?;
        for id in &item_ids {
            tags.extend(self.properties.find_tags("item", id)?);
        }
        tags.sort();
        tags.dedup();
        self.properties.delete_all_for_targets("item", &item_ids)?;
        self.properties
            .delete_all_for_targets(PAGE_TARGET, std::slice::from_ref(&page_id.to_string()))?;
        self.items.delete_by_page_id(page_id)?;
        self.pages.delete(page_id)?;
        self.export_queue.remove(page_id)?;
        // Remove the markdown file too, otherwise the folder scan would bring the page back
        self.export.delete_page_file(&page);
        self.delete_unused_tag_pages(&tags);
        Ok(())
    }

    pub fn get_recent_pages(&self, max_results: i64, page_type: Option<&str>) -> AppResult<Vec<Page>> {
        self.pages.find_recent(max_results, page_type)
    }

    /// The pages and items that link to the given page (backlinks). A page links by having
    /// the tag of a tag page in its tags (an entry without an item); an item links by a
    /// hashtag of that tag or by a wikilink to the page. Paths and tags are matched ignoring
    /// case, as pages are found. The page's own items are left out. Pages come newest first,
    /// each with its tag entry before its items, and the items in page order.
    pub fn get_backlinks(&self, page_id: &Hmtid) -> AppResult<Vec<BacklinkEntry>> {
        let Some(target) = self.pages.find_by_id(page_id)? else {
            return Ok(Vec::new());
        };
        let mut tagging_pages: BTreeSet<String> = BTreeSet::new();
        let mut linking_items: BTreeSet<String> = BTreeSet::new();
        if let Some(tag) = self.tag_of_page(&target) {
            tagging_pages.extend(self.properties.find_page_ids_by_tag_value_ignoring_case(&tag)?);
            linking_items.extend(self.properties.find_item_ids_by_tag_value_ignoring_case(&tag, i64::MAX)?);
        }
        let paths = link_paths_of(&target);
        for path in &paths {
            for item in self.items.find_by_body_containing_link_to(path)? {
                let links = wikilink::extract_wikilink_paths(&item.ast);
                if links.iter().any(|l| paths.iter().any(|p| l.eq_ignore_ascii_case(p))) {
                    linking_items.insert(item.id.to_string());
                }
            }
        }

        // Source page id -> (does its tags property link, its linking items)
        let mut by_page: BTreeMap<String, (bool, Vec<Item>)> = BTreeMap::new();
        for id in tagging_pages {
            by_page.entry(id).or_default().0 = true;
        }
        for id in linking_items {
            let Ok(item_id) = Hmtid::parse(&id) else { continue };
            if let Some(item) = self.items.find_by_id(&item_id)? {
                by_page.entry(item.page_id.to_string()).or_default().1.push(item);
            }
        }
        by_page.remove(page_id.as_str());

        let mut sources: Vec<(Page, bool, Vec<Item>)> = Vec::new();
        for (id, (tagged, items)) in by_page {
            let Ok(source_id) = Hmtid::parse(&id) else { continue };
            if let Some(page) = self.pages.find_by_id(&source_id)? {
                sources.push((page, tagged, items));
            }
        }
        sources.sort_by_key(|s| std::cmp::Reverse(s.0.modified_date));

        let mut entries = Vec::new();
        for (page, tagged, mut items) in sources {
            if tagged {
                entries.push(BacklinkEntry {
                    source_page: page.clone(),
                    source_item: None,
                });
            }
            items.sort_by(|a, b| a.idx.cmp(&b.idx));
            for item in items {
                entries.push(BacklinkEntry {
                    source_page: page.clone(),
                    source_item: Some(item),
                });
            }
        }
        Ok(entries)
    }

    /// The tag whose tag page this is, when the page lies under the tag directory. A tag with
    /// pages under it is an index page: `tags/a/index` is the page of the tag `a`.
    fn tag_of_page(&self, page: &Page) -> Option<String> {
        if page.page_type != "doc" {
            return None;
        }
        let path = if page.name == "index" {
            page.dir_path.trim_end_matches('/').to_string()
        } else {
            format!("{}{}", page.dir_path, page.name)
        };
        let tags_dir = self.config.tags_dir();
        let head = path.get(..tags_dir.len())?;
        let tag = path.get(tags_dir.len()..)?;
        (head.eq_ignore_ascii_case(&tags_dir) && !tag.is_empty()).then(|| tag.to_string())
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
