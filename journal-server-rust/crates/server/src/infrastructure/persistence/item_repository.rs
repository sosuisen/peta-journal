//! Item persistence.

use std::sync::Arc;

use anyhow::Context;
use hmtid::Hmtid;
use rusqlite::{params, Connection, Row};

use crate::domain::model::{Item, ItemPatch, ItemPlacement};
use crate::error::{AppError, AppResult};

use super::db::{placeholders, Db};
use super::page_repository::parse;

/// Not in the index alphabet, so a parked index never equals a real one.
const PARKED_IDX_PREFIX: &str = "~";

/// An index no real item holds, for an item that is to be arranged.
pub fn parked_idx(id: &Hmtid) -> String {
    format!("{PARKED_IDX_PREFIX}{id}")
}

/// The item columns `map_item` reads, joined with the page columns an item carries.
const SELECT_ITEMS: &str =
    "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, \
     p.dir_path, p.name, p.type \
     FROM items b JOIN pages p ON b.page_id = p.id ";

/// An item and all its descendants, by a recursive query on parent_id, in page order.
const SELECT_ITEM_WITH_DESCENDANTS: &str = "WITH RECURSIVE descendants AS (\
       SELECT i.* FROM items i WHERE i.id = ? \
       UNION ALL \
       SELECT i.* FROM items i JOIN descendants d ON i.parent_id = d.id\
     ) \
     SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, \
     p.dir_path, p.name, p.type \
     FROM descendants b JOIN pages p ON b.page_id = p.id ORDER BY b.idx";

#[derive(Debug, Clone)]
pub struct ItemRepository {
    db: Arc<Db>,
}

fn map_item(conn: &Connection, row: &Row<'_>) -> rusqlite::Result<Item> {
    let id: String = row.get(0)?;
    let page_id: String = row.get(1)?;
    let parent: Option<String> = row.get(2)?;
    let transclude: Option<String> = row.get(10)?;
    let collapsed: i64 = row.get(9)?;
    Ok(Item {
        id: parse(&id)?,
        page_id: parse(&page_id)?,
        parent_item_id: parent.as_deref().map(parse).transpose()?,
        dir_path: row.get(11)?,
        name: row.get(12)?,
        item_type: row.get(13)?,
        created_date: row.get(3)?,
        modified_date: row.get(4)?,
        idx: row.get(5)?,
        level: row.get(6)?,
        body: row.get(7)?,
        ast: row.get(8)?,
        collapsed: collapsed != 0,
        types: load_types(conn, &id)?,
        transclude_item_id: transclude.as_deref().map(parse).transpose()?,
    })
}

fn load_types(conn: &Connection, item_id: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare_cached("SELECT type FROM item_types WHERE item_id = ?")?;
    let rows = stmt.query_map([item_id], |r| r.get::<_, String>(0))?;
    rows.collect()
}

fn delete_types(conn: &Connection, item_id: &str) -> rusqlite::Result<()> {
    conn.execute("DELETE FROM item_types WHERE item_id = ?", [item_id])?;
    Ok(())
}

fn insert_type(conn: &Connection, id: &Hmtid, item_type: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO item_types (item_id, type) VALUES (?, ?)",
        params![id.as_str(), item_type],
    )?;
    Ok(())
}

impl ItemRepository {
    pub fn new(db: Arc<Db>) -> ItemRepository {
        ItemRepository { db }
    }

    /// The database, for a service that runs several repository calls in one transaction.
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// Runs a query whose result rows are items in the shape of `SELECT_ITEMS`.
    fn query_items(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> AppResult<Vec<Item>> {
        query_items_in(&*self.db.conn()?, sql, params)
    }

    pub fn find_by_id(&self, id: &Hmtid) -> AppResult<Option<Item>> {
        Self::find_by_id_in(&*self.db.conn()?, id)
    }

    pub fn find_by_id_in(conn: &Connection, id: &Hmtid) -> AppResult<Option<Item>> {
        Ok(query_items_in(
            conn,
            &format!("{SELECT_ITEMS}WHERE b.id = ?"),
            &[&id.as_str()],
        )?
        .into_iter()
        .next())
    }

    pub fn find_by_page_id(&self, page_id: &Hmtid) -> AppResult<Vec<Item>> {
        Self::find_by_page_id_in(&*self.db.conn()?, page_id)
    }

    pub fn find_by_page_id_in(conn: &Connection, page_id: &Hmtid) -> AppResult<Vec<Item>> {
        query_items_in(
            conn,
            &format!("{SELECT_ITEMS}WHERE b.page_id = ? ORDER BY b.idx"),
            &[&page_id.as_str()],
        )
    }

    /// The items whose body may hold a wikilink to the path: `[[path]]`, `[[path#id]]` or
    /// `[[path|label]]`. LIKE ignores the case of ASCII letters, as page paths are matched.
    /// The caller checks the parsed links, since `[[path/more]]` matches too.
    pub fn find_by_body_containing_link_to(&self, path: &str) -> AppResult<Vec<Item>> {
        let escaped = path
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_");
        let pattern = format!("%[[{escaped}%");
        self.query_items(
            &format!("{SELECT_ITEMS}WHERE b.body LIKE ? ESCAPE '\\'"),
            &[&pattern],
        )
    }

    pub fn find_by_types(&self, types: &[String]) -> AppResult<Vec<Item>> {
        if types.is_empty() {
            return Ok(Vec::new());
        }
        let sql = format!(
            "{SELECT_ITEMS}WHERE b.id IN (SELECT item_id FROM item_types WHERE type IN ({})) \
             ORDER BY b.created_date DESC",
            placeholders(types.len())
        );
        let params: Vec<&dyn rusqlite::ToSql> =
            types.iter().map(|t| t as &dyn rusqlite::ToSql).collect();
        self.query_items(&sql, &params)
    }

    /// All items that transclude the given source item.
    pub fn find_transcluders(&self, source_item_id: &Hmtid) -> AppResult<Vec<Item>> {
        Self::find_transcluders_in(&*self.db.conn()?, source_item_id)
    }

    pub fn find_transcluders_in(conn: &Connection, source_item_id: &Hmtid) -> AppResult<Vec<Item>> {
        query_items_in(
            conn,
            &format!("{SELECT_ITEMS}WHERE b.transclude_item_id = ?"),
            &[&source_item_id.as_str()],
        )
    }

    /// An item and all its descendants, using a recursive query on parent_id.
    pub fn find_item_with_descendants(&self, item_id: &Hmtid) -> AppResult<Vec<Item>> {
        Self::find_item_with_descendants_in(&*self.db.conn()?, item_id)
    }

    pub fn find_item_with_descendants_in(
        conn: &Connection,
        item_id: &Hmtid,
    ) -> AppResult<Vec<Item>> {
        query_items_in(conn, SELECT_ITEM_WITH_DESCENDANTS, &[&item_id.as_str()])
    }

    pub fn save(&self, item: &Item) -> AppResult<Item> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        Self::save_in(&tx, item)?;
        tx.commit()?;
        Ok(item.clone())
    }

    /// Inserts or updates an item on the given connection, as part of a larger transaction.
    pub fn save_in(tx: &Connection, item: &Item) -> AppResult<()> {
        let parent = item.parent_item_id.as_ref().map(|p| p.to_string());
        let transclude = item.transclude_item_id.as_ref().map(|p| p.to_string());
        tx.execute(
            "INSERT INTO items (id, page_id, parent_id, created_date, modified_date, idx, level, body, ast, collapsed, transclude_item_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET page_id=?, parent_id=?, modified_date=?, idx=?, level=?, body=?, ast=?, collapsed=?, transclude_item_id=?",
            params![
                item.id.as_str(),
                item.page_id.as_str(),
                parent,
                item.created_date,
                item.modified_date,
                item.idx,
                item.level,
                item.body,
                item.ast,
                item.collapsed as i64,
                transclude,
                item.page_id.as_str(),
                parent,
                item.modified_date,
                item.idx,
                item.level,
                item.body,
                item.ast,
                item.collapsed as i64,
                transclude,
            ],
        )
        .with_context(|| format!("Failed to save item: {}", item.id))?;
        delete_types(tx, item.id.as_str())?;
        for t in &item.types {
            insert_type(tx, &item.id, t)?;
        }
        Ok(())
    }

    /// Writes only the fields set in the patch, on the given connection, as part of a larger
    /// transaction. Returns whether the item exists.
    pub fn patch_in(tx: &Connection, id: &Hmtid, patch: &ItemPatch) -> AppResult<bool> {
        let found = patch_columns(tx, id, patch)?;
        if found {
            patch_types(tx, id, patch)?;
        }
        Ok(found)
    }

    /// Adds delta to the level of every given item, on the given connection, as part of a
    /// larger transaction. A level never goes below 1.
    pub fn shift_levels_in(
        tx: &Connection,
        ids: &[Hmtid],
        delta: i32,
        modified_date: i64,
    ) -> AppResult<()> {
        let mut stmt = tx.prepare(
            "UPDATE items SET level = MAX(1, level + ?), modified_date = ? WHERE id = ?",
        )?;
        for id in ids {
            stmt.execute(params![delta, modified_date, id.as_str()])?;
        }
        Ok(())
    }

    /// Moves items to their new places on a page, on the given connection, as part of a larger
    /// transaction.
    pub fn move_items_in(
        tx: &Connection,
        target_page_id: &Hmtid,
        placements: &[ItemPlacement],
        level_delta: i32,
        modified_date: i64,
    ) -> AppResult<()> {
        // (page_id, idx) is unique and checked per statement. A moving item may take the index
        // that another moving item still holds, so all of them leave their index first.
        {
            let mut stmt = tx.prepare("UPDATE items SET idx = ? WHERE id = ?")?;
            for p in placements {
                let n = stmt.execute(params![parked_idx(&p.item_id), p.item_id.as_str()])?;
                if n == 0 {
                    return Err(AppError::bad_request(format!(
                        "Item not found: {}",
                        p.item_id
                    )));
                }
            }
        }
        {
            let mut stmt = tx.prepare(
                "UPDATE items SET page_id = ?, parent_id = NULL, idx = ?, level = MAX(1, level + ?), \
                 modified_date = ? WHERE id = ?",
            )?;
            for p in placements {
                stmt.execute(params![
                    target_page_id.as_str(),
                    p.idx,
                    level_delta,
                    modified_date,
                    p.item_id.as_str()
                ])
                .with_context(|| {
                    format!(
                        "Failed to move item {} to page: {target_page_id}",
                        p.item_id
                    )
                })?;
            }
        }
        Ok(())
    }

    /// Gives the items of a page new indices and levels, in one transaction. The items not
    /// named keep theirs. A new item may be saved first with [`parked_idx`], as no other item
    /// can hold it.
    pub fn arrange(&self, page_id: &Hmtid, rows: &[(Hmtid, String, i32)]) -> AppResult<()> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        Self::arrange_in(&tx, page_id, rows)?;
        tx.commit()
            .with_context(|| format!("Failed to arrange the items of page: {page_id}"))?;
        Ok(())
    }

    /// `arrange` on the given connection, as part of a larger transaction.
    pub fn arrange_in(
        tx: &Connection,
        page_id: &Hmtid,
        rows: &[(Hmtid, String, i32)],
    ) -> AppResult<()> {
        // (page_id, idx) is unique and checked per statement, as in `move_items`
        {
            let mut stmt = tx.prepare("UPDATE items SET idx = ? WHERE id = ?")?;
            for (id, _, _) in rows {
                stmt.execute(params![parked_idx(id), id.as_str()])?;
            }
        }
        {
            let mut stmt = tx.prepare(
                "UPDATE items SET idx = ?, level = ?, parent_id = NULL WHERE id = ? AND page_id = ?",
            )?;
            for (id, idx, level) in rows {
                stmt.execute(params![idx, level, id.as_str(), page_id.as_str()])?;
            }
        }
        Ok(())
    }

    pub fn delete(&self, id: &Hmtid) -> AppResult<()> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        Self::delete_in(&tx, id)?;
        tx.commit()?;
        Ok(())
    }

    /// Deletes an item on the given connection, as part of a larger transaction.
    pub fn delete_in(tx: &Connection, id: &Hmtid) -> AppResult<()> {
        // Foreign keys are not enforced on these connections, so the types are removed by hand
        delete_types(tx, id.as_str())?;
        tx.execute("DELETE FROM items WHERE id = ?", [id.as_str()])?;
        Ok(())
    }

    pub fn delete_by_page_id(&self, page_id: &Hmtid) -> AppResult<()> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        Self::delete_by_page_id_in(&tx, page_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Deletes every item of a page on the given connection, as part of a larger transaction.
    pub fn delete_by_page_id_in(tx: &Connection, page_id: &Hmtid) -> AppResult<()> {
        tx.execute(
            "DELETE FROM item_types WHERE item_id IN (SELECT id FROM items WHERE page_id = ?)",
            [page_id.as_str()],
        )?;
        tx.execute("DELETE FROM items WHERE page_id = ?", [page_id.as_str()])?;
        Ok(())
    }

    /// Gives an item a new index on the given connection, as part of a larger transaction.
    pub fn update_index_in(conn: &Connection, item_id: &Hmtid, new_idx: &str) -> AppResult<()> {
        conn.execute(
            "UPDATE items SET idx = ? WHERE id = ?",
            params![new_idx, item_id.as_str()],
        )?;
        Ok(())
    }

    /// Recalculates parent_id for all items on a page based on idx ordering and level.
    pub fn recalculate_parent_ids(&self, page_id: &Hmtid) -> AppResult<()> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        Self::recalculate_parent_ids_in(&tx, page_id)?;
        tx.commit()?;
        Ok(())
    }

    /// Recalculates the parents of a page's items on the given connection, as part of a
    /// larger transaction.
    pub fn recalculate_parent_ids_in(tx: &Connection, page_id: &Hmtid) -> AppResult<()> {
        let items: Vec<(String, i32)> = {
            let mut stmt =
                tx.prepare("SELECT id, level FROM items WHERE page_id = ? ORDER BY idx")?;
            let rows = stmt.query_map([page_id.as_str()], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<Result<_, _>>()?
        };
        {
            let mut update = tx.prepare("UPDATE items SET parent_id = ? WHERE id = ?")?;
            // The parent of an item is the nearest item above it with a smaller level
            for i in 0..items.len() {
                let level = items[i].1;
                let mut parent: Option<&str> = None;
                if level > 1 {
                    for j in (0..i).rev() {
                        if items[j].1 < level {
                            parent = Some(&items[j].0);
                            break;
                        }
                    }
                }
                update.execute(params![parent, items[i].0])?;
            }
        }
        Ok(())
    }
}

/// Runs a query whose result rows are items in the shape of `SELECT_ITEMS`.
fn query_items_in(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> AppResult<Vec<Item>> {
    let mut stmt = conn.prepare(sql)?;
    let mut rows = stmt.query(params)?;
    let mut items = Vec::new();
    while let Some(row) = rows.next()? {
        items.push(map_item(conn, row)?);
    }
    Ok(items)
}

fn patch_columns(conn: &Connection, id: &Hmtid, patch: &ItemPatch) -> rusqlite::Result<bool> {
    let mut sql = String::from("UPDATE items SET modified_date = ?");
    let mut values: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(patch.modified_date)];
    if patch.changes_body() {
        sql.push_str(", body = ?, ast = ?");
        values.push(Box::new(patch.body.clone().unwrap_or_default()));
        values.push(Box::new(patch.ast.clone().unwrap_or_default()));
    }
    if let Some(level) = patch.level {
        sql.push_str(", level = ?");
        values.push(Box::new(level));
    }
    if let Some(collapsed) = patch.collapsed {
        sql.push_str(", collapsed = ?");
        values.push(Box::new(collapsed as i64));
    }
    sql.push_str(" WHERE id = ?");
    values.push(Box::new(id.to_string()));
    let refs: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    Ok(conn.execute(&sql, refs.as_slice())? > 0)
}

fn patch_types(conn: &Connection, id: &Hmtid, patch: &ItemPatch) -> rusqlite::Result<()> {
    if patch.changes_body() {
        conn.execute(
            "DELETE FROM item_types WHERE item_id = ? AND type <> ?",
            params![id.as_str(), ItemPatch::TASK_TYPE],
        )?;
        for t in patch.body_types.iter().flatten() {
            insert_type(conn, id, t)?;
        }
    }
    match patch.task {
        Some(true) => insert_type(conn, id, ItemPatch::TASK_TYPE)?,
        Some(false) => {
            conn.execute(
                "DELETE FROM item_types WHERE item_id = ? AND type = ?",
                params![id.as_str(), ItemPatch::TASK_TYPE],
            )?;
        }
        None => {}
    }
    Ok(())
}
