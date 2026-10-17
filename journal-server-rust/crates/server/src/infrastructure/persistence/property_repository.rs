//! Scalar and list properties of items and pages.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{Datelike, NaiveDate, TimeZone};
use rusqlite::{params, Row};

use crate::error::{AppError, AppResult};

use super::db::{placeholders, Db};

/// Scalar properties per target: target id -> (key -> value as text).
pub type ScalarProps = BTreeMap<String, BTreeMap<String, String>>;
/// List properties per target: target id -> (key -> values).
pub type ListProps = BTreeMap<String, BTreeMap<String, Vec<String>>>;

/// The types a user may give a new key. `list` and `tags` stay internal: the only list
/// property is `tag`, which the server derives from the body of an item.
pub const USER_VALUE_TYPES: [&str; 5] = ["text", "number", "checkbox", "date", "datetime"];

/// A property definition: the key, its value type and who may use it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDefinition {
    pub key: String,
    pub value_type: String,
    /// Written by the server itself; cannot be defined, changed or removed by the user.
    pub system: bool,
    /// May be set on a page (every user key, and `tag` among the system keys).
    pub page: bool,
}

/// A key is a word of letters, digits, `_` and `-`: it is written unquoted in the metadata
/// line of an item and as a YAML key in the front matter of a page.
pub fn validate_key(key: &str) -> AppResult<()> {
    if key.is_empty() {
        return Err(AppError::bad_request("Property key is required"));
    }
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::bad_request(format!(
            "Property key '{key}' may only contain letters, digits, '_' and '-'"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct PropertyRepository {
    db: Arc<Db>,
}

impl PropertyRepository {
    pub fn new(db: Arc<Db>) -> PropertyRepository {
        PropertyRepository { db }
    }

    /// Scalar properties (text, number, checkbox, date, datetime) of the given targets.
    pub fn find_scalar_by_targets(&self, target_type: &str, target_ids: &[String]) -> AppResult<ScalarProps> {
        let mut result = ScalarProps::new();
        if target_ids.is_empty() {
            return Ok(result);
        }
        let sql = format!(
            "SELECT p.target_id, p.key, p.value_text, p.value_number, p.value_checkbox, pd.value_type \
             FROM properties p JOIN property_definitions pd ON p.key = pd.key \
             WHERE p.target_type = ? AND p.target_id IN ({})",
            placeholders(target_ids.len())
        );
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&target_type];
        for id in target_ids {
            params.push(id);
        }
        let mut rows = stmt.query(params.as_slice())?;
        while let Some(row) = rows.next()? {
            let target_id: String = row.get(0)?;
            let key: String = row.get(1)?;
            let value_type: String = row.get(5)?;
            if let Some(value) = extract_value(row, &value_type)? {
                result.entry(target_id).or_default().insert(key, value);
            }
        }
        Ok(result)
    }

    /// List/tags properties of the given targets, in sort order.
    pub fn find_list_by_targets(&self, target_type: &str, target_ids: &[String]) -> AppResult<ListProps> {
        let mut result = ListProps::new();
        if target_ids.is_empty() {
            return Ok(result);
        }
        let sql = format!(
            "SELECT target_id, key, value FROM property_list_items \
             WHERE target_type = ? AND target_id IN ({}) ORDER BY target_id, key, sort_order",
            placeholders(target_ids.len())
        );
        let conn = self.db.conn()?;
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
        Ok(self
            .find_definitions()?
            .into_iter()
            .map(|d| (d.key, d.value_type))
            .collect())
    }

    /// All property definitions with their flags, ordered by key.
    pub fn find_definitions(&self) -> AppResult<Vec<PropertyDefinition>> {
        let conn = self.db.conn()?;
        let mut stmt =
            conn.prepare("SELECT key, value_type, is_system, for_page FROM property_definitions ORDER BY key")?;
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
        Ok(self.find_definitions()?.into_iter().find(|d| d.key == key))
    }

    /// Defines a user key. A system key, an existing key and a type the user may not choose
    /// are refused.
    pub fn insert_definition(&self, key: &str, value_type: &str, page: bool) -> AppResult<()> {
        validate_key(key)?;
        if !USER_VALUE_TYPES.contains(&value_type) {
            return Err(AppError::bad_request(format!(
                "Value type must be one of {}",
                USER_VALUE_TYPES.join(", ")
            )));
        }
        if let Some(existing) = self.find_definition(key)? {
            let what = if existing.system { "a system key" } else { "already defined" };
            return Err(AppError::bad_request(format!("Property key '{key}' is {what}")));
        }
        let conn = self.db.conn()?;
        conn.execute(
            "INSERT INTO property_definitions (key, value_type, is_system, for_page) VALUES (?, ?, 0, ?)",
            params![key, value_type, page as i64],
        )?;
        Ok(())
    }

    /// Inserts or replaces a scalar property, creating its definition if needed.
    pub fn upsert_scalar(
        &self,
        target_type: &str,
        target_id: &str,
        key: &str,
        value_type: &str,
        value: &str,
    ) -> AppResult<()> {
        validate_key(key)?;
        let conn = self.db.conn()?;
        conn.execute(
            "INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES (?, ?)",
            params![key, value_type],
        )?;
        let sql = "INSERT OR REPLACE INTO properties (target_type, target_id, key, value_text, value_number, value_checkbox) VALUES (?, ?, ?, ?, ?, ?)";
        match value_type {
            "number" => {
                let n: f64 = value
                    .trim()
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Not a number: {value}")))?;
                conn.execute(sql, params![target_type, target_id, key, Option::<String>::None, n, Option::<i64>::None])?;
            }
            "checkbox" => {
                let b: i64 = if value == "true" { 1 } else { 0 };
                conn.execute(sql, params![target_type, target_id, key, Option::<String>::None, Option::<f64>::None, b])?;
            }
            _ => {
                conn.execute(sql, params![target_type, target_id, key, value, Option::<f64>::None, Option::<i64>::None])?;
            }
        }
        Ok(())
    }

    /// Removes a property, scalar or list, of one target.
    pub fn delete_property(&self, target_type: &str, target_id: &str, key: &str) -> AppResult<()> {
        let conn = self.db.conn()?;
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

    /// Deletes every scalar and list property of the given targets.
    pub fn delete_all_for_targets(&self, target_type: &str, target_ids: &[String]) -> AppResult<()> {
        if target_ids.is_empty() {
            return Ok(());
        }
        let ph = placeholders(target_ids.len());
        let conn = self.db.conn()?;
        for sql in [
            format!("DELETE FROM properties WHERE target_type = ? AND target_id IN ({ph})"),
            format!("DELETE FROM property_list_items WHERE target_type = ? AND target_id IN ({ph})"),
        ] {
            let mut params: Vec<&dyn rusqlite::ToSql> = vec![&target_type];
            for id in target_ids {
                params.push(id);
            }
            conn.execute(&sql, params.as_slice())?;
        }
        Ok(())
    }

    /// Replaces all tag entries of a target with the given tags.
    pub fn sync_tags(&self, target_type: &str, target_id: &str, new_tags: &[String]) -> AppResult<()> {
        let conn = self.db.conn()?;
        conn.execute(
            "DELETE FROM property_list_items WHERE target_type = ? AND target_id = ? AND key = 'tag'",
            params![target_type, target_id],
        )?;
        if !new_tags.is_empty() {
            let mut stmt = conn.prepare(
                "INSERT INTO property_list_items (target_type, target_id, key, sort_order, value) VALUES (?, ?, 'tag', ?, ?)",
            )?;
            for (i, tag) in new_tags.iter().enumerate() {
                stmt.execute(params![target_type, target_id, i as i64, tag])?;
            }
        }
        Ok(())
    }

    pub fn find_item_ids_by_tag_value(&self, tag_value: &str, max_results: i64) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT target_id FROM property_list_items WHERE key = 'tag' AND value = ? AND target_type = 'item' LIMIT ?",
        )?;
        let rows = stmt.query_map(params![tag_value, max_results], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn find_any_tag_exists(&self) -> bool {
        let Ok(conn) = self.db.conn() else { return false };
        conn.query_row(
            "SELECT COUNT(*) FROM property_list_items WHERE key = 'tag' LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
    }

    /// Item IDs whose page path (dir_path || name) starts with the given prefix.
    pub fn find_item_ids_by_page_prefix(&self, path_prefix: &str, max_results: i64) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT i.id AS item_id FROM items i JOIN pages p ON i.page_id = p.id \
             WHERE (p.dir_path || p.name) LIKE ? || '%' LIMIT ?",
        )?;
        let rows = stmt.query_map(params![path_prefix, max_results], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Item IDs that have a specific scalar property key=value.
    pub fn find_item_ids_by_scalar_property(&self, key: &str, value: &str, max_results: i64) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(
            "SELECT target_id FROM properties WHERE target_type = 'item' AND key = ? AND value_text = ? LIMIT ?",
        )?;
        let rows = stmt.query_map(params![key, value, max_results], |r| r.get::<_, String>(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Item IDs by date range on created_date or modified_date.
    /// Supported expr formats: `2026-03`, `2026-03-12`, `>2026-03-01`, `<2026-03-01`, `2026-03-01..2026-03-12`.
    pub fn find_item_ids_by_date_range(&self, column: &str, expr: &str, max_results: i64) -> AppResult<Vec<String>> {
        if column != "created_date" && column != "modified_date" {
            return Err(AppError::bad_request(format!("Invalid column: {column}")));
        }
        let (sql, values): (String, Vec<i64>) = if let Some((a, b)) = expr.split_once("..") {
            (
                format!("SELECT id FROM items WHERE {column} >= ? AND {column} < ? LIMIT ?"),
                vec![parse_date_to_millis(a, false)?, parse_date_to_millis(b, true)?],
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
                vec![parse_date_to_millis(expr, false)?, parse_date_to_millis(expr, true)?],
            )
        };
        let conn = self.db.conn()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
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
        let year: i32 = y.parse().map_err(|_| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        let month: u32 = m.parse().map_err(|_| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        let first = NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| AppError::bad_request(format!("Invalid date: {date_str}")))?;
        if to_end {
            let (ny, nm) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
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

fn extract_value(row: &Row<'_>, value_type: &str) -> rusqlite::Result<Option<String>> {
    Ok(match value_type {
        "number" => {
            let num: Option<f64> = row.get(3)?;
            num.map(|n| {
                if n == n.floor() && n.abs() < 1e15 {
                    format!("{}", n as i64)
                } else {
                    format_double(n)
                }
            })
        }
        "checkbox" => {
            let cb: Option<i64> = row.get(4)?;
            cb.map(|c| if c != 0 { "true".to_string() } else { "false".to_string() })
        }
        _ => row.get::<_, Option<String>>(2)?,
    })
}

/// Formats a non-integral double the way Java's `String.valueOf(double)` does for common values.
fn format_double(n: f64) -> String {
    let s = format!("{n}");
    if s.contains('.') || s.contains('e') || s.contains("inf") || s.contains("NaN") {
        s
    } else {
        format!("{s}.0")
    }
}
