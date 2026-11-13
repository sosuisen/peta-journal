//! Pages whose markdown file must be (re)written.

use std::sync::Arc;

use anyhow::Context;
use hmtid::Hmtid;
use rusqlite::{params, Connection};

use crate::error::AppResult;
use crate::infrastructure::persistence::db::{now_millis, Db};
use crate::infrastructure::persistence::page_repository::parse;

#[derive(Debug, Clone)]
pub struct ExportQueueRepository {
    db: Arc<Db>,
}

/// How many pages one `dequeue_all` takes.
pub const BATCH: usize = 10;

fn is_locked(e: &rusqlite::Error) -> bool {
    e.to_string().contains("database is locked")
}

impl ExportQueueRepository {
    pub fn new(db: Arc<Db>) -> ExportQueueRepository {
        ExportQueueRepository { db }
    }

    pub fn enqueue(&self, page_id: &Hmtid) -> AppResult<()> {
        let max_retries = 3;
        for attempt in 0..max_retries {
            let conn = self.db.conn()?;
            let now = now_millis();
            let result = conn.execute(
                "INSERT INTO export_queue (page_id, queued_at) VALUES (?, ?) ON CONFLICT(page_id) DO UPDATE SET queued_at = ?",
                params![page_id.as_str(), now, now],
            );
            match result {
                Ok(_) => return Ok(()),
                Err(e) if is_locked(&e) && attempt < max_retries - 1 => {
                    std::thread::sleep(std::time::Duration::from_millis(50 * (attempt as u64 + 1)));
                }
                Err(e) => {
                    return Err(anyhow::Error::new(e)
                        .context(format!("Failed to enqueue export for page: {page_id}"))
                        .into())
                }
            }
        }
        Ok(())
    }

    /// Takes a page off the queue on the given connection, as part of a larger transaction.
    pub fn remove_in(conn: &Connection, page_id: &Hmtid) -> AppResult<()> {
        conn.execute(
            "DELETE FROM export_queue WHERE page_id = ?",
            [page_id.as_str()],
        )
        .with_context(|| format!("Failed to remove export for page: {page_id}"))?;
        Ok(())
    }

    /// True when the page waits for its file to be written.
    pub fn holds(&self, page_id: &Hmtid) -> AppResult<bool> {
        let conn = self.db.conn()?;
        let pages: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM export_queue WHERE page_id = ?",
                [page_id.as_str()],
                |row| row.get(0),
            )
            .with_context(|| format!("Failed to look for the export of page: {page_id}"))?;
        Ok(pages > 0)
    }

    /// How many pages wait.
    pub fn waiting(&self) -> AppResult<usize> {
        let conn = self.db.conn()?;
        let pages: i64 = conn
            .query_row("SELECT COUNT(*) FROM export_queue", [], |row| row.get(0))
            .context("Failed to count the queued exports")?;
        Ok(usize::try_from(pages).unwrap_or(0))
    }

    /// Takes up to `BATCH` queued pages, oldest first, and removes them from the queue.
    pub fn dequeue_all(&self) -> AppResult<Vec<Hmtid>> {
        let max_retries = 3;
        for attempt in 0..max_retries {
            match self.try_dequeue() {
                Ok(ids) => return Ok(ids),
                Err(e) if is_locked(&e) && attempt < max_retries - 1 => {
                    std::thread::sleep(std::time::Duration::from_millis(
                        100 * (attempt as u64 + 1),
                    ));
                }
                Err(e) => {
                    return Err(anyhow::Error::new(e)
                        .context("Failed to dequeue exports")
                        .into())
                }
            }
        }
        Ok(Vec::new())
    }

    fn try_dequeue(&self) -> rusqlite::Result<Vec<Hmtid>> {
        let mut conn = self
            .db
            .conn()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
        let tx = conn.immediate()?;
        let ids: Vec<String> = {
            let mut stmt =
                tx.prepare("SELECT page_id FROM export_queue ORDER BY queued_at LIMIT ?")?;
            let rows = stmt.query_map([BATCH as i64], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };
        if !ids.is_empty() {
            let mut del = tx.prepare("DELETE FROM export_queue WHERE page_id = ?")?;
            for id in &ids {
                del.execute([id])?;
            }
        }
        tx.commit()?;
        ids.iter().map(|s| parse(s)).collect()
    }
}
