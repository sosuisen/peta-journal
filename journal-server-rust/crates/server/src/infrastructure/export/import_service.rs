//! Brings changes made to markdown files (by hand, by another tool, or by Dropbox from another
//! device) into the database.
//!
//! Rules:
//! - A file whose content hash matches `file_sync_state` is our own write: skipped.
//! - Pages are matched by frontmatter id first, then by path; see `PageMatcher`.
//! - Last writer wins by `modified`: when the DB is newer than the file, the file is backed up as
//!   `name.conflict-yyyyMMdd-HHmmss.md` and overwritten from the DB.
//! - Items are merged by id; see `ItemMerger`.
//! - Nothing is deleted from the DB because a file disappeared; that goes through the
//!   pending-deletes page.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use parking_lot::Mutex;

use crate::application::page_service::PageService;
use crate::domain::model::Page;
use crate::error::AppResult;
use crate::infrastructure::persistence::db::now_millis;
use crate::presentation::sse::SseBroadcaster;

use super::export_service::{mtime_millis, MarkdownExportService};
use super::file_sync_state::FileSyncStateRepository;
use super::item_merger::ItemMerger;
use super::outline_parser::{OutlineMarkdownParser, ParsedItem};
use super::page_matcher::{parse_instant, PageMatch, PageMatcher};
use super::path_mapper::{self, PageLocation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ignored,
    Missing,
    Unchanged,
    Imported,
    Conflict,
    Error,
}

pub struct MarkdownImportService {
    parser: OutlineMarkdownParser,
    page_matcher: PageMatcher,
    item_merger: ItemMerger,
    file_sync_state: FileSyncStateRepository,
    export: Arc<MarkdownExportService>,
    sse: SseBroadcaster,
    /// Takes the tags of the front matter, and keeps the tag pages in step with them and
    /// with the hashtags of the imported bodies.
    page_service: Arc<PageService>,
    /// One file at a time, like the Java `synchronized` method.
    lock: Mutex<()>,
}

struct FileAttrs {
    size: i64,
    mtime: i64,
}

impl MarkdownImportService {
    pub fn new(
        parser: OutlineMarkdownParser,
        page_matcher: PageMatcher,
        item_merger: ItemMerger,
        file_sync_state: FileSyncStateRepository,
        export: Arc<MarkdownExportService>,
        sse: SseBroadcaster,
        page_service: Arc<PageService>,
    ) -> MarkdownImportService {
        MarkdownImportService {
            parser,
            page_matcher,
            item_merger,
            file_sync_state,
            export,
            sse,
            page_service,
            lock: Mutex::new(()),
        }
    }

    /// Syncs one file from the folder into the DB. Safe to call repeatedly.
    pub fn sync_file(&self, root: &Path, rel_path: &str) -> Outcome {
        let _guard = self.lock.lock();
        if path_mapper::is_ignored(rel_path) {
            return Outcome::Ignored;
        }
        let Some(loc) = path_mapper::from_rel_path(rel_path) else {
            return Outcome::Ignored;
        };
        let file = root.join(rel_path);
        if !file.is_file() {
            return Outcome::Missing;
        }
        let (content, attrs) = match read_stable(&file).and_then(|c| {
            let meta = std::fs::metadata(&file)?;
            Ok((
                c,
                FileAttrs {
                    size: meta.len() as i64,
                    mtime: mtime_millis(&meta),
                },
            ))
        }) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Cannot read {}: {e}", file.display());
                return Outcome::Error;
            }
        };

        let hash = path_mapper::sha256(&content);
        let state = match self.file_sync_state.find_by_path(rel_path) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to read sync state of {rel_path}: {e}");
                return Outcome::Error;
            }
        };
        if let Some(state) = state {
            if hash == state.content_hash {
                // Our own write (or an already imported version): just refresh the bookkeeping
                if state.is_missing()
                    || state.file_size != attrs.size
                    || state.file_mtime != attrs.mtime
                {
                    if let Err(e) = self.file_sync_state.upsert_synced(
                        rel_path,
                        &state.page_id,
                        &hash,
                        attrs.size,
                        attrs.mtime,
                    ) {
                        tracing::warn!("Failed to refresh sync state of {rel_path}: {e}");
                    }
                }
                return Outcome::Unchanged;
            }
        }

        match self.import_content(root, rel_path, &loc, content, &hash, &attrs) {
            Ok(outcome) => outcome,
            Err(e) => {
                tracing::error!("Failed to import {rel_path}: {e:#}");
                Outcome::Error
            }
        }
    }

    fn import_content(
        &self,
        root: &Path,
        rel_path: &str,
        loc: &PageLocation,
        content: String,
        hash: &str,
        attrs: &FileAttrs,
    ) -> AppResult<Outcome> {
        let started = std::time::Instant::now();
        if content.trim().is_empty() {
            // Sync tools and some editors create an empty file first and fill it in later.
            // Don't create or wipe a page for that; the watcher retries and the scan picks it up later.
            tracing::debug!("{rel_path} is empty, waiting for content");
            return Ok(Outcome::Error);
        }

        // Editors on Windows may save CRLF; the metadata comments are matched per line
        let content = content.replace("\r\n", "\n");
        let parsed = self.parser.parse(&content)?;
        let fm = &parsed.frontmatter;

        let file_modified =
            parse_instant(fm.get("modified").map(String::as_str)).unwrap_or(attrs.mtime);
        let now = now_millis();

        let PageMatch {
            mut page,
            is_new,
            renamed,
            write_back,
        } = self
            .page_matcher
            .match_file(root, rel_path, loc, fm, file_modified)?;

        if !is_new
            && file_modified < page.modified_date
            && !self.file_sync_state.find_by_page_id(&page.id)?.is_empty()
        {
            // DB is newer: keep the DB, keep the file's content as a backup, then overwrite the file.
            // A moved file still moves the page, so the export lands at the new path.
            // A page that never had a file on this device (e.g. the root index a fresh install creates)
            // is not "newer" in any meaningful way, so the file wins and this branch is skipped.
            if renamed {
                page = self
                    .page_matcher
                    .move_page(&page, loc, page.modified_date)?;
            }
            self.backup_conflict(root, rel_path, loc, &content, hash, &page, attrs)?;
            self.export.queue_export(&page.id);
            return Ok(Outcome::Conflict);
        }

        let empty = [ParsedItem::empty()];
        let items: &[ParsedItem] = if parsed.items.is_empty() {
            &empty
        } else {
            &parsed.items
        };
        let merge = self.item_merger.merge(&page, items, now)?;
        // The file is the newer version of the page, so a file without tags has none
        let page_tags = frontmatter_tags(fm);
        let page_tags_before = self.page_service.replace_page_tags(&page.id, &page_tags)?;

        let page_changed = renamed || merge.changed || is_new || page_tags_before != page_tags;
        if page_changed {
            let new_modified = if is_new {
                page.modified_date
            } else if file_modified > page.modified_date {
                file_modified
            } else {
                now
            };
            page = if renamed {
                self.page_matcher.move_page(&page, loc, new_modified)?
            } else {
                self.page_matcher.touch(&page, loc, new_modified)?
            };
        } else if file_modified > page.modified_date {
            // The same page written later on another device: the page takes the date of the
            // file, or a later change here would count as older than the file there
            page = self.page_matcher.touch(&page, loc, file_modified)?;
        }

        self.file_sync_state
            .upsert_synced(rel_path, &page.id, hash, attrs.size, attrs.mtime)?;

        if write_back || merge.needs_write_back || !tags_written_as_exported(fm, &page_tags) {
            // Give the file its ids / frontmatter. The export records the new hash, so it won't echo back.
            self.export.queue_export(&page.id);
        }
        // Last, when the page and its file are recorded: a tag page deleted here may be
        // this very page, and nothing may save it again afterwards
        self.page_service.update_tag_pages(
            &union(&merge.tags_before, &page_tags_before),
            &union(&merge.tags_after, &page_tags),
        );
        // A new or moved file changes the pages of a directory. An atlas file that only
        // changed came from another device, which wrote it from its pages. Writing it again
        // here from pages that differ for a while, as a rename on its way, would send it back
        // and forth until the pages agree; a page file that comes writes it again anyway.
        if page_changed && (is_new || renamed || !page.is_atlas()) {
            self.page_service.sync_atlases();
        }
        if page_changed {
            self.sse
                .broadcast_pages_updated_from(page.id.as_str(), "file");
            tracing::info!(
                "Imported {rel_path} ({} items, {}, {} ms)",
                items.len(),
                if is_new {
                    "new page"
                } else if renamed {
                    "renamed"
                } else {
                    "updated"
                },
                started.elapsed().as_millis()
            );
        }
        Ok(Outcome::Imported)
    }

    #[allow(clippy::too_many_arguments)]
    fn backup_conflict(
        &self,
        root: &Path,
        rel_path: &str,
        loc: &PageLocation,
        content: &str,
        hash: &str,
        page: &Page,
        attrs: &FileAttrs,
    ) -> AppResult<()> {
        let file = root.join(rel_path);
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let backup = file.with_file_name(path_mapper::conflict_file_name(&loc.name, &ts));
        match std::fs::write(&backup, content) {
            Ok(_) => {
                tracing::warn!(
                "{rel_path} is older than the database version; kept it as {} and restored from DB",
                backup.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
            )
            }
            Err(e) => tracing::error!("Could not write conflict backup {}: {e}", backup.display()),
        }
        // Remember this content so we don't back it up again before the export overwrites it
        self.file_sync_state
            .upsert_synced(rel_path, &page.id, hash, attrs.size, attrs.mtime)?;
        Ok(())
    }
}

/// Reads the file, retrying briefly if it is still being written (Dropbox and editors may create
/// the file first and fill it in afterwards).
/// The `tags` list of the front matter. The parser joins a block list with commas; a flow
/// list `[a, b]` and a single value are taken too. A `#` typed out of habit is dropped.
/// Files written before the key was renamed have `tag`, which is read when `tags` is absent.
fn frontmatter_tags(fm: &BTreeMap<String, String>) -> Vec<String> {
    let Some(raw) = fm.get("tags").or_else(|| fm.get("tag")) else {
        return Vec::new();
    };
    let raw = raw.trim();
    let raw = raw
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .unwrap_or(raw);
    let mut tags: Vec<String> = Vec::new();
    for t in raw.split(',') {
        let t = t
            .trim()
            .trim_matches(|c| c == '"' || c == '\'')
            .trim_start_matches('#')
            .trim();
        if !t.is_empty() && !tags.iter().any(|x| x == t) {
            tags.push(t.to_string());
        }
    }
    tags
}

/// True when the front matter holds the tags as the export writes them: a `tags` block list
/// of the plain values, or no key at all for no tags. An old `tag` key, a `#`, a flow list or
/// a duplicate is rewritten, so the file and the page agree.
fn tags_written_as_exported(fm: &BTreeMap<String, String>, tags: &[String]) -> bool {
    let expected = (!tags.is_empty()).then(|| tags.join(","));
    !fm.contains_key("tag") && fm.get("tags") == expected.as_ref()
}

fn union(a: &[String], b: &[String]) -> Vec<String> {
    let mut all = a.to_vec();
    all.extend(b.iter().filter(|t| !a.contains(t)).cloned());
    all
}

fn read_stable(file: &Path) -> std::io::Result<String> {
    let mut last: Option<std::io::Error> = None;
    for attempt in 0..3u64 {
        match std::fs::metadata(file)
            .and_then(|m| std::fs::read_to_string(file).map(|c| (m.len(), c)))
        {
            Ok((size1, content)) => {
                if std::fs::metadata(file).map(|m| m.len()).unwrap_or(u64::MAX) == size1 {
                    return Ok(content);
                }
            }
            Err(e) => last = Some(e),
        }
        std::thread::sleep(std::time::Duration::from_millis(200 * (attempt + 1)));
    }
    if let Some(e) = last {
        return Err(e);
    }
    std::fs::read_to_string(file)
}
