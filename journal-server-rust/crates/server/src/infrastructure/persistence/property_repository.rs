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
/// property is `tags`, which the server derives from the body of an item.
pub const USER_VALUE_TYPES: [&str; 6] =
    ["text", "number", "checkbox", "date", "datetime", "bounds"];

/// The start of the keys of the card groups on an atlas page. Each group is a list of the
/// ids of its pages under a key of its own, which the frontend names.
pub const ATLAS_GROUP_PREFIX: &str = "atlas_group_";

pub fn is_atlas_group_key(key: &str) -> bool {
    key.len() > ATLAS_GROUP_PREFIX.len() && key.starts_with(ATLAS_GROUP_PREFIX)
}

/// The start of the keys of the titles of the card groups on an atlas page. A title is text
/// under the key of its group with this prefix instead.
pub const ATLAS_TITLE_PREFIX: &str = "atlas_title_";

/// The start of the keys of the callouts on an atlas page: text under a key of its own, which
/// the frontend names.
pub const ATLAS_CALLOUT_PREFIX: &str = "atlas_callout_";

/// A key of text the frontend makes on an atlas page: a group title or a callout
pub fn is_atlas_text_key(key: &str) -> bool {
    [ATLAS_TITLE_PREFIX, ATLAS_CALLOUT_PREFIX]
        .iter()
        .any(|prefix| key.len() > prefix.len() && key.starts_with(prefix))
}

/// A key the frontend makes for an atlas, not one the user chooses
pub fn is_atlas_key(key: &str) -> bool {
    is_atlas_group_key(key) || is_atlas_text_key(key)
}

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

/// A key is a word of letters, digits, `_` and `-`: it is written unquoted in the metadata
/// line of an item and as a YAML key in the front matter of a page.
pub fn validate_key(key: &str) -> AppResult<()> {
    if key.is_empty() {
        return Err(AppError::bad_request("Property key is required"));
    }
    if !key
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
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

    /// Scalar properties (text, number, checkbox, date, datetime, bounds) of the given targets.
    pub fn find_scalar_by_targets(
        &self,
        target_type: &str,
        target_ids: &[String],
    ) -> AppResult<ScalarProps> {
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
    pub fn find_list_by_targets(
        &self,
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
            let what = if existing.system {
                "a system key"
            } else {
                "already defined"
            };
            return Err(AppError::bad_request(format!(
                "Property key '{key}' is {what}"
            )));
        }
        let conn = self.db.conn()?;
        conn.execute(
            "INSERT INTO property_definitions (key, value_type, is_system, for_page) VALUES (?, ?, 0, ?)",
            params![key, value_type, page as i64],
        )?;
        Ok(())
    }

    /// Removes a user key with every value set under it, on items and pages alike, and
    /// answers with the ids of the pages that lost a value: their own, or one of their
    /// items' values. A system key and an unknown key are refused.
    pub fn delete_definition(&self, key: &str) -> AppResult<Vec<String>> {
        match self.find_definition(key)? {
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
        let mut conn = self.db.conn()?;
        let tx = conn.immediate()?;
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
        tx.commit()?;
        Ok(page_ids)
    }

    /// Inserts or replaces a scalar property, creating its definition if needed. The value
    /// is checked and stored by the type of the definition: `value_type` only names the type
    /// of a new key.
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
        let value_type: String = conn.query_row(
            "SELECT value_type FROM property_definitions WHERE key = ?",
            params![key],
            |r| r.get(0),
        )?;
        let sql = "INSERT OR REPLACE INTO properties (target_type, target_id, key, value_text, value_number, value_checkbox) VALUES (?, ?, ?, ?, ?, ?)";
        match value_type.as_str() {
            "number" => {
                let n: f64 = value
                    .trim()
                    .parse()
                    .map_err(|_| AppError::bad_request(format!("Not a number: {value}")))?;
                conn.execute(
                    sql,
                    params![
                        target_type,
                        target_id,
                        key,
                        Option::<String>::None,
                        n,
                        Option::<i64>::None
                    ],
                )?;
            }
            "checkbox" => {
                let b: i64 = if value == "true" { 1 } else { 0 };
                conn.execute(
                    sql,
                    params![
                        target_type,
                        target_id,
                        key,
                        Option::<String>::None,
                        Option::<f64>::None,
                        b
                    ],
                )?;
            }
            "bounds" => {
                let bounds = normalize_bounds(value)?;
                conn.execute(
                    sql,
                    params![
                        target_type,
                        target_id,
                        key,
                        bounds,
                        Option::<f64>::None,
                        Option::<i64>::None
                    ],
                )?;
            }
            _ => {
                conn.execute(
                    sql,
                    params![
                        target_type,
                        target_id,
                        key,
                        value,
                        Option::<f64>::None,
                        Option::<i64>::None
                    ],
                )?;
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
    pub fn delete_all_for_targets(
        &self,
        target_type: &str,
        target_ids: &[String],
    ) -> AppResult<()> {
        if target_ids.is_empty() {
            return Ok(());
        }
        let ph = placeholders(target_ids.len());
        let conn = self.db.conn()?;
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

    /// Replaces all tag entries of a target with the given tags.
    pub fn sync_tags(
        &self,
        target_type: &str,
        target_id: &str,
        new_tags: &[String],
    ) -> AppResult<()> {
        self.replace_list(target_type, target_id, "tags", new_tags)
    }

    /// Replaces the ids of a card group on an atlas page. The key gets a system definition
    /// of its own, as every value must have one. A system key stays out of the user's
    /// hands, and the settings leave group keys out of their list.
    pub fn sync_atlas_group(&self, page_id: &str, key: &str, ids: &[String]) -> AppResult<()> {
        validate_key(key)?;
        self.db.conn()?.execute(
            "INSERT OR IGNORE INTO property_definitions (key, value_type, is_system, for_page) VALUES (?, 'list', 1, 1)",
            params![key],
        )?;
        self.replace_list("page", page_id, key, ids)
    }

    /// Drops the definition of a card group that no page holds any more, so the definitions
    /// do not pile up as groups come and go.
    pub fn delete_unused_atlas_group(&self, key: &str) -> AppResult<()> {
        self.db.conn()?.execute(
            "DELETE FROM property_definitions WHERE key = ?1 \
             AND NOT EXISTS (SELECT 1 FROM property_list_items WHERE key = ?1)",
            params![key],
        )?;
        Ok(())
    }

    /// Removes the definition of a group title or a callout once no page has it. Each has its
    /// own key, so the definitions would pile up as they come and go.
    pub fn delete_unused_atlas_text(&self, key: &str) -> AppResult<()> {
        self.db.conn()?.execute(
            "DELETE FROM property_definitions WHERE key = ?1 \
             AND NOT EXISTS (SELECT 1 FROM properties WHERE key = ?1)",
            params![key],
        )?;
        Ok(())
    }

    fn replace_list(
        &self,
        target_type: &str,
        target_id: &str,
        key: &str,
        values: &[String],
    ) -> AppResult<()> {
        let conn = self.db.conn()?;
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

    /// The tags of one target, in sort order.
    pub fn find_tags(&self, target_type: &str, target_id: &str) -> AppResult<Vec<String>> {
        let conn = self.db.conn()?;
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

    pub fn find_any_tag_exists(&self) -> bool {
        let Ok(conn) = self.db.conn() else {
            return false;
        };
        conn.query_row(
            "SELECT COUNT(*) FROM property_list_items WHERE key = 'tags' LIMIT 1",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false)
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
            "SELECT target_id FROM properties WHERE target_type = 'item' AND key = ? AND value_text = ? LIMIT ?",
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

/// Reads a bounds value `x,y,width,height` and writes it back without spaces, each number as
/// a number property shows it. It stays text rather than four columns, so the front matter
/// and the metadata line of an item hold it as one plain value.
pub fn normalize_bounds(value: &str) -> AppResult<String> {
    let invalid = || {
        AppError::bad_request(format!(
            "Bounds must be four numbers x,y,width,height: {value}"
        ))
    };
    let numbers = value
        .split(',')
        .map(|n| n.trim().parse::<f64>().ok().filter(|n| n.is_finite()))
        .collect::<Option<Vec<f64>>>()
        .ok_or_else(invalid)?;
    if numbers.len() != 4 {
        return Err(invalid());
    }
    Ok(numbers
        .into_iter()
        .map(format_number)
        .collect::<Vec<_>>()
        .join(","))
}

fn format_number(n: f64) -> String {
    if n == n.floor() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format_double(n)
    }
}

fn extract_value(row: &Row<'_>, value_type: &str) -> rusqlite::Result<Option<String>> {
    Ok(match value_type {
        "number" => {
            let num: Option<f64> = row.get(3)?;
            num.map(format_number)
        }
        "checkbox" => {
            let cb: Option<i64> = row.get(4)?;
            cb.map(|c| {
                if c != 0 {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            })
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
