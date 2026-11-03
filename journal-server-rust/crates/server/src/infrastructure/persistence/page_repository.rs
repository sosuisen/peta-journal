//! Page persistence.

use std::sync::Arc;

use anyhow::Context;
use chrono::NaiveDate;
use hmtid::Hmtid;
use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::domain::model::Page;
use crate::error::AppResult;

use super::db::Db;

/// The columns `map_page` reads, in the order of the pages table.
const SELECT_PAGES: &str =
    "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages ";

#[derive(Debug, Clone)]
pub struct PageRepository {
    db: Arc<Db>,
}

fn map_page(row: &Row<'_>) -> rusqlite::Result<Page> {
    let id: String = row.get(0)?;
    let parent: Option<String> = row.get(1)?;
    Ok(Page {
        id: parse(&id)?,
        parent_id: parent.as_deref().map(parse).transpose()?,
        dir_path: row.get(2)?,
        name: row.get(3)?,
        page_type: row.get(4)?,
        created_date: row.get(5)?,
        modified_date: row.get(6)?,
    })
}

/// Parses a stored id; a bad value in the database is reported as a query error.
pub(crate) fn parse(s: &str) -> rusqlite::Result<Hmtid> {
    Hmtid::parse(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

impl PageRepository {
    pub fn new(db: Arc<Db>) -> PageRepository {
        PageRepository { db }
    }

    fn query_pages(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> AppResult<Vec<Page>> {
        query_pages_in(&*self.db.conn()?, sql, params)
    }

    fn first_page(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> AppResult<Option<Page>> {
        Ok(self.query_pages(sql, params)?.into_iter().next())
    }

    pub fn find_by_id(&self, id: &Hmtid) -> AppResult<Option<Page>> {
        Self::find_by_id_in(&*self.db.conn()?, id)
    }

    pub fn find_by_id_in(conn: &Connection, id: &Hmtid) -> AppResult<Option<Page>> {
        Ok(query_pages_in(
            conn,
            &format!("{SELECT_PAGES}WHERE id = ?"),
            &[&id.as_str()],
        )?
        .into_iter()
        .next())
    }

    pub fn find_journal(&self, date: NaiveDate) -> AppResult<Option<Page>> {
        let file_name = date.format("%Y-%m-%d").to_string();
        self.first_page(
            &format!("{SELECT_PAGES}WHERE type = 'journal' AND dir_path = '' AND name = ?"),
            &[&file_name],
        )
    }

    pub fn find_most_recent_journal_before(&self, before: NaiveDate) -> AppResult<Option<Page>> {
        let file_name = before.format("%Y-%m-%d").to_string();
        self.first_page(
            &format!(
                "{SELECT_PAGES}WHERE type = 'journal' AND dir_path = '' AND name < ? AND name GLOB '????-??-??' \
                 ORDER BY name DESC LIMIT 1"
            ),
            &[&file_name],
        )
    }

    pub fn find_most_recent_weekly_journal_before(
        &self,
        before_week: &str,
    ) -> AppResult<Option<Page>> {
        self.first_page(
            &format!(
                "{SELECT_PAGES}WHERE type = 'journal' AND dir_path = '' AND name < ? AND name GLOB '????-W??' \
                 ORDER BY name DESC LIMIT 1"
            ),
            &[&before_week],
        )
    }

    pub fn find_by_type_dir_path_file_name(
        &self,
        page_type: &str,
        dir_path: &str,
        file_name: &str,
    ) -> AppResult<Option<Page>> {
        self.first_page(
            &format!("{SELECT_PAGES}WHERE type = ? AND dir_path = ? AND name = ?"),
            &[&page_type, &dir_path, &file_name],
        )
    }

    /// The pages at a path, ignoring the case of the letters, oldest first. Windows and
    /// macOS store `Foo.md` and `foo.md` as one file, so two such pages would overwrite
    /// each other's file.
    pub fn find_by_type_dir_path_file_name_ignoring_case(
        &self,
        page_type: &str,
        dir_path: &str,
        file_name: &str,
    ) -> AppResult<Vec<Page>> {
        self.query_pages(
            &format!(
                "{SELECT_PAGES}WHERE type = ? AND dir_path = ? COLLATE NOCASE AND name = ? COLLATE NOCASE ORDER BY id"
            ),
            &[&page_type, &dir_path, &file_name],
        )
    }

    /// Recently modified pages, newest first. `page_type` None means all types.
    pub fn find_recent(&self, max_results: i64, page_type: Option<&str>) -> AppResult<Vec<Page>> {
        let limit = if max_results > 0 { max_results } else { 10 };
        match page_type {
            Some(t) => self.query_pages(
                &format!("{SELECT_PAGES}WHERE type = ? ORDER BY modified_date DESC LIMIT ?"),
                &[&t, &limit],
            ),
            None => self.query_pages(
                &format!("{SELECT_PAGES}ORDER BY modified_date DESC LIMIT ?"),
                &[&limit],
            ),
        }
    }

    pub fn find_all(&self) -> AppResult<Vec<Page>> {
        self.query_pages(&format!("{SELECT_PAGES}ORDER BY modified_date DESC"), &[])
    }

    /// Marks a page as changed, on the given connection, as part of a larger transaction.
    pub fn touch_in(conn: &Connection, page_id: &Hmtid, modified_date: i64) -> AppResult<()> {
        conn.execute(
            "UPDATE pages SET modified_date = ? WHERE id = ?",
            params![modified_date, page_id.as_str()],
        )?;
        Ok(())
    }

    /// Inserts or updates a page. The parent is resolved from `dir_path` and `name`.
    pub fn save(&self, page: &Page) -> AppResult<Page> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        let parent_id = resolve_parent_id(&tx, &page.id, &page.dir_path, &page.name)?;
        let parent_str = parent_id.as_ref().map(|p| p.to_string());
        tx.execute(
            "INSERT INTO pages (id, parent_id, dir_path, name, type, created_date, modified_date) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET parent_id=?, dir_path=?, name=?, type=?, created_date=?, modified_date=?",
            params![
                page.id.as_str(),
                parent_str,
                page.dir_path,
                page.name,
                page.page_type,
                page.created_date,
                page.modified_date,
                parent_str,
                page.dir_path,
                page.name,
                page.page_type,
                page.created_date,
                page.modified_date,
            ],
        )
        .with_context(|| format!("Failed to save page: {}", page.id))?;
        tx.commit()?;
        Ok(Page {
            parent_id,
            ..page.clone()
        })
    }

    /// Changes a page's id, moving its items and related rows along.
    pub fn rekey(&self, old_id: &Hmtid, new_id: &Hmtid) -> AppResult<()> {
        let updates = [
            "UPDATE pages SET id = ? WHERE id = ?",
            "UPDATE pages SET parent_id = ? WHERE parent_id = ?",
            "UPDATE items SET page_id = ? WHERE page_id = ?",
            "UPDATE export_queue SET page_id = ? WHERE page_id = ?",
            "UPDATE file_sync_state SET page_id = ? WHERE page_id = ?",
            "UPDATE properties SET target_id = ? WHERE target_type = 'page' AND target_id = ?",
            "UPDATE property_list_items SET target_id = ? WHERE target_type = 'page' AND target_id = ?",
            // The card of the page on the atlas of its directory names it by id
            "UPDATE properties SET value_text = ? WHERE target_type = 'item' AND key = 'atlas_page' AND value_text = ?",
        ];
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        // Rows reference each other by page id; check constraints at commit, not per statement
        tx.execute_batch("PRAGMA defer_foreign_keys = ON")?;
        for sql in updates {
            tx.execute(sql, params![new_id.as_str(), old_id.as_str()])?;
        }
        tx.commit()
            .with_context(|| format!("Failed to rekey page {old_id} -> {new_id}"))?;
        Ok(())
    }

    pub fn delete(&self, id: &Hmtid) -> AppResult<()> {
        let conn = self.db.conn()?;
        conn.execute("DELETE FROM pages WHERE id = ?", [id.as_str()])?;
        Ok(())
    }

    pub fn find_by_path_prefix(&self, query: &str, max_results: i64) -> AppResult<Vec<Page>> {
        let limit = if max_results > 0 { max_results } else { 20 };
        self.query_pages(
            &format!(
                "{SELECT_PAGES}WHERE (dir_path || name) LIKE ? || '%' ORDER BY modified_date DESC LIMIT ?"
            ),
            &[&query, &limit],
        )
    }

    pub fn find_by_path_contains(&self, query: &str, max_results: i64) -> AppResult<Vec<Page>> {
        let limit = if max_results > 0 { max_results } else { 20 };
        self.query_pages(
            &format!(
                "{SELECT_PAGES}WHERE (dir_path || name) LIKE '%' || ? || '%' ORDER BY modified_date DESC LIMIT ?"
            ),
            &[&query, &limit],
        )
    }

    /// Index pages whose last dir_path segment matches the query.
    pub fn find_index_pages_by_dir_segment(
        &self,
        query: &str,
        max_results: i64,
    ) -> AppResult<Vec<Page>> {
        let limit = if max_results > 0 { max_results } else { 20 };
        self.query_pages(
            &format!(
                "{SELECT_PAGES}WHERE name = 'index' AND \
                 (dir_path = ? || '/' OR dir_path LIKE '%/' || ? || '/') \
                 ORDER BY length(dir_path) ASC LIMIT ?"
            ),
            &[&query, &query, &limit],
        )
    }

    pub fn find_by_parent_id(&self, parent_id: &Hmtid) -> AppResult<Vec<Page>> {
        self.query_pages(
            &format!("{SELECT_PAGES}WHERE parent_id = ? ORDER BY name"),
            &[&parent_id.as_str()],
        )
    }
}

/// Resolves the parent page ID based on dir_path and name.
fn resolve_parent_id(
    conn: &Connection,
    page_id: &Hmtid,
    dir_path: &str,
    name: &str,
) -> rusqlite::Result<Option<Hmtid>> {
    if name == "index" {
        // Index page's parent is the index page one level up
        if dir_path.is_empty() {
            return Ok(None);
        }
        let trimmed = dir_path.strip_suffix('/').unwrap_or(dir_path);
        let parent_dir = match trimmed.rfind('/') {
            Some(pos) => &trimmed[..=pos],
            None => "",
        };
        find_page_id(conn, parent_dir, "index")
    } else {
        // Non-index page's parent is the index page at the same dirPath
        let found = find_page_id(conn, dir_path, "index")?;
        Ok(match found {
            Some(ref id) if id == page_id => None,
            other => other,
        })
    }
}

/// Runs a query whose result rows are pages in the shape of `SELECT_PAGES`.
fn query_pages_in(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> AppResult<Vec<Page>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, map_page)?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn find_page_id(conn: &Connection, dir_path: &str, name: &str) -> rusqlite::Result<Option<Hmtid>> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM pages WHERE dir_path = ? AND name = ? LIMIT 1",
            params![dir_path, name],
            |r| r.get(0),
        )
        .optional()?;
    id.as_deref().map(parse).transpose()
}
