//! Brings changes made to markdown files (by hand, by another tool, or by Dropbox from another
//! device) into the database.
//!
//! Rules:
//! - A file whose content hash matches `file_sync_state` is our own write: skipped.
//! - Pages are matched by frontmatter id first, then by path; see `PageMatcher`.
//! - A file with the id of a page whose file is still elsewhere waits: a move may bring the
//!   new file before it takes the old one away. It comes in as the page moved once the old
//!   file goes, or as a copy when the old file is still there after a while.
//! - Last writer wins by `modified`, for each item; see `ItemMerger`. When a change in the file
//!   that was not seen here loses, the file is backed up as `name.conflict-yyyyMMdd-HHmmss.md`.
//! - The tags and the other properties of a page come from the front matter of a file newer
//!   than the page. A device that got a page only from its file must hold them, or the file
//!   loses them when that device writes it.
//! - An atlas goes by its page: when the DB is newer than the file, the file is backed up and
//!   overwritten from the DB.
//! - A Dropbox conflicted copy is taken in as one more version of its page, and then
//!   deleted. It never takes items away.
//! - Nothing is deleted from the DB because a file disappeared; that goes through the
//!   pending-deletes page.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::Mutex;

use crate::application::page_service::PageService;
use crate::domain::model::Page;
use crate::error::AppError;

use super::definitions_file::{self, DefinitionsFileService, DEFINITIONS_FILE};
use super::export_service::{mtime_millis, MarkdownExportService};
use super::file_sync_state::FileSyncStateRepository;
use super::item_merger::{ItemMerger, MergeMode};
use super::outline_parser::{OutlineMarkdownParser, ParsedItem};
use super::page_matcher::{parse_instant, Matched, PageMatch, PageMatcher};
use super::path_mapper::{self, PageLocation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Ignored,
    Missing,
    Unchanged,
    Imported,
    Conflict,
    /// The file waits for the file of the same page to go; see `MarkdownImportService`.
    Waiting,
    Error,
}

/// Hears of the pages that a file changed. The journal server tells its clients, which read
/// the page again.
pub trait ImportEvents: Send + Sync {
    fn page_changed(&self, page_id: &hmtid::Hmtid);
    /// The property definitions changed: a file brought them, or a key of a value (ADR 046).
    fn definitions_changed(&self);
}

/// A file that waits, with the file of the same page and when it began to wait.
struct Wait {
    other: String,
    since: Instant,
}

pub struct MarkdownImportService {
    parser: OutlineMarkdownParser,
    page_matcher: PageMatcher,
    item_merger: ItemMerger,
    file_sync_state: FileSyncStateRepository,
    export: Arc<MarkdownExportService>,
    events: Arc<dyn ImportEvents>,
    /// Takes the tags of the front matter, and keeps the tag pages in step with them and
    /// with the hashtags of the imported bodies.
    page_service: Arc<PageService>,
    /// Takes in `settings/properties.json`, which holds the definitions of the user keys
    definitions: Arc<DefinitionsFileService>,
    /// How long a file waits for the file of the same page to go before it is a copy.
    copy_wait: Duration,
    /// The files that wait, by path. They are lost on a restart; the scan brings them again.
    waiting: Mutex<BTreeMap<String, Wait>>,
    /// The items that show a source item not taken in yet, with that source. They are lost on
    /// a restart; the scan brings their files again only when they change, so the item keeps
    /// the body it had and shows no source.
    waiting_transclusions: Mutex<Vec<(hmtid::Hmtid, hmtid::Hmtid)>>,
    /// One file at a time, like the Java `synchronized` method.
    lock: Mutex<()>,
}

struct FileAttrs {
    size: i64,
    mtime: i64,
}

impl MarkdownImportService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        parser: OutlineMarkdownParser,
        page_matcher: PageMatcher,
        item_merger: ItemMerger,
        file_sync_state: FileSyncStateRepository,
        export: Arc<MarkdownExportService>,
        events: Arc<dyn ImportEvents>,
        page_service: Arc<PageService>,
        definitions: Arc<DefinitionsFileService>,
        copy_wait: Duration,
    ) -> MarkdownImportService {
        MarkdownImportService {
            parser,
            page_matcher,
            item_merger,
            file_sync_state,
            export,
            events,
            page_service,
            definitions,
            copy_wait,
            waiting: Mutex::new(BTreeMap::new()),
            waiting_transclusions: Mutex::new(Vec::new()),
            lock: Mutex::new(()),
        }
    }

    /// Syncs one file from the folder into the DB. Safe to call repeatedly. When the file is
    /// gone, the files that wait for it come in.
    pub fn sync_file(&self, root: &Path, rel_path: &str) -> Outcome {
        let _guard = self.lock.lock();
        let before = self.definitions.find_definitions();
        let outcome = self.sync_locked(root, rel_path);
        self.transclude_waiting();
        if self.definitions.find_definitions() != before {
            self.events.definitions_changed();
        }
        outcome
    }

    /// Sets the transclusions whose source has come in. Their pages are written again, so
    /// that the files keep the transclusions.
    fn transclude_waiting(&self) {
        let mut waiting = self.waiting_transclusions.lock();
        match self.item_merger.transclude_waiting(&mut waiting) {
            Ok(pages) => {
                for page_id in pages {
                    self.export.queue_export(&page_id);
                    self.events.page_changed(&page_id);
                }
            }
            Err(e) => tracing::error!("Failed to set the waiting transclusions: {e}"),
        }
    }

    fn sync_locked(&self, root: &Path, rel_path: &str) -> Outcome {
        if rel_path == DEFINITIONS_FILE {
            return self.definitions.take_in(root).unwrap_or_else(|e| {
                tracing::error!("Failed to take in {rel_path}: {e:#}");
                Outcome::Error
            });
        }
        if definitions_file::is_copy_of_the_file(rel_path) {
            return self
                .definitions
                .take_in_copy(root, rel_path)
                .unwrap_or_else(|e| {
                    tracing::error!("Failed to merge {rel_path}: {e:#}");
                    Outcome::Error
                });
        }
        if let Some(loc) = path_mapper::conflicted_copy_of(rel_path) {
            return self.merge_copy(root, rel_path, &loc).unwrap_or_else(|e| {
                tracing::error!("Failed to merge {rel_path}: {e:#}");
                Outcome::Error
            });
        }
        if path_mapper::is_ignored(rel_path) {
            return Outcome::Ignored;
        }
        let Some(loc) = path_mapper::from_rel_path(rel_path) else {
            return Outcome::Ignored;
        };
        let file = root.join(rel_path);
        if !file.is_file() {
            self.sync_waiting_for(root, rel_path);
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
                        state.last_synced,
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

    /// Takes in a conflicted copy as one more version of the page at `loc`, then deletes it.
    /// The other device needs no copy after that: the file of the page holds it all.
    fn merge_copy(
        &self,
        root: &Path,
        rel_path: &str,
        loc: &PageLocation,
    ) -> Result<Outcome, AppError> {
        let file = root.join(rel_path);
        if !file.is_file() {
            return Ok(Outcome::Ignored);
        }
        let page_path = path_mapper::to_rel_path(&loc.page_type, &loc.dir_path, &loc.name);
        let Some(state) = self.file_sync_state.find_by_path(&page_path)? else {
            return Ok(Outcome::Waiting);
        };
        let Some(page) = self.page_service.get_page(&state.page_id)? else {
            return Ok(Outcome::Waiting);
        };
        if page.is_atlas() {
            return Ok(Outcome::Ignored);
        }
        let content = read_stable(&file)?;
        if content.trim().is_empty() {
            return Ok(Outcome::Error);
        }
        // The version written here, while the version that won may still be on its way. Taken
        // in now, it would change nothing and be gone when that version drops its items.
        if path_mapper::sha256(&content) == state.content_hash {
            return Ok(Outcome::Waiting);
        }
        let content = content.replace("\r\n", "\n");
        let parsed = self.parser.parse(&content)?;
        let mtime = mtime_millis(&std::fs::metadata(&file)?);
        let file_modified =
            parse_instant(parsed.frontmatter.get("modified").map(String::as_str)).unwrap_or(mtime);
        let now = self.file_sync_state.db().now_millis();
        let mode = MergeMode::Copy {
            last_synced: state.last_synced,
        };
        let merge = self
            .item_merger
            .merge(&page, &parsed.items, file_modified, mode, now)?;
        let waits = !merge.waiting_transclusions.is_empty();
        self.waiting_transclusions
            .lock()
            .extend(merge.waiting_transclusions.iter().cloned());
        let page_tags = union(
            &self.page_service.page_tags(&page.id)?,
            &frontmatter_tags(&parsed.frontmatter),
        );
        let page_tags_before = self.page_service.replace_page_tags(&page.id, &page_tags)?;
        if merge.file_lost {
            self.write_conflict_backup(root, rel_path, loc, &content);
        }
        if merge.changed || page_tags_before != page_tags {
            let page = self.page_matcher.touch(&page, loc, now)?;
            if !waits {
                self.export.queue_export(&page.id);
            }
            self.page_service.update_tag_pages(
                &union(&merge.tags_before, &page_tags_before),
                &union(&merge.tags_after, &page_tags),
            );
            self.page_service.sync_atlases();
            self.events.page_changed(&page.id);
        }
        std::fs::remove_file(&file)?;
        tracing::info!("Merged and deleted {rel_path}");
        Ok(if merge.file_lost {
            Outcome::Conflict
        } else {
            Outcome::Imported
        })
    }

    /// Takes in the files that wait for the file that went, as the page moved.
    fn sync_waiting_for(&self, root: &Path, gone: &str) {
        let waiting: Vec<String> = {
            let mut waiting = self.waiting.lock();
            waiting.remove(gone);
            waiting
                .iter()
                .filter(|(_, w)| w.other == gone)
                .map(|(rel, _)| rel.clone())
                .collect()
        };
        for rel in waiting {
            self.sync_locked(root, &rel);
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
    ) -> Result<Outcome, AppError> {
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
        let now = self.file_sync_state.db().now_millis();

        let since = self.waiting.lock().get(rel_path).map(|w| w.since);
        let copy_allowed = since.unwrap_or_else(Instant::now).elapsed() >= self.copy_wait;
        let matched =
            self.page_matcher
                .match_file(root, rel_path, loc, fm, file_modified, copy_allowed)?;
        let PageMatch {
            mut page,
            is_new,
            renamed,
            write_back,
        } = match matched {
            Matched::Page(m) => {
                self.waiting.lock().remove(rel_path);
                m
            }
            Matched::Elsewhere { other } => {
                if since.is_none() {
                    tracing::info!("{rel_path} has the id of {other}, waiting for that file to go");
                }
                self.waiting.lock().insert(
                    rel_path.to_string(),
                    Wait {
                        other,
                        since: since.unwrap_or_else(Instant::now),
                    },
                );
                return Ok(Outcome::Waiting);
            }
        };

        let states = self.file_sync_state.find_by_page_id(&page.id)?;
        // A page that never had a file on this device (e.g. the root index a fresh install
        // creates) is not "newer" in any meaningful way, so the file wins
        let had_file = !states.is_empty();
        if had_file && page.is_atlas() && file_modified < page.modified_date {
            // DB is newer: keep the DB, keep the file's content as a backup, then overwrite the file.
            // A moved file still moves the page, so the export lands at the new path.
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
        let mode = match states.iter().map(|s| s.last_synced).max() {
            Some(last_synced) if had_file && !page.is_atlas() => {
                MergeMode::ItemByItem { last_synced }
            }
            _ => MergeMode::WholeFile,
        };
        let merge = self
            .item_merger
            .merge(&page, items, file_modified, mode, now)?;
        let waits = !merge.waiting_transclusions.is_empty();
        self.waiting_transclusions
            .lock()
            .extend(merge.waiting_transclusions.iter().cloned());
        // The page has nothing the file lacks when it did not change since the last sync: the
        // file is newer then, whatever its time. A file time can be coarser than the clock of
        // the page, and come out just before the last change of the page (#730)
        let last_synced = states.iter().map(|s| s.last_synced).max();
        let changed_since_sync = last_synced.is_none_or(|synced| page.modified_date > synced);
        let file_is_older = had_file && changed_since_sync && file_modified < page.modified_date;
        // A file newer than the page says which tags it has, so a file without tags has none
        let page_tags = if file_is_older {
            self.page_service.page_tags(&page.id)?
        } else {
            frontmatter_tags(fm)
        };
        let page_tags_before = self.page_service.replace_page_tags(&page.id, &page_tags)?;
        let properties_changed =
            !file_is_older && self.page_service.replace_page_properties(&page.id, fm)?;
        if merge.file_lost {
            self.write_conflict_backup(root, rel_path, loc, &content);
        }

        let page_changed = renamed
            || properties_changed
            || merge.changed
            || merge.changed_here
            || is_new
            || page_tags_before != page_tags;
        if page_changed {
            let new_modified = if is_new {
                page.modified_date
            } else if merge.changed_here {
                now
            } else if file_modified > page.modified_date {
                file_modified
            } else if file_is_older {
                page.modified_date
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

        let content_date = items
            .iter()
            .filter_map(|i| i.modified)
            .fold(file_modified, i64::max);
        self.file_sync_state.upsert_synced(
            rel_path,
            &page.id,
            hash,
            attrs.size,
            attrs.mtime,
            content_date,
        )?;
        if renamed {
            self.forget_old_paths(root, rel_path, &page)?;
        }

        if merge.restamped {
            // At once: the next edit by hand starts from this file, and with the old date of
            // the item in it, the edit would count as older than the item
            self.export.export_page(&page.id)?;
        } else if !waits
            && (write_back
                || merge.needs_write_back
                || page.modified_date > file_modified
                || !tags_written_as_exported(fm, &page_tags))
        {
            // Give the file its ids / frontmatter. A page that waits for the source of a
            // transclusion is written once the transclusion is set, or its file would lose it. The export records the new hash, so it won't echo back.
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
            self.events.page_changed(&page.id);
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
        Ok(if merge.file_lost {
            Outcome::Conflict
        } else {
            Outcome::Imported
        })
    }

    /// Drops the sync state of the files the moved page had before, when they are gone. They
    /// would count as deleted files otherwise, and the page would be offered for deletion.
    fn forget_old_paths(&self, root: &Path, rel_path: &str, page: &Page) -> Result<(), AppError> {
        for old in self.file_sync_state.find_by_page_id(&page.id)? {
            if old.rel_path != rel_path && !root.join(&old.rel_path).exists() {
                self.file_sync_state.delete_by_path(&old.rel_path)?;
            }
        }
        Ok(())
    }

    /// Keeps the content of a file next to it, as it loses to the database.
    fn write_conflict_backup(
        &self,
        root: &Path,
        rel_path: &str,
        loc: &PageLocation,
        content: &str,
    ) {
        let file = root.join(rel_path);
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        let backup = file.with_file_name(path_mapper::conflict_file_name(&loc.name, &ts));
        match std::fs::write(&backup, content) {
            Ok(_) => tracing::warn!(
                "{rel_path} lost to the database version; kept it as {}",
                backup
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            ),
            Err(e) => tracing::error!("Could not write conflict backup {}: {e}", backup.display()),
        }
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
    ) -> Result<(), AppError> {
        self.write_conflict_backup(root, rel_path, loc, content);
        // Remember this content so we don't back it up again before the export overwrites it
        // An atlas goes by the date of its page, so no date of the content is needed
        self.file_sync_state.upsert_synced(
            rel_path,
            &page.id,
            hash,
            attrs.size,
            attrs.mtime,
            attrs.mtime,
        )?;
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

/// Whether `sync_file` takes the file in: the files of the pages, their conflicted copies,
/// and `settings/properties.json` with its conflicted copies. The folder watcher hears of
/// every file; it passes on only these.
pub fn takes_in(rel_path: &str) -> bool {
    !path_mapper::is_ignored(rel_path)
        || rel_path == DEFINITIONS_FILE
        || definitions_file::is_copy_of_the_file(rel_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    // 取り込むのは、ページのファイルと、settings/properties.jsonとそのconflicted copyである。ほかの設定のファイルは取り込まない。
    #[test]
    fn the_files_of_pages_and_of_the_definitions_are_taken_in() {
        assert!(takes_in("docs/d/a.md"));
        assert!(takes_in("settings/properties.json"));
        assert!(takes_in(
            "settings/properties (PC's conflicted copy 2026-11-15).json"
        ));
        assert!(!takes_in("settings/other.json"));
        assert!(!takes_in("settings/properties (backup).json"));
    }
}
