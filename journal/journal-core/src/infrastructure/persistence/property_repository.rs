//! Scalar and list properties of items and pages.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Datelike, NaiveDate, TimeZone};
use rusqlite::{params, Connection};

use crate::domain::bounds::Bounds;
use crate::domain::number_text::format_number;
use crate::domain::property_definitions::{Definition, Definitions};
pub use crate::domain::property_key::validate_key;
use crate::domain::property_key::PropertyKey;
use crate::error::{AppError, AppResult};

use super::db::{now_millis, placeholders, Db};

/// Scalar properties per target: target id -> (key -> value as text).
pub type ScalarProps = BTreeMap<String, BTreeMap<String, String>>;
/// List properties per target: target id -> (key -> values).
pub type ListProps = BTreeMap<String, BTreeMap<String, Vec<String>>>;

/// A property definition: the key, its value type and who may use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDefinition {
    pub key: String,
    pub value_type: String,
    /// Written by the server itself; cannot be defined, changed or removed by the user.
    pub system: bool,
    /// May be set on a page (every user key, and `tags` among the system keys).
    pub page: bool,
}

#[derive(Debug, Clone)]
pub struct PropertyRepository {
    db: Arc<Db>,
}

impl PropertyRepository {
    pub fn new(db: Arc<Db>) -> PropertyRepository {
        PropertyRepository { db }
    }

    /// Scalar properties (text, number, checkbox, date, datetime, bounds) of the given targets.
    pub fn find_scalar_by_targets(
        &self,
        target_type: &str,
        target_ids: &[String],
    ) -> AppResult<ScalarProps> {
        Self::find_scalar_by_targets_in(&*self.db.conn()?, target_type, target_ids)
    }

    /// The scalar properties of the given targets, on the given connection.
    pub fn find_scalar_by_targets_in(
        conn: &Connection,
        target_type: &str,
        target_ids: &[String],
    ) -> AppResult<ScalarProps> {
        let mut result = ScalarProps::new();
        if target_ids.is_empty() {
            return Ok(result);
        }
        let sql = format!(
            "SELECT target_id, key, value FROM properties \
             WHERE target_type = ? AND target_id IN ({})",
            placeholders(target_ids.len())
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&target_type];
        for id in target_ids {
            params.push(id);
        }
        let mut rows = stmt.query(params.as_slice())?;
        while let Some(row) = rows.next()? {
            let target_id: String = row.get(0)?;
            let key: String = row.get(1)?;
            let value: String = row.get(2)?;
            result.entry(target_id).or_default().insert(key, value);
        }
        Ok(result)
    }

    /// List/tags properties of the given targets, in sort order.
    pub fn find_list_by_targets(
        &self,
        target_type: &str,
        target_ids: &[String],
    ) -> AppResult<ListProps> {
        Self::find_list_by_targets_in(&*self.db.conn()?, target_type, target_ids)
    }

    /// The list properties of the given targets, on the given connection.
    pub fn find_list_by_targets_in(
        conn: &Connection,
        target_type: &str,
        target_ids: &[String],
    ) -> AppResult<ListProps> {
        let mut result = ListProps::new();
        if target_ids.is_empty() {
            return Ok(result);
        }
        let sql = format!(
            "SELECT target_id, key, value FROM property_list_items \
             WHERE target_type = ? AND target_id IN ({}) ORDER BY target_id, key, sort_order",
            placeholders(target_ids.len())
        );
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&target_type];
        for id in target_ids {
            params.push(id);
        }
        let mut rows = stmt.query(params.as_slice())?;
        while let Some(row) = rows.next()? {
            let target_id: String = row.get(0)?;
            let key: String = row.get(1)?;
            let value: String = row.get(2)?;
            result
                .entry(target_id)
                .or_default()
                .entry(key)
                .or_default()
                .push(value);
        }
        Ok(result)
    }

    /// All property definitions, ordered by key: key -> value type.
    pub fn find_all_definitions(&self) -> AppResult<BTreeMap<String, String>> {
        Self::find_all_definitions_in(&*self.db.conn()?)
    }

    /// All property definitions, on the given connection: key -> value type.
    pub fn find_all_definitions_in(conn: &Connection) -> AppResult<BTreeMap<String, String>> {
        Ok(Self::find_definitions_in(conn)?
            .into_iter()
            .map(|d| (d.key, d.value_type))
            .collect())
    }

    /// All property definitions with their flags, ordered by key.
    pub fn find_definitions(&self) -> AppResult<Vec<PropertyDefinition>> {
        Self::find_definitions_in(&*self.db.conn()?)
    }

    /// All property definitions with their flags, on the given connection.
    pub fn find_definitions_in(conn: &Connection) -> AppResult<Vec<PropertyDefinition>> {
        let mut stmt = conn.prepare(
            "SELECT key, value_type, is_system, for_page FROM property_definitions ORDER BY key",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(PropertyDefinition {
                key: r.get(0)?,
                value_type: r.get(1)?,
                system: r.get::<_, i64>(2)? != 0,
                page: r.get::<_, i64>(3)? != 0,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn find_definition(&self, key: &str) -> AppResult<Option<PropertyDefinition>> {
        Self::find_definition_in(&*self.db.conn()?, key)
    }

    /// The definition of one key, on the given connection.
    pub fn find_definition_in(
        conn: &Connection,
        key: &str,
    ) -> AppResult<Option<PropertyDefinition>> {
        Ok(Self::find_definitions_in(conn)?
            .into_iter()
            .find(|d| d.key == key))
    }

    /// Defines a user key. A system key, an existing key and a type other than the one the
    /// name gives are refused (ADR 047).
    pub fn insert_definition(&self, key: &str, value_type: &str, page: bool) -> AppResult<()> {
        validate_key(key)?;
        PropertyKey::parse(key)?.check_type(value_type)?;
        // `rating_n_n` would show as `rating_n`, which looks like a number of `rating`
        property_names::name_of(property_names::body_of(key), property_names::type_of(key))
            .map_err(|_| {
                AppError::bad_request(format!(
                    "Property key '{key}' has a body that ends with a reserved suffix"
                ))
            })?;
        if let Some(existing) = self.find_definition(key)? {
            let what = if existing.system {
                "a system key"
            } else {
                "already defined"
            };
            return Err(AppError::bad_request(format!(
                "Property key '{key}' is {what}"
            )));
        }
        // The key is defined and no longer deleted at once, or neither
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        tx.execute(
            "INSERT INTO property_definitions (key, value_type, is_system, for_page, modified) VALUES (?, ?, 0, ?, ?)",
            params![key, value_type, page as i64, now_millis()],
        )?;
        tx.execute(
            "DELETE FROM deleted_property_definitions WHERE key = ?",
            params![key],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Defines a user key as another device defined it, or changes its definition to that.
    /// The values stay as they are: the type is how they are read (ADR 046).
    pub fn put_definition(
        &self,
        key: &str,
        value_type: &str,
        page: bool,
        modified: i64,
    ) -> AppResult<()> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        tx.execute(
            "INSERT INTO property_definitions (key, value_type, is_system, for_page, modified, provisional)
             VALUES (?1, ?2, 0, ?3, ?4, 0)
             ON CONFLICT(key) DO UPDATE SET value_type = ?2, for_page = ?3, modified = ?4, provisional = 0
             WHERE is_system = 0",
            params![key, value_type, page as i64, modified],
        )?;
        tx.execute(
            "DELETE FROM deleted_property_definitions WHERE key = ?",
            params![key],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Takes the deletion of a user key on another device: the date is kept, and the
    /// definition goes. While a value of the key is still here, the definition stays as a
    /// provisional one, to read that value; the values go when the files that dropped them
    /// come in (ADR 046).
    pub fn put_deletion(&self, key: &str, deleted: i64) -> AppResult<()> {
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
        tx.execute(
            "INSERT INTO deleted_property_definitions (key, deleted_date) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET deleted_date = MAX(deleted_date, excluded.deleted_date)",
            params![key, deleted],
        )?;
        let held: bool = tx.query_row(
            "SELECT EXISTS (SELECT 1 FROM properties WHERE key = ?1)
                 OR EXISTS (SELECT 1 FROM property_list_items WHERE key = ?1)",
            params![key],
            |r| r.get(0),
        )?;
        if held {
            tx.execute(
                "UPDATE property_definitions SET provisional = 1 WHERE key = ? AND is_system = 0",
                params![key],
            )?;
        } else {
            tx.execute(
                "DELETE FROM property_definitions WHERE key = ? AND is_system = 0",
                params![key],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The definitions this device gives the other devices: the real user definitions, and
    /// the keys deleted here (ADR 046).
    pub fn find_synced_definitions(&self) -> AppResult<Definitions> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT key, value_type, for_page, modified FROM property_definitions
             WHERE is_system = 0 AND provisional = 0",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Definition::Defined {
                    value_type: r.get(1)?,
                    page: r.get::<_, i64>(2)? != 0,
                    modified: r.get(3)?,
                },
            ))
        })?;
        let mut definitions: Definitions = rows.collect::<Result<_, _>>()?;
        let mut stmt =
            conn.prepare("SELECT key, deleted_date FROM deleted_property_definitions")?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                Definition::Deleted { deleted: r.get(1)? },
            ))
        })?;
        for row in rows {
            let (key, deleted) = row?;
            definitions.entry(key).or_insert(deleted);
        }
        Ok(definitions)
    }

    /// `delete_definition` on the given connection, as part of a larger transaction.
    pub fn delete_definition_in(tx: &Connection, key: &str) -> AppResult<Vec<String>> {
        match Self::find_definition_in(tx, key)? {
            None => {
                return Err(AppError::bad_request(format!(
                    "Property key '{key}' is not defined"
                )))
            }
            Some(def) if def.system => {
                return Err(AppError::bad_request(format!(
                    "Property key '{key}' is a system key"
                )))
            }
            Some(_) => {}
        }
        let page_ids: Vec<String> = {
            let mut stmt = tx.prepare(
                "SELECT target_id FROM properties WHERE key = ?1 AND target_type = 'page'
                 UNION SELECT target_id FROM property_list_items WHERE key = ?1 AND target_type = 'page'
                 UNION SELECT i.page_id FROM properties p JOIN items i ON p.target_id = i.id
                       WHERE p.key = ?1 AND p.target_type = 'item'
                 UNION SELECT i.page_id FROM property_list_items p JOIN items i ON p.target_id = i.id
                       WHERE p.key = ?1 AND p.target_type = 'item'",
            )?;
            let rows = stmt.query_map(params![key], |r| r.get::<_, String>(0))?;
            rows.collect::<Result<_, _>>()?
        };
        tx.execute("DELETE FROM properties WHERE key = ?", params![key])?;
        tx.execute(
            "DELETE FROM property_list_items WHERE key = ?",
            params![key],
        )?;
        tx.execute(
            "DELETE FROM property_definitions WHERE key = ?",
            params![key],
        )?;
        tx.execute(
            "INSERT INTO deleted_property_definitions (key, deleted_date) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET deleted_date = MAX(deleted_date, excluded.deleted_date)",
            params![key, now_millis()],
        )?;
        Ok(page_ids)
    }

    /// Sets a scalar value of a key, as part of a larger transaction. A key not defined yet
    /// is defined with the given type; a defined key keeps its type.
    pub fn upsert_scalar_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> AppResult<()> {
        validate_key(key)?;
        define_in(conn, key, value_type)?;
        Self::write_scalar_in(conn, target_type, target_id, key, value)
    }

    /// Takes in a scalar value from a file, as part of a larger transaction. A key with no
    /// definition gets a provisional one of the type its name gives (ADR 047), only to read
    /// the value. A value of a user
    /// key comes in as it is written, even when it does not fit the type (ADR 046).
    pub fn take_in_scalar_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
        key: &str,
        value: &str,
    ) -> AppResult<()> {
        validate_key(key)?;
        define_provisional_in(conn, key, property_names::type_of(key).as_str())?;
        let system = Self::find_definition_in(conn, key)?.is_some_and(|def| def.system);
        if system {
            // The app reads a system key by its type, so a value that does not fit stays out
            return Self::write_scalar_in(conn, target_type, target_id, key, value);
        }
        // The type of a user key is how its value is read; the value stays as the file has it
        conn.execute(
            "INSERT OR REPLACE INTO properties (target_type, target_id, key, value) VALUES (?, ?, ?, ?)",
            params![target_type, target_id, key, value],
        )?;
        Ok(())
    }

    /// Takes in a list from a file, as part of a larger transaction. A key with no
    /// definition gets a provisional one of `list` (ADR 046).
    pub fn take_in_list_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
        key: &str,
        values: &[String],
    ) -> AppResult<()> {
        validate_key(key)?;
        define_provisional_in(conn, key, "list")?;
        replace_list_in(conn, target_type, target_id, key, values)
    }

    /// Removes the provisional definitions whose keys have no value left: they were only
    /// there to read the values (ADR 046).
    pub fn drop_unused_provisional_in(conn: &Connection) -> AppResult<()> {
        conn.execute(
            "DELETE FROM property_definitions
             WHERE provisional = 1 AND is_system = 0
               AND key NOT IN (SELECT key FROM properties)
               AND key NOT IN (SELECT key FROM property_list_items)",
            [],
        )?;
        Ok(())
    }

    /// Sets a scalar value of a defined key, as part of a larger transaction. The value is
    /// checked against the type of the definition and kept as text in the form the export
    /// writes (`1.50` becomes `1.5`). A key with no definition is refused.
    pub fn write_scalar_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
        key: &str,
        value: &str,
    ) -> AppResult<()> {
        let value_type = Self::find_definition_in(conn, key)?
            .map(|def| def.value_type)
            .ok_or_else(|| AppError::bad_request(format!("Property '{key}' is not defined")))?;
        let text = match value_type.as_str() {
            "number" => {
                let n: f64 = value
                    .trim()
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Not a number: {value}")))?;
                format_number(n)
            }
            "checkbox" => (if value == "true" { "true" } else { "false" }).to_string(),
            "bounds" => value.parse::<Bounds>()?.to_string(),
            _ => value.to_string(),
        };
        conn.execute(
            "INSERT OR REPLACE INTO properties (target_type, target_id, key, value) VALUES (?, ?, ?, ?)",
            params![target_type, target_id, key, text],
        )?;
        Ok(())
    }

    /// `upsert_list` on the given connection, as part of a larger transaction.
    pub fn upsert_list_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
        key: &str,
        value_type: &str,
        values: &[String],
    ) -> AppResult<()> {
        validate_key(key)?;
        define_in(conn, key, value_type)?;
        replace_list_in(conn, target_type, target_id, key, values)
    }

    /// Removes a property of one target on the given connection, as part of a larger
    /// transaction.
    pub fn delete_property_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
        key: &str,
    ) -> AppResult<()> {
        conn.execute(
            "DELETE FROM properties WHERE target_type = ? AND target_id = ? AND key = ?",
            params![target_type, target_id, key],
        )?;
        conn.execute(
            "DELETE FROM property_list_items WHERE target_type = ? AND target_id = ? AND key = ?",
            params![target_type, target_id, key],
        )?;
        Ok(())
    }

    /// Removes every property of the targets on the given connection, as part of a larger
    /// transaction.
    pub fn delete_all_for_targets_in(
        conn: &Connection,
        target_type: &str,
        target_ids: &[String],
    ) -> AppResult<()> {
        if target_ids.is_empty() {
            return Ok(());
        }
        let ph = placeholders(target_ids.len());
        for sql in [
            format!("DELETE FROM properties WHERE target_type = ? AND target_id IN ({ph})"),
            format!(
                "DELETE FROM property_list_items WHERE target_type = ? AND target_id IN ({ph})"
            ),
        ] {
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&target_type];
            for id in target_ids {
                params.push(id);
            }
            conn.execute(&sql, params.as_slice())?;
        }
        Ok(())
    }

    /// Replaces the tags of a target on the given connection, as part of a larger transaction.
    pub fn sync_tags_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
        new_tags: &[String],
    ) -> AppResult<()> {
        replace_list_in(conn, target_type, target_id, "tags", new_tags)
    }

    /// The tags of one target, in sort order.
    pub fn find_tags(&self, target_type: &str, target_id: &str) -> AppResult<Vec<String>> {
        Self::find_tags_in(&*self.db.conn()?, target_type, target_id)
    }

    pub fn find_tags_in(
        conn: &Connection,
        target_type: &str,
        target_id: &str,
    ) -> AppResult<Vec<String>> {
        let mut stmt = conn.prepare(
            "SELECT value FROM property_list_items \
             WHERE target_type = ? AND target_id = ? AND key = 'tags' ORDER BY sort_order",
        )?;
        let rows = stmt.query_map(params![target_type, target_id], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Every tag that a page or an item carries, each once.
    pub fn find_all_tag_values(&self) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT value FROM property_list_items WHERE key = 'tags' ORDER BY value",
        )?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// How many pages and items carry the tag in any case of its letters.
    pub fn count_tag_uses_ignoring_case(&self, tag_value: &str) -> AppResult<i64> {
        let conn = self.db.conn()?;
        Ok(conn.query_row(
            "SELECT COUNT(*) FROM property_list_items WHERE key = 'tags' AND value = ? COLLATE NOCASE",
            params![tag_value],
            |r| r.get(0),
        )?)
    }

    /// The pages that carry the tag in their `tags` property, in any case of its letters.
    pub fn find_page_ids_by_tag_value_ignoring_case(
        &self,
        tag_value: &str,
    ) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT DISTINCT target_id FROM property_list_items \
             WHERE key = 'tags' AND value = ? COLLATE NOCASE AND target_type = 'page'",
        )?;
        let rows = stmt.query_map(params![tag_value], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The items that carry the tag in any case of its letters, as tags that differ only in
    /// case share one tag page.
    pub fn find_item_ids_by_tag_value_ignoring_case(
        &self,
        tag_value: &str,
        max_results: i64,
    ) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT target_id FROM property_list_items              WHERE key = 'tags' AND value = ? COLLATE NOCASE AND target_type = 'item' LIMIT ?",
        )?;
        let rows = stmt.query_map(params![tag_value, max_results], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Item IDs whose page path (dir_path || name) starts with the given prefix.
    pub fn find_item_ids_by_page_prefix(
        &self,
        path_prefix: &str,
        max_results: i64,
    ) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT i.id AS item_id FROM items i JOIN pages p ON i.page_id = p.id \
             WHERE (p.dir_path || p.name) LIKE ? || '%' LIMIT ?",
        )?;
        let rows = stmt.query_map(params![path_prefix, max_results], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Item IDs that have a specific scalar property key=value.
    pub fn find_item_ids_by_scalar_property(
        &self,
        key: &str,
        value: &str,
        max_results: i64,
    ) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT target_id FROM properties WHERE target_type = 'item' AND key = ? AND value = ? LIMIT ?",
        )?;
        let rows = stmt.query_map(params![key, value, max_results], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Item IDs by date range on created_date or modified_date.
    /// Supported expr formats: `2026-03`, `2026-03-12`, `>2026-03-01`, `<2026-03-01`, `2026-03-01..2026-03-12`.
    pub fn find_item_ids_by_date_range(
        &self,
        column: &str,
        expr: &str,
        max_results: i64,
    ) -> AppResult<Vec<String>> {
        if column != "created_date" && column != "modified_date" {
            return Err(AppError::bad_request(format!("Invalid column: {column}")));
        }
        let (sql, values): (String, Vec<i64>) = if let Some((a, b)) = expr.split_once("..") {
            (
                format!("SELECT id FROM items WHERE {column} >= ? AND {column} < ? LIMIT ?"),
                vec![
                    parse_date_to_millis(a, false)?,
                    parse_date_to_millis(b, true)?,
                ],
            )
        } else if let Some(rest) = expr.strip_prefix('>') {
            (
                format!("SELECT id FROM items WHERE {column} >= ? LIMIT ?"),
                vec![parse_date_to_millis(rest, true)?],
            )
        } else if let Some(rest) = expr.strip_prefix('<') {
            (
                format!("SELECT id FROM items WHERE {column} < ? LIMIT ?"),
                vec![parse_date_to_millis(rest, false)?],
            )
        } else {
            (
                format!("SELECT id FROM items WHERE {column} >= ? AND {column} < ? LIMIT ?"),
                vec![
                    parse_date_to_millis(expr, false)?,
                    parse_date_to_millis(expr, true)?,
                ],
            )
        };
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<&dyn rusqlite::ToSql> =
            values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        params.push(&max_results);
        let rows = stmt.query_map(params.as_slice(), |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

/// Parses a date string to epoch millis in Asia/Tokyo. With `to_end`, returns the exclusive
/// upper bound (next day or next month start).
fn parse_date_to_millis(date_str: &str, to_end: bool) -> AppResult<i64> {
    let zone = chrono_tz::Asia::Tokyo;
    let date = if date_str.len() == 7 {
        let (y, m) = date_str
            .split_once('-')
            .ok_or_else(|| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        let year: i32 = y
            .parse()
            .map_err(|_| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        let month: u32 = m
            .parse()
            .map_err(|_| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        let first = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        if to_end {
            let (ny, nm) = if month == 12 {
                (year + 1, 1)
            } else {
                (year, month + 1)
            };
            NaiveDate::from_ymd_opt(ny, nm, 1).unwrap_or(first)
        } else {
            first
        }
    } else {
        let d = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
            .map_err(|_| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        if to_end {
            d.succ_opt().unwrap_or(d)
        } else {
            d
        }
    };
    let _ = date.year();
    let dt = zone
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .ok_or_else(|| AppError::bad_request(format!("Invalid date: {date_str}")))?;
    Ok(dt.timestamp_millis())
}

/// Defines a key with no definition as the user wrote a value to it: a real definition of
/// now, which goes to the other devices (ADR 046). A defined key keeps its definition.
fn define_in(conn: &Connection, key: &str, value_type: &str) -> AppResult<()> {
    let defined = conn.execute(
        "INSERT OR IGNORE INTO property_definitions (key, value_type, modified) VALUES (?, ?, ?)",
        params![key, value_type, now_millis()],
    )?;
    if defined > 0 {
        conn.execute(
            "DELETE FROM deleted_property_definitions WHERE key = ?",
            params![key],
        )?;
    }
    Ok(())
}

/// Gives a key with no definition a provisional one, which is not written to
/// `settings/properties.json`. A defined key keeps its definition.
fn define_provisional_in(conn: &Connection, key: &str, value_type: &str) -> AppResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO property_definitions (key, value_type, provisional) VALUES (?, ?, 1)",
        params![key, value_type],
    )?;
    Ok(())
}

/// The values of a list as a client sends them: comma-separated, with the spaces around
/// each value and the empty values dropped.
pub fn list_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .collect()
}

/// Replaces a list property of a target with the given values.
fn replace_list_in(
    conn: &Connection,
    target_type: &str,
    target_id: &str,
    key: &str,
    values: &[String],
) -> AppResult<()> {
    conn.execute(
        "DELETE FROM property_list_items WHERE target_type = ? AND target_id = ? AND key = ?",
        params![target_type, target_id, key],
    )?;
    if !values.is_empty() {
        let mut stmt = conn.prepare(
            "INSERT INTO property_list_items (target_type, target_id, key, sort_order, value) VALUES (?, ?, ?, ?, ?)",
        )?;
        for (i, value) in values.iter().enumerate() {
            stmt.execute(params![target_type, target_id, key, i as i64, value])?;
        }
    }
    Ok(())
}
