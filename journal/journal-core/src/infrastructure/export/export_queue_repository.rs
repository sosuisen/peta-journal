//! Pages whose markdown file must be (re)written.

use std::sync::Arc;

use anyhow::Context;
use hmtid::Hmtid;
use rusqlite::params;

use crate::error::AppError;
use crate::infrastructure::persistence::db::Db;
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

    pub fn enqueue(&self, page_id: &Hmtid) -> Result<(), AppError> {
        let max_retries = 3;
        for attempt in 0..max_retries {
            let conn = self.db.conn()?;
            let now = self.db.now_millis();
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

    /// True when the page waits for its file to be written.
    pub fn holds(&self, page_id: &Hmtid) -> Result<bool, AppError> {
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
    pub fn waiting(&self) -> Result<usize, AppError> {
        let conn = self.db.conn()?;
        let pages: i64 = conn
            .query_row("SELECT COUNT(*) FROM export_queue", [], |row| row.get(0))
            .context("Failed to count the queued exports")?;
        Ok(usize::try_from(pages).unwrap_or(0))
    }

    /// When a page was last put in the queue, in milliseconds; None when no page waits. A page
    /// that waits and is put in again moves it on: the app knows from it that a page changed
    /// (ADR 055).
    pub fn last_queued_at(&self) -> Result<Option<i64>, AppError> {
        let conn = self.db.conn()?;
        let last = conn
            .query_row("SELECT MAX(queued_at) FROM export_queue", [], |row| {
                row.get(0)
            })
            .context("Failed to read when a page was last queued")?;
        Ok(last)
    }

    /// Takes up to `BATCH` queued pages, oldest first, and removes them from the queue.
    pub fn dequeue_all(&self) -> Result<Vec<Hmtid>, AppError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::{Clock, SteppedClock, SystemClock};
    use crate::domain::id_generator::IdGenerator;
    use crate::infrastructure::persistence::database_initializer::initialize;

    /// A queue on a new database, and a page that may be put in it.
    fn queue() -> (tempfile::TempDir, ExportQueueRepository, Hmtid) {
        queue_dated_by(Arc::new(SystemClock))
    }

    /// The same, with the clock that dates what is put in.
    fn queue_dated_by(clock: Arc<dyn Clock>) -> (tempfile::TempDir, ExportQueueRepository, Hmtid) {
        let dir = tempfile::TempDir::new().unwrap();
        let db = Arc::new(Db::open(&dir.path().join("journal.db"), clock).unwrap());
        let ids = IdGenerator::default();
        initialize(&db, &ids).unwrap();
        let page = ids.generate_page_id();
        db.conn()
            .unwrap()
            .execute(
                "INSERT INTO pages (id, dir_path, name, created_date, modified_date) \
                 VALUES (?, '', 'notes', 0, 0)",
                params![page.as_str()],
            )
            .unwrap();
        (dir, ExportQueueRepository::new(db), page)
    }

    // 待っているページがなければ、最後に待ち行列に入れた時刻はない。
    #[test]
    fn an_empty_queue_has_no_time_of_the_last_page_put_in() {
        let (_dir, queue, _page) = queue();
        assert_eq!(queue.last_queued_at().unwrap(), None);
    }

    // 待っているページをもう一度入れると、最後に入れた時刻が進む。ページが変わったことを、アプリの同期が知るためである（ADR 055）。
    #[test]
    fn a_page_put_in_again_moves_the_time_of_the_last_page_put_in_on() {
        let clock = Arc::new(SteppedClock::default());
        let (_dir, queue, page) = queue_dated_by(clock.clone());
        queue.enqueue(&page).unwrap();
        let first = queue.last_queued_at().unwrap().unwrap();
        clock.advance(5);
        queue.enqueue(&page).unwrap();
        assert!(queue.last_queued_at().unwrap().unwrap() > first);
        assert_eq!(queue.waiting().unwrap(), 1);
    }
}
