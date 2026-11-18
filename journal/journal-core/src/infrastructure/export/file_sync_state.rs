//! What we last knew about each markdown file in the export directory, so that the folder -> DB
//! sync can tell our own writes from external edits.

use std::sync::Arc;

use anyhow::Context;
use hmtid::Hmtid;
use rusqlite::{params, OptionalExtension, Row};

use crate::error::AppError;
use crate::infrastructure::persistence::db::{now_millis, Db};
use crate::infrastructure::persistence::page_repository::parse;

pub const SYNCED: &str = "synced";
pub const MISSING: &str = "missing";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSyncState {
    /// Path relative to the markdown directory, using '/' as separator.
    pub rel_path: String,
    pub page_id: Hmtid,
    /// SHA-256 of the file content as last read or written by us.
    pub content_hash: String,
    pub file_size: i64,
    /// File mtime (epoch millis) at that time.
    pub file_mtime: i64,
    /// Until when the page and the file were in step: the later of the file mtime and the
    /// latest date the content holds. The clock of the file system can be behind the clock
    /// that dates the items, so the mtime alone may be before a date written in the file.
    /// A date of the content after the time of the sync counts as that time.
    pub last_synced: i64,
    pub synced_at: i64,
    /// `synced` or `missing`.
    pub status: String,
    /// When the file was first seen missing (epoch millis).
    pub missing_since: Option<i64>,
}

impl FileSyncState {
    pub fn is_missing(&self) -> bool {
        self.status == MISSING
    }
}

const COLUMNS: &str = "rel_path, page_id, content_hash, file_size, file_mtime, last_synced, \
    synced_at, status, missing_since";

fn map(row: &Row<'_>) -> rusqlite::Result<FileSyncState> {
    let page_id: String = row.get(1)?;
    Ok(FileSyncState {
        rel_path: row.get(0)?,
        page_id: parse(&page_id)?,
        content_hash: row.get(2)?,
        file_size: row.get(3)?,
        file_mtime: row.get(4)?,
        last_synced: row.get(5)?,
        synced_at: row.get(6)?,
        status: row.get(7)?,
        missing_since: row.get(8)?,
    })
}

/// Until when a file and its page were in step; see `FileSyncState::last_synced`.
///
/// Why not the date of the content as it is: the clock of another device may be ahead, and
/// nothing was seen here after now. An item made here before the date of that device would
/// count as seen, and would go with the next file that does not hold it.
fn last_synced(file_mtime: i64, content_date: i64, now: i64) -> i64 {
    file_mtime.max(content_date.min(now))
}

#[derive(Debug, Clone)]
pub struct FileSyncStateRepository {
    db: Arc<Db>,
}

impl FileSyncStateRepository {
    pub fn new(db: Arc<Db>) -> FileSyncStateRepository {
        FileSyncStateRepository { db }
    }

    /// Records that the file at rel_path now matches the given content (read or written by us).
    /// `content_date` is the latest date the content holds, of the page or of an item.
    pub fn upsert_synced(
        &self,
        rel_path: &str,
        page_id: &Hmtid,
        content_hash: &str,
        file_size: i64,
        file_mtime: i64,
        content_date: i64,
    ) -> Result<(), AppError> {
        let now = now_millis();
        let last_synced = last_synced(file_mtime, content_date, now);
        let conn = self.db.conn()?;
        conn.execute(
            &format!(
                "INSERT INTO file_sync_state ({COLUMNS}) VALUES (?, ?, ?, ?, ?, ?, ?, 'synced', NULL) \
                 ON CONFLICT(rel_path) DO UPDATE SET page_id=?, content_hash=?, file_size=?, file_mtime=?, \
                 last_synced=?, synced_at=?, status='synced', missing_since=NULL"
            ),
            params![
                rel_path,
                page_id.as_str(),
                content_hash,
                file_size,
                file_mtime,
                last_synced,
                now,
                page_id.as_str(),
                content_hash,
                file_size,
                file_mtime,
                last_synced,
                now
            ],
        )
        .with_context(|| format!("Failed to upsert file sync state: {rel_path}"))?;
        Ok(())
    }

    /// Marks the file as missing. Keeps the original missing_since if already missing.
    pub fn mark_missing(&self, rel_path: &str) -> Result<(), AppError> {
        let now = now_millis();
        let conn = self.db.conn()?;
        conn.execute(
            "UPDATE file_sync_state SET status='missing', synced_at=?, \
             missing_since=COALESCE(missing_since, ?) WHERE rel_path = ?",
            params![now, now, rel_path],
        )?;
        Ok(())
    }

    pub fn find_by_path(&self, rel_path: &str) -> Result<Option<FileSyncState>, AppError> {
        let conn = self.db.conn()?;
        Ok(conn
            .query_row(
                &format!("SELECT {COLUMNS} FROM file_sync_state WHERE rel_path = ?"),
                [rel_path],
                map,
            )
            .optional()?)
    }

    fn query(
        &self,
        sql: &str,
        params: &[&dyn rusqlite::ToSql],
    ) -> Result<Vec<FileSyncState>, AppError> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, map)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn find_by_page_id(&self, page_id: &Hmtid) -> Result<Vec<FileSyncState>, AppError> {
        self.query(
            &format!("SELECT {COLUMNS} FROM file_sync_state WHERE page_id = ?"),
            &[&page_id.as_str()],
        )
    }

    pub fn find_all(&self) -> Result<Vec<FileSyncState>, AppError> {
        self.query(&format!("SELECT {COLUMNS} FROM file_sync_state"), &[])
    }

    pub fn find_missing(&self) -> Result<Vec<FileSyncState>, AppError> {
        self.query(
            &format!("SELECT {COLUMNS} FROM file_sync_state WHERE status = 'missing' ORDER BY missing_since"),
            &[],
        )
    }

    pub fn delete_by_path(&self, rel_path: &str) -> Result<(), AppError> {
        let conn = self.db.conn()?;
        conn.execute("DELETE FROM file_sync_state WHERE rel_path = ?", [rel_path])?;
        Ok(())
    }

    pub fn delete_by_page_id(&self, page_id: &Hmtid) -> Result<(), AppError> {
        let conn = self.db.conn()?;
        conn.execute(
            "DELETE FROM file_sync_state WHERE page_id = ?",
            [page_id.as_str()],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 中の日付がファイルの更新時刻より遅ければ、前回の同期は中の日付になる。
    #[test]
    fn the_last_sync_is_the_date_of_the_content_when_the_mtime_is_behind_it() {
        assert_eq!(last_synced(10, 20, 30), 20);
    }

    // ファイルの更新時刻が中の日付より遅ければ、前回の同期は更新時刻になる。
    #[test]
    fn the_last_sync_is_the_mtime_when_it_is_after_the_date_of_the_content() {
        assert_eq!(last_synced(20, 10, 30), 20);
    }

    // 今より先の中の日付は、今として数える。
    #[test]
    fn a_date_of_the_content_after_now_counts_as_now() {
        assert_eq!(last_synced(10, 99, 30), 30);
    }
}
