//! Page use cases.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use hmtid::Hmtid;
use rusqlite::{Connection, Transaction};

use super::atlas_service::AtlasService;
use super::config_service::ConfigService;
use super::markdown_parser::MarkdownParser;
use crate::domain::id_generator::IdGenerator;
use crate::domain::journal_name;
use crate::domain::model::{BacklinkEntry, Item, Page, ATLAS_NAME};
use crate::domain::nfc::to_nfc;
use crate::domain::property_key::PropertyKey;
use crate::domain::search::SearchIndex;
use crate::domain::wikilink;
use crate::error::AppError;
use crate::infrastructure::export::export_service::MarkdownExportService;
use crate::infrastructure::persistence::property_repository::{
    list_values, ListProps, ScalarProps,
};
use crate::infrastructure::persistence::{ItemRepository, PageRepository, PropertyRepository};

const PAGE_TARGET: &str = "page";
/// The keys of the front matter that are not scalar properties: the page's own id and dates,
/// and the tags with the name they had before.
const FRONT_MATTER_KEYS: [&str; 5] = ["id", "created", "modified", "tags", "tag"];

/// Result of getting or creating a document page.
#[derive(Debug, Clone)]
pub struct DocResult {
    pub page: Page,
    pub items: Vec<Item>,
    pub created: bool,
}

/// What a page operation does outside the database, once its transaction is committed: the
/// search index, the files, the tag pages and the atlas pages.
#[derive(Default)]
struct AfterCommit {
    /// Pages to index anew, each with the items to index under its title.
    indexed: Vec<(Page, Vec<Item>)>,
    removed_page_ids: Vec<String>,
    exported: BTreeSet<Hmtid>,
    /// Pages whose markdown file goes too.
    deleted: Vec<Page>,
    /// The tags before and after, for the tag pages.
    tag_changes: Vec<(Vec<String>, Vec<String>)>,
    /// The pages of a directory changed, so the atlas pages are written again.
    atlases_stale: bool,
}

pub struct PageService {
    pages: PageRepository,
    items: ItemRepository,
    search: Arc<dyn SearchIndex>,
    export: Arc<MarkdownExportService>,
    properties: PropertyRepository,
    ids: Arc<IdGenerator>,
    config: Arc<ConfigService>,
    /// Keeps the atlas pages in step with the pages of their directories.
    atlas: Arc<AtlasService>,
}

/// A title a page may take: not blank, not padded, and a file name on every OS.
pub fn validate_title(title: &str) -> Result<(), AppError> {
    if title.trim().is_empty() {
        return Err(AppError::bad_request(
            "Page title cannot be empty or whitespace-only",
        ));
    }
    if title != title.trim() {
        return Err(AppError::bad_request(
            "Page title cannot have leading or trailing whitespace",
        ));
    }
    validate_file_name(title)
}

/// A file name on every OS. The clients check a name with the same rule (`page-names`)
/// before they ask.
pub fn validate_file_name(title: &str) -> Result<(), AppError> {
    page_names::check_file_name(title).map_err(|e| AppError::bad_request(e.to_string()))
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
        properties: PropertyRepository,
        ids: Arc<IdGenerator>,
        config: Arc<ConfigService>,
        atlas: Arc<AtlasService>,
    ) -> PageService {
        PageService {
            pages,
            items,
            search,
            export,
            properties,
            ids,
            config,
            atlas,
        }
    }

    pub fn get_page(&self, page_id: &Hmtid) -> Result<Option<Page>, AppError> {
        self.pages.find_by_id(page_id)
    }

    /// The page a change is made to; a missing one is bad input, as the id came from the caller.
    pub fn require_page(&self, page_id: &Hmtid) -> Result<Page, AppError> {
        self.pages
            .find_by_id(page_id)?
            .ok_or_else(|| AppError::bad_request(format!("Page not found: {page_id}")))
    }

    pub fn get_page_items(&self, page_id: &Hmtid) -> Result<Vec<Item>, AppError> {
        self.items.find_by_page_id(page_id)
    }

    /// The document a title names in a folder, when there is one. Nothing is made: an app
    /// that only reads asks this before it opens a page (ADR 030).
    pub fn find_doc(&self, title: &str, dir_path: &str) -> Result<Option<Page>, AppError> {
        let conn = self.items.db().conn()?;
        self.find_doc_in(&conn, &to_nfc(dir_path), &to_nfc(title))
    }

    /// Gets an existing document page by title and directory, or creates a new one.
    pub fn get_or_create_doc(
        &self,
        title: Option<&str>,
        dir_path: Option<&str>,
    ) -> Result<DocResult, AppError> {
        let title = to_nfc(title.unwrap_or(""));
        let title = title.as_str();
        validate_title(title)?;
        let dir = to_nfc(dir_path.unwrap_or(""));
        let dir = dir.as_str();
        let file_name = title;

        // Reject journal-type names as doc titles only when dirPath is empty
        if dir.is_empty() && journal_name::is_journal_file_name(file_name) {
            return Err(AppError::bad_request(format!(
                "Cannot create a doc with a journal-type name: {title}"
            )));
        }

        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        self.make_ancestor_pages_in(&tx, dir, true, &mut effects)?;
        let mut result = match self.find_doc_in(&tx, dir, file_name)? {
            Some(page) => DocResult {
                items: ItemRepository::find_by_page_id_in(&tx, &page.id)?,
                page,
                created: false,
            },
            None => self.create_doc_in(&tx, dir, file_name, &mut effects)?,
        };
        tx.commit()?;
        self.finish(effects);
        if result.created {
            // A new atlas page is written at once, by the sync after the commit
            result.items = self.items.find_by_page_id(&result.page.id)?;
        }
        Ok(result)
    }

    /// Creates a document page with one empty item, in the transaction.
    fn create_doc_in(
        &self,
        tx: &Connection,
        dir: &str,
        file_name: &str,
        effects: &mut AfterCommit,
    ) -> Result<DocResult, AppError> {
        let id = self.ids.generate_page_id();
        let now = self.items.db().now_millis();
        let page = PageRepository::save_in(tx, &Page::new(id.clone(), dir, file_name, "doc", now))?;
        // The items of an atlas page are its cards, which the server writes; an empty first
        // item could take the index a card took already
        if !page.is_atlas() {
            ItemRepository::save_in(tx, &Item::new_on(self.ids.generate_item_id(), &page, now))?;
        }
        effects.indexed.push((page.clone(), Vec::new()));
        effects.exported.insert(id.clone());
        effects.atlases_stale = true;
        Ok(DocResult {
            items: ItemRepository::find_by_page_id_in(tx, &id)?,
            page,
            created: true,
        })
    }

    /// The document a title names in a folder: the page with that name, else the index page of
    /// the folder of that name (opening `[[a]]` shows `a/index`), else a page whose path
    /// differs only in case. Such a page has the same file on Windows and macOS: a second
    /// page there would overwrite its file again and again.
    fn find_doc_in(
        &self,
        tx: &Connection,
        dir: &str,
        file_name: &str,
    ) -> Result<Option<Page>, AppError> {
        if let Some(page) =
            PageRepository::find_by_type_dir_path_file_name_in(tx, "doc", dir, file_name)?
        {
            return Ok(Some(page));
        }
        let index_dir = format!("{dir}{file_name}/");
        if let Some(page) =
            PageRepository::find_by_type_dir_path_file_name_in(tx, "doc", &index_dir, "index")?
        {
            return Ok(Some(page));
        }
        Ok(
            PageRepository::find_by_type_dir_path_file_name_ignoring_case_in(
                tx, "doc", dir, file_name,
            )?
            .into_iter()
            .chain(
                PageRepository::find_by_type_dir_path_file_name_ignoring_case_in(
                    tx, "doc", &index_dir, "index",
                )?,
            )
            .next(),
        )
    }

    /// Updates a page's metadata. Only the given parameters are applied.
    /// When another page has the new name, the update is refused as a conflict unless
    /// `overwrite` is set; then that page is deleted and this one takes its place.
    pub fn update_page(
        &self,
        page_id: &Hmtid,
        title: Option<&str>,
        dir_path: Option<&str>,
        overwrite: bool,
    ) -> Result<Page, AppError> {
        self.rename_page(page_id, title, dir_path, overwrite, false)
    }

    /// Updates a page's metadata as [`Self::update_page`] does. With `drop_atlas`, the atlas
    /// page of an index page is deleted first, in the same transaction: a demotion leaves
    /// no directory for it to belong to.
    pub(crate) fn rename_page(
        &self,
        page_id: &Hmtid,
        title: Option<&str>,
        dir_path: Option<&str>,
        overwrite: bool,
        drop_atlas: bool,
    ) -> Result<Page, AppError> {
        let existing = self.require_page(page_id)?;

        let mut file_name = existing.name.clone();
        if let Some(title) = title {
            validate_title(title)?;
            file_name = title.to_string();
        }

        // The atlas page belongs to its directory: it moves only with the directory, and no
        // other page may take its name
        let dir_changes = dir_path.is_some_and(|d| d != existing.dir_path);
        if existing.is_atlas() && (file_name != existing.name || dir_changes) {
            return Err(AppError::bad_request(
                "The atlas page cannot be renamed or moved",
            ));
        }
        if !existing.is_atlas() && file_name == ATLAS_NAME && existing.page_type == "doc" {
            return Err(AppError::bad_request(format!(
                "'{ATLAS_NAME}' is a reserved page name"
            )));
        }

        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        if drop_atlas {
            for child in PageRepository::find_by_parent_id_in(&tx, page_id)? {
                if child.is_atlas() {
                    self.delete_page_in(&tx, &child, &mut effects)?;
                }
            }
        }

        // Prevent demoting an index page that still has children
        if existing.name == "index" && file_name != "index" {
            let children = PageRepository::find_by_parent_id_in(&tx, page_id)?;
            if !children.is_empty() {
                return Err(AppError::bad_request(format!(
                    "Cannot rename index page with {} child page(s). Delete or move child pages first.",
                    children.len()
                )));
            }
        }

        let new_dir_path = dir_path.map_or_else(|| existing.dir_path.clone(), str::to_string);

        // Names are matched ignoring case, as the files of both would be one file on Windows
        let taken: Vec<Page> = PageRepository::find_by_type_dir_path_file_name_ignoring_case_in(
            &tx,
            &existing.page_type,
            &new_dir_path,
            &file_name,
        )?
        .into_iter()
        .filter(|p| p.id != existing.id)
        .collect();
        if !taken.is_empty() && !overwrite {
            return Err(AppError::conflict(format!(
                "A page named '{new_dir_path}{file_name}' already exists"
            )));
        }
        for page in &taken {
            self.delete_page_in(&tx, page, &mut effects)?;
        }

        // Ensure ancestor index pages exist for the new dir_path
        if dir_path.is_some() && file_name != "index" {
            self.make_ancestor_pages_in(&tx, &new_dir_path, true, &mut effects)?;
        }

        // Detect dir_path change for index pages to propagate to descendants
        let old_dir_path = existing.dir_path.clone();
        let dir_path_changed = existing.name == "index" && new_dir_path != old_dir_path;

        let updated = Page {
            dir_path: new_dir_path.clone(),
            name: file_name,
            modified_date: self.items.db().now_millis(),
            ..existing.clone()
        };
        let updated = PageRepository::save_in(&tx, &updated)?;
        // The items are indexed again under the new page title
        effects.indexed.push((
            updated.clone(),
            ItemRepository::find_by_page_id_in(&tx, page_id)?,
        ));

        if dir_path_changed {
            self.propagate_dir_path_change_in(
                &tx,
                page_id,
                &old_dir_path,
                &new_dir_path,
                &mut effects,
            )?;
        }

        effects.exported.insert(page_id.clone());
        effects.atlases_stale = true;
        tx.commit()?;
        self.finish(effects);
        Ok(updated)
    }

    /// The scalar and list properties of one page.
    pub fn page_properties(&self, page_id: &Hmtid) -> Result<(ScalarProps, ListProps), AppError> {
        let id = page_id.to_string();
        let scalars = self
            .properties
            .find_scalar_by_targets(PAGE_TARGET, std::slice::from_ref(&id))?;
        let lists = self
            .properties
            .find_list_by_targets(PAGE_TARGET, std::slice::from_ref(&id))?;
        Ok((scalars, lists))
    }

    /// Sets a property of a page. `created` and `modified` are the page's own dates, as an
    /// RFC 3339 date-time; `tags` takes a comma separated list; every other key must be
    /// defined for pages, and an unknown key is defined as a user key of the given type.
    /// The page file is rewritten.
    /// The tags keep their tag pages, as `update_tag_pages` says.
    pub fn set_property(
        &self,
        page_id: &Hmtid,
        key: &str,
        _value_type: &str,
        value: &str,
    ) -> Result<Page, AppError> {
        let page = self.require_page(page_id)?;
        if key == "created" || key == "modified" {
            let millis = chrono::DateTime::parse_from_rfc3339(value.trim())
                .map(|dt| dt.timestamp_millis())
                .map_err(|_| {
                    AppError::bad_request(format!(
                        "Not a date-time: {value} (expected e.g. 2026-01-02T03:04:05Z)"
                    ))
                })?;
            let mut conn = self.items.db().conn()?;
            let tx = conn.immediate()?;
            let updated = PageRepository::save_in(
                &tx,
                &Page {
                    created_date: if key == "created" {
                        millis
                    } else {
                        page.created_date
                    },
                    modified_date: if key == "modified" {
                        millis
                    } else {
                        self.items.db().now_millis()
                    },
                    ..page.clone()
                },
            )?;
            tx.commit()?;
            self.export.queue_export(&page.id);
            return Ok(updated);
        }
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        // A type is chosen only when a key is defined (ADR 047), so a key with no definition
        // takes no value
        let definition = PropertyRepository::find_definition_in(&tx, key)?
            .ok_or_else(|| AppError::bad_request(format!("Property '{key}' is not defined")))?;
        if !definition.page {
            return Err(AppError::bad_request(format!(
                "Property '{key}' cannot be set on a page"
            )));
        }
        let value_type = definition.value_type.as_str();
        let mut effects = AfterCommit::default();
        if value_type == "tags" || value_type == "list" {
            let values = list_values(value);
            if key == "tags" {
                let before = PropertyRepository::find_tags_in(&tx, PAGE_TARGET, page_id.as_str())?;
                PropertyRepository::sync_tags_in(&tx, PAGE_TARGET, page_id.as_str(), &values)?;
                effects.tag_changes.push((before, values));
            } else {
                PropertyRepository::upsert_list_in(
                    &tx,
                    PAGE_TARGET,
                    page_id.as_str(),
                    key,
                    &values,
                )?;
            }
        } else {
            PropertyRepository::upsert_scalar_in(&tx, PAGE_TARGET, page_id.as_str(), key, value)?;
        }
        self.touch_and_commit(tx, &page, effects)
    }

    /// Sets the tags of a page as an imported file has them, and returns the tags it had.
    /// The tag pages are left to the caller, which knows when the import is complete.
    /// The tags of the page.
    pub fn page_tags(&self, page_id: &Hmtid) -> Result<Vec<String>, AppError> {
        let conn = self.items.db().conn()?;
        PropertyRepository::find_tags_in(&conn, PAGE_TARGET, page_id.as_str())
    }

    pub fn replace_page_tags(
        &self,
        page_id: &Hmtid,
        tags: &[String],
    ) -> Result<Vec<String>, AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let before = PropertyRepository::find_tags_in(&tx, PAGE_TARGET, page_id.as_str())?;
        if before != tags {
            PropertyRepository::sync_tags_in(&tx, PAGE_TARGET, page_id.as_str(), tags)?;
        }
        tx.commit()?;
        Ok(before)
    }

    /// Makes the properties of a page match the front matter of an imported file. A key with
    /// no definition is defined as text. The tags are not among them; see `replace_page_tags`.
    /// Returns true if anything changed.
    pub fn replace_page_properties(
        &self,
        page_id: &Hmtid,
        front_matter: &BTreeMap<String, String>,
    ) -> Result<bool, AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let id = page_id.to_string();
        let ids = std::slice::from_ref(&id);
        let scalars = PropertyRepository::find_scalar_by_targets_in(&tx, PAGE_TARGET, ids)?
            .remove(&id)
            .unwrap_or_default();
        let mut lists = PropertyRepository::find_list_by_targets_in(&tx, PAGE_TARGET, ids)?
            .remove(&id)
            .unwrap_or_default();
        lists.remove("tags");
        let definitions = PropertyRepository::find_definitions_in(&tx)?;
        let mut changed = false;
        for (key, value) in front_matter {
            let definition = definitions.iter().find(|def| def.key == *key);
            // `for_page` only chooses the keys the UI offers on a page: another device may let
            // a page have the key (ADR 046). The system keys are alike on every device.
            if FRONT_MATTER_KEYS.contains(&key.as_str())
                || definition.is_some_and(|def| def.system && !def.page)
            {
                continue;
            }
            // The name tells a list, so a key comes in as a list even with no definition
            if PropertyKey::parse(key).is_ok_and(|key| key.is_user_list()) {
                let values = list_values(value);
                if lists.get(key).is_none_or(|have| *have != values) {
                    PropertyRepository::take_in_list_in(&tx, PAGE_TARGET, &id, key, &values)?;
                    changed = true;
                }
            } else if scalars.get(key) != Some(value) {
                // A line written by hand may not fit; the rest of the file still comes in
                match PropertyRepository::take_in_scalar_in(&tx, PAGE_TARGET, &id, key, value) {
                    Ok(()) => changed = true,
                    Err(e) => {
                        tracing::warn!("Skipping property {key}={value} on page {page_id}: {e}")
                    }
                }
            }
        }
        let gone = scalars
            .keys()
            .chain(lists.keys())
            .filter(|key| !front_matter.contains_key(*key));
        for key in gone {
            PropertyRepository::delete_property_in(&tx, PAGE_TARGET, &id, key)?;
            changed = true;
        }
        PropertyRepository::drop_unused_provisional_in(&tx)?;
        tx.commit()?;
        Ok(changed)
    }

    /// Keeps the tag pages in step with a change of tags on one page or item: the added
    /// tags get their pages, and the removed ones lose theirs when nothing else uses them.
    pub fn update_tag_pages(&self, before: &[String], after: &[String]) {
        let added: Vec<String> = after
            .iter()
            .filter(|t| !before.contains(t))
            .cloned()
            .collect();
        let removed: Vec<String> = before
            .iter()
            .filter(|t| !after.contains(t))
            .cloned()
            .collect();
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
    pub fn ensure_all_tag_pages(&self) -> Result<usize, AppError> {
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
    fn delete_tag_page_if_unused(&self, tag: &str, dir: &str, name: &str) -> Result<(), AppError> {
        if self.properties.count_tag_uses_ignoring_case(tag)? > 0 {
            return Ok(());
        }
        for page in self
            .pages
            .find_by_type_dir_path_file_name_ignoring_case("doc", dir, name)?
        {
            if self.is_empty_page(&page)? {
                self.delete_page(&page.id)?;
            }
        }
        Ok(())
    }

    /// A page with one empty item, as a new page has.
    fn is_empty_page(&self, page: &Page) -> Result<bool, AppError> {
        let items = self.items.find_by_page_id(&page.id)?;
        Ok(
            matches!(items.as_slice(), [only] if only.body.trim().is_empty() && only.transclude_item_id.is_none()),
        )
    }

    /// Deletes the tag pages that only repeat an older one in another case, like
    /// `NextJS` beside `Nextjs`. Such pages were made before tag pages were matched ignoring
    /// case, and they overwrite the file of the older page. Only empty ones go; the older page
    /// is written again at once, since its file was the one deleted. Returns how many went.
    pub fn delete_case_duplicate_tag_pages(&self) -> Result<usize, AppError> {
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
            let pages = self
                .pages
                .find_by_type_dir_path_file_name_ignoring_case("doc", &dir, &name)?;
            let Some((kept, others)) = pages.split_first() else {
                continue;
            };
            let mut any = false;
            for page in others {
                if self.is_empty_page(page)? {
                    self.delete_page(&page.id)?;
                    deleted += 1;
                    any = true;
                } else {
                    tracing::warn!(
                        "{}{} repeats {}{} in another case but is not empty",
                        page.dir_path,
                        page.name,
                        kept.dir_path,
                        kept.name
                    );
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
    pub fn delete_property(&self, page_id: &Hmtid, key: &str) -> Result<Page, AppError> {
        if key == "created" || key == "modified" {
            return Err(AppError::bad_request(format!(
                "Property '{key}' cannot be removed from a page"
            )));
        }
        let page = self.require_page(page_id)?;
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let before = if key == "tags" {
            PropertyRepository::find_tags_in(&tx, PAGE_TARGET, page_id.as_str())?
        } else {
            Vec::new()
        };
        PropertyRepository::delete_property_in(&tx, PAGE_TARGET, page_id.as_str(), key)?;
        let mut effects = AfterCommit::default();
        effects.tag_changes.push((before, Vec::new()));
        self.touch_and_commit(tx, &page, effects)
    }

    /// Removes a user property key everywhere. Every page that lost a value, on itself or
    /// on one of its items, gets a new modified date and its file rewritten; their ids come
    /// back so the caller can tell the clients.
    pub fn delete_property_definition(&self, key: &str) -> Result<Vec<Hmtid>, AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        let mut touched = Vec::new();
        let now = self.items.db().now_millis();
        for id in PropertyRepository::delete_definition_in(&tx, key, now)? {
            let Ok(page_id) = Hmtid::parse(&id) else {
                continue;
            };
            if PageRepository::find_by_id_in(&tx, &page_id)?.is_some() {
                PageRepository::touch_in(&tx, &page_id, now)?;
                effects.exported.insert(page_id.clone());
                touched.push(page_id);
            }
        }
        tx.commit()?;
        self.finish(effects);
        Ok(touched)
    }

    /// Gives the page a new modified date, commits, and queues its file: what every change
    /// of a page does. Returns the page as it is now.
    fn touch_and_commit(
        &self,
        tx: Transaction<'_>,
        page: &Page,
        mut effects: AfterCommit,
    ) -> Result<Page, AppError> {
        let updated = PageRepository::save_in(
            &tx,
            &Page {
                modified_date: self.items.db().now_millis(),
                ..page.clone()
            },
        )?;
        tx.commit()?;
        effects.exported.insert(page.id.clone());
        self.finish(effects);
        Ok(updated)
    }

    /// The work that lives outside the database, done once the commit is through.
    fn finish(&self, effects: AfterCommit) {
        for id in &effects.removed_page_ids {
            self.search.remove_page(id);
        }
        for (page, items) in &effects.indexed {
            self.search.index_page(page);
            let title = page.title();
            for item in items {
                self.search.index_item(item, &title);
            }
        }
        for page_id in &effects.exported {
            self.export.queue_export(page_id);
        }
        for page in &effects.deleted {
            // Remove the markdown file too, otherwise the folder scan would bring the page back
            self.export.delete_page_file(page);
            // An atlas keeps the card of a page it does not know, so a deleted page says so
            if let Err(e) = self.atlas.forget_page(&page.id) {
                tracing::warn!(
                    "Could not take the deleted page {} off the atlases: {e}",
                    page.id
                );
            }
        }
        for (before, after) in &effects.tag_changes {
            self.update_tag_pages(before, after);
        }
    }

    /// Deletes a page with all its items and their properties. The tags of the page and of
    /// its items go with them, so their tag pages are deleted when nothing else uses them.
    pub fn delete_page(&self, page_id: &Hmtid) -> Result<(), AppError> {
        let page = self.require_page(page_id)?;
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        self.delete_page_in(&tx, &page, &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(())
    }

    /// Deletes a page with its items and their properties, in the transaction. Its file, its
    /// card on the atlases and the tag pages its tags held up go after the commit.
    fn delete_page_in(
        &self,
        tx: &Connection,
        page: &Page,
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
        let page_id = &page.id;
        if page.name == "index" && page.dir_path.is_empty() {
            return Err(AppError::bad_request("Cannot delete the root index page."));
        }
        if page.name == "index" {
            let children = PageRepository::find_by_parent_id_in(tx, page_id)?;
            if !children.is_empty() {
                return Err(AppError::bad_request(format!(
                    "Cannot delete index page with {} child page(s). Delete or move child pages first.",
                    children.len()
                )));
            }
        }

        let item_ids: Vec<String> = ItemRepository::find_by_page_id_in(tx, page_id)?
            .iter()
            .map(|i| i.id.to_string())
            .collect();
        let mut tags = PropertyRepository::find_tags_in(tx, PAGE_TARGET, page_id.as_str())?;
        for id in &item_ids {
            tags.extend(PropertyRepository::find_tags_in(tx, "item", id)?);
        }
        tags.sort();
        tags.dedup();
        PropertyRepository::delete_all_for_targets_in(tx, "item", &item_ids)?;
        PropertyRepository::delete_all_for_targets_in(
            tx,
            PAGE_TARGET,
            std::slice::from_ref(&page_id.to_string()),
        )?;
        // The items, their types and the page's place in the export queue go with the page,
        // by ON DELETE CASCADE
        PageRepository::delete_in(tx, page_id)?;
        effects.removed_page_ids.push(page_id.to_string());
        effects.deleted.push(page.clone());
        effects.tag_changes.push((tags, Vec::new()));
        effects.atlases_stale = true;
        Ok(())
    }

    pub fn get_recent_pages(
        &self,
        max_results: i64,
        page_type: Option<&str>,
    ) -> Result<Vec<Page>, AppError> {
        self.pages.find_recent(max_results, page_type)
    }

    /// The pages and items that link to the given page (backlinks). A page links by having
    /// the tag of a tag page in its tags (an entry without an item); an item links by a
    /// hashtag of that tag or by a wikilink to the page. Paths and tags are matched ignoring
    /// case, as pages are found. The page's own items are left out. Pages come newest first,
    /// each with its tag entry before its items, and the items in page order.
    pub fn get_backlinks(&self, page_id: &Hmtid) -> Result<Vec<BacklinkEntry>, AppError> {
        let Some(target) = self.pages.find_by_id(page_id)? else {
            return Ok(Vec::new());
        };
        let mut tagging_pages: BTreeSet<String> = BTreeSet::new();
        let mut linking_items: BTreeSet<String> = BTreeSet::new();
        if let Some(tag) = self.tag_of_page(&target) {
            tagging_pages.extend(
                self.properties
                    .find_page_ids_by_tag_value_ignoring_case(&tag)?,
            );
            linking_items.extend(
                self.properties
                    .find_item_ids_by_tag_value_ignoring_case(&tag, i64::MAX)?,
            );
        }
        let paths = link_paths_of(&target);
        for path in &paths {
            for item in self.items.find_by_body_containing_link_to(path)? {
                let links = wikilink::extract_wikilink_paths(&MarkdownParser.parse(&item.body));
                if links
                    .iter()
                    .any(|l| paths.iter().any(|p| l.eq_ignore_ascii_case(p)))
                {
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
            let Ok(item_id) = Hmtid::parse(&id) else {
                continue;
            };
            if let Some(item) = self.items.find_by_id(&item_id)? {
                by_page
                    .entry(item.page_id.to_string())
                    .or_default()
                    .1
                    .push(item);
            }
        }
        by_page.remove(page_id.as_str());

        let mut sources: Vec<(Page, bool, Vec<Item>)> = Vec::new();
        for (id, (tagged, items)) in by_page {
            let Ok(source_id) = Hmtid::parse(&id) else {
                continue;
            };
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
    pub fn tag_of_page(&self, page: &Page) -> Option<String> {
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

    /// Ensures the ancestor directory pages of a file being taken in. Their files are not
    /// written: the device that sent the file may send its index files too.
    pub fn ensure_ancestor_pages_from_file(&self, dir_path: &str) -> Result<(), AppError> {
        let mut conn = self.items.db().conn()?;
        let tx = conn.immediate()?;
        let mut effects = AfterCommit::default();
        self.make_ancestor_pages_in(&tx, dir_path, false, &mut effects)?;
        tx.commit()?;
        self.finish(effects);
        Ok(())
    }

    /// Makes every ancestor directory page of the given dirPath that is missing, in the
    /// transaction. For example, dirPath "dir1/dir2/" creates pages ("dir1/", "index") and
    /// ("dir1/dir2/", "index"). With `export`, their files are written too.
    fn make_ancestor_pages_in(
        &self,
        tx: &Connection,
        dir_path: &str,
        export: bool,
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
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

            if PageRepository::find_by_type_dir_path_file_name_in(tx, "doc", &index_dir, "index")?
                .is_some()
            {
                continue;
            }
            // Check if a named page should be promoted to index
            if let Some(page) =
                PageRepository::find_by_type_dir_path_file_name_in(tx, "doc", &parent_dir, segment)?
            {
                let renamed = PageRepository::save_in(
                    tx,
                    &Page {
                        dir_path: index_dir.clone(),
                        name: "index".to_string(),
                        modified_date: self.items.db().now_millis(),
                        ..page
                    },
                )?;
                if export {
                    effects.exported.insert(renamed.id.clone());
                }
                effects.indexed.push((renamed, Vec::new()));
            } else {
                let now = self.items.db().now_millis();
                let page = Page::new(self.ids.generate_page_id(), &index_dir, "index", "doc", now);
                let page = PageRepository::save_in(tx, &page)?;
                ItemRepository::save_in(
                    tx,
                    &Item::new_on(self.ids.generate_item_id(), &page, now),
                )?;
                if export {
                    // A file at once, so another device takes this index page and does not
                    // make one of its own
                    effects.exported.insert(page.id.clone());
                }
                effects.indexed.push((page, Vec::new()));
            }
        }
        Ok(())
    }

    /// Recursively propagates a dir_path change to all child pages, in the transaction.
    fn propagate_dir_path_change_in(
        &self,
        tx: &Connection,
        parent_page_id: &Hmtid,
        old_prefix: &str,
        new_prefix: &str,
        effects: &mut AfterCommit,
    ) -> Result<(), AppError> {
        for child in PageRepository::find_by_parent_id_in(tx, parent_page_id)? {
            let suffix = child.dir_path.get(old_prefix.len()..).unwrap_or("");
            let child_new_dir = format!("{new_prefix}{suffix}");
            let updated = PageRepository::save_in(
                tx,
                &Page {
                    dir_path: child_new_dir.clone(),
                    ..child.clone()
                },
            )?;
            effects
                .indexed
                .push((updated, ItemRepository::find_by_page_id_in(tx, &child.id)?));
            effects.exported.insert(child.id.clone());
            if child.name == "index" {
                self.propagate_dir_path_change_in(
                    tx,
                    &child.id,
                    &child.dir_path,
                    &child_new_dir,
                    effects,
                )?;
            }
        }
        Ok(())
    }
}
