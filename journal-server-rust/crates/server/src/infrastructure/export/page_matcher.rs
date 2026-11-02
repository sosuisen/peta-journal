//! Decides which page a markdown file belongs to, and saves pages at the location their file has.
//! 1. By frontmatter id: same path is the page itself; a new path is a rename, unless the old
//!    file still exists. The file may be the page moved, with the old file not taken away
//!    yet, so the caller waits for that first. When a copy is allowed, the file is a copy and
//!    gets a page of its own.
//! 2. By path: a file without an id gets ours. A file with another id comes from a device that
//!    created the same page independently; the older id wins on every device.
//! 3. Otherwise a new page is created, keeping the file's id when it has one.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use hmtid::Hmtid;

use crate::application::page_service::PageService;
use crate::domain::id_generator::IdGenerator;
use crate::domain::model::Page;
use crate::domain::search::SearchIndex;
use crate::error::{AppError, AppResult};
use crate::infrastructure::persistence::{ItemRepository, PageRepository};

use super::path_mapper::{self, PageLocation};

/// The page a file belongs to.
#[derive(Debug, Clone)]
pub struct PageMatch {
    /// The page, already saved when it is new.
    pub page: Page,
    /// The page was created for this file.
    pub is_new: bool,
    /// The page exists but its file moved to this path; see `move_page`.
    pub renamed: bool,
    /// The file lacks our id or carries a foreign one and must be rewritten after the import.
    pub write_back: bool,
}

/// What `match_file` found for a file.
#[derive(Debug, Clone)]
pub enum Matched {
    Page(PageMatch),
    /// The file has the id of a page whose file at `other` is still there, and a copy is not
    /// allowed yet.
    Elsewhere {
        other: String,
    },
}

pub struct PageMatcher {
    pages: PageRepository,
    items: ItemRepository,
    search: Arc<dyn SearchIndex>,
    ids: Arc<IdGenerator>,
    page_service: Arc<PageService>,
}

impl PageMatcher {
    pub fn new(
        pages: PageRepository,
        items: ItemRepository,
        search: Arc<dyn SearchIndex>,
        ids: Arc<IdGenerator>,
        page_service: Arc<PageService>,
    ) -> PageMatcher {
        PageMatcher {
            pages,
            items,
            search,
            ids,
            page_service,
        }
    }

    /// Finds the page a file belongs to, or creates one. A file with the id of a page whose
    /// file is still elsewhere becomes a new page only when `copy_allowed`.
    pub fn match_file(
        &self,
        root: &Path,
        rel_path: &str,
        loc: &PageLocation,
        frontmatter: &BTreeMap<String, String>,
        file_modified: i64,
        copy_allowed: bool,
    ) -> AppResult<Matched> {
        let mut file_id = parse_id(frontmatter.get("id").map(String::as_str));
        let mut write_back = false;

        // 1. Match by frontmatter id
        if let Some(fid) = &file_id {
            if let Some(p) = self.pages.find_by_id(fid)? {
                let current_rel = path_mapper::to_rel_path_of(&p);
                if current_rel == rel_path {
                    return Ok(Matched::Page(PageMatch {
                        page: p,
                        is_new: false,
                        renamed: false,
                        write_back: false,
                    }));
                }
                if !root.join(&current_rel).exists() {
                    tracing::info!("Page {} moved: {current_rel} -> {rel_path}", p.id);
                    return Ok(Matched::Page(PageMatch {
                        page: p,
                        is_new: false,
                        renamed: true,
                        write_back: false,
                    }));
                }
                if !copy_allowed {
                    return Ok(Matched::Elsewhere { other: current_rel });
                }
                // Same id in two files: this one is a copy, give it a new identity
                tracing::info!("{rel_path} is a copy of {current_rel}, creating a new page");
                file_id = None;
                write_back = true;
            }
        }

        // 2. Match by path
        if let Some(mut page) =
            self.pages
                .find_by_type_dir_path_file_name(&loc.page_type, &loc.dir_path, &loc.name)?
        {
            match &file_id {
                None => write_back = true, // hand-made file: give it our id
                Some(fid) if *fid != page.id => {
                    // Two devices created the same page independently (typical for today's journal).
                    // Both must end up with one id, so the older id wins on every device.
                    if fid < &page.id {
                        page = self.adopt_page_id(&page, fid, rel_path)?;
                    } else {
                        write_back = true; // our id is older: rewrite the file with it
                    }
                }
                Some(_) => {}
            }
            return Ok(Matched::Page(PageMatch {
                page,
                is_new: false,
                renamed: false,
                write_back,
            }));
        }

        // 3. New page
        let id = match file_id {
            Some(fid) => fid,
            None => {
                write_back = true;
                self.ids.generate_page_id()
            }
        };
        let created =
            parse_instant(frontmatter.get("created").map(String::as_str)).unwrap_or(file_modified);
        let page = Page {
            modified_date: file_modified,
            ..Page::new(id, &loc.dir_path, &loc.name, &loc.page_type, created)
        };
        let page = self.save(&page, loc, file_modified)?;
        tracing::info!("Created page {} from {rel_path}", page.id);
        Ok(Matched::Page(PageMatch {
            page,
            is_new: true,
            renamed: false,
            write_back,
        }))
    }

    /// Saves a page at the location of its file, creating the index pages above it first,
    /// and reindexes its items under the new title.
    pub fn move_page(&self, page: &Page, loc: &PageLocation, modified: i64) -> AppResult<Page> {
        let moved = self.save(page, loc, modified)?;
        self.search
            .index_items(&self.items.find_by_page_id(&moved.id)?, &moved.title());
        Ok(moved)
    }

    /// Saves a page with a new modified date at a location whose index pages exist already.
    pub fn touch(&self, page: &Page, loc: &PageLocation, modified: i64) -> AppResult<Page> {
        let saved = self.pages.save(&Page {
            dir_path: loc.dir_path.clone(),
            name: loc.name.clone(),
            page_type: loc.page_type.clone(),
            modified_date: modified,
            ..page.clone()
        })?;
        self.search.index_page(&saved);
        Ok(saved)
    }

    fn save(&self, page: &Page, loc: &PageLocation, modified: i64) -> AppResult<Page> {
        if loc.page_type == "doc" && !loc.dir_path.is_empty() && loc.name != "index" {
            // The index file may be on its way from the device that sent this file; the scan
            // writes one later when none comes
            self.page_service
                .ensure_ancestor_pages_from_file(&loc.dir_path)?;
        }
        self.touch(page, loc, modified)
    }

    /// Gives a page the id another device chose for it and reindexes it under that id.
    fn adopt_page_id(&self, page: &Page, new_id: &Hmtid, rel_path: &str) -> AppResult<Page> {
        tracing::info!(
            "Adopting page id {new_id} from {rel_path} (was {})",
            page.id
        );
        self.search.remove_page(page.id.as_str());
        self.pages.rekey(&page.id, new_id)?;
        let rekeyed = self
            .pages
            .find_by_id(new_id)?
            .ok_or_else(|| AppError::internal(format!("Page vanished after rekey: {new_id}")))?;
        self.search.index_page(&rekeyed);
        self.search
            .index_items(&self.items.find_by_page_id(&rekeyed.id)?, &rekeyed.title());
        Ok(rekeyed)
    }
}

pub fn parse_id(s: Option<&str>) -> Option<Hmtid> {
    s.filter(|s| !s.is_empty())
        .and_then(|s| Hmtid::parse(s).ok())
}

/// Parses an ISO-8601 instant like `2026-09-27T10:15:00Z` to epoch millis.
pub fn parse_instant(s: Option<&str>) -> Option<i64> {
    let s = s.filter(|s| !s.is_empty())?;
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_instants() {
        assert_eq!(parse_instant(Some("1970-01-01T00:00:01Z")), Some(1000));
        assert_eq!(parse_instant(Some("1970-01-01T00:00:00.500Z")), Some(500));
        assert_eq!(parse_instant(Some("bad")), None);
        assert_eq!(parse_instant(None), None);
    }
}
