//! Creates the tables and indexes, seeds what every database has, then runs the migrations.
//! Safe to run on an existing database: everything is "if not exists" or checks first.
//!
//! A migration stays until every database it was written for has been opened once; then it
//! goes. The ones here came with the Rust server; the Java server's are gone with it.

use anyhow::Context;
use rusqlite::{params, Connection};

use crate::domain::id_generator::IdGenerator;

use super::db::{now_millis, Db};

/// Bump this constant whenever the DB schema changes.
pub const SCHEMA_VERSION: &str = "0.1";

pub fn initialize(db: &Db, ids: &IdGenerator) -> anyhow::Result<()> {
    let conn = db.conn()?;
    run(&conn, ids).context("Failed to initialize database")?;
    tracing::info!("Database initialized");
    Ok(())
}

fn run(conn: &Connection, ids: &IdGenerator) -> rusqlite::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS pages (
            id TEXT PRIMARY KEY,
            parent_id TEXT,
            dir_path TEXT NOT NULL,
            name TEXT NOT NULL,
            type TEXT NOT NULL DEFAULT 'doc',
            created_date INTEGER NOT NULL,
            modified_date INTEGER NOT NULL
        );

        CREATE TABLE IF NOT EXISTS items (
            id TEXT PRIMARY KEY,
            page_id TEXT NOT NULL,
            parent_id TEXT,
            created_date INTEGER NOT NULL,
            modified_date INTEGER NOT NULL,
            idx TEXT NOT NULL,
            level INTEGER NOT NULL DEFAULT 0,
            body TEXT NOT NULL DEFAULT '',
            ast TEXT NOT NULL DEFAULT '',
            collapsed INTEGER NOT NULL DEFAULT 0,
            transclude_item_id TEXT REFERENCES items(id) ON DELETE SET NULL,
            FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS item_types (
            item_id TEXT NOT NULL,
            type TEXT NOT NULL,
            PRIMARY KEY (item_id, type),
            FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS export_queue (
            page_id TEXT PRIMARY KEY,
            queued_at INTEGER NOT NULL,
            FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS file_sync_state (
            rel_path TEXT PRIMARY KEY,
            page_id TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            file_mtime INTEGER NOT NULL,
            synced_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'synced' CHECK(status IN ('synced', 'missing')),
            missing_since INTEGER
        );
        CREATE INDEX IF NOT EXISTS idx_fss_page_id ON file_sync_state(page_id);

        CREATE TABLE IF NOT EXISTS property_definitions (
            key TEXT PRIMARY KEY,
            value_type TEXT NOT NULL CHECK(value_type IN (
                'text', 'number', 'checkbox', 'date', 'datetime', 'bounds', 'list', 'tags'
            )),
            is_system INTEGER NOT NULL DEFAULT 0,
            for_page INTEGER NOT NULL DEFAULT 1
        );

        CREATE TABLE IF NOT EXISTS properties (
            target_type TEXT NOT NULL CHECK(target_type IN ('item', 'page')),
            target_id TEXT NOT NULL,
            key TEXT NOT NULL REFERENCES property_definitions(key),
            value_text TEXT,
            value_number REAL,
            value_checkbox INTEGER,
            PRIMARY KEY (target_type, target_id, key)
        );

        CREATE TABLE IF NOT EXISTS property_list_items (
            id INTEGER PRIMARY KEY,
            target_type TEXT NOT NULL CHECK(target_type IN ('item', 'page')),
            target_id TEXT NOT NULL,
            key TEXT NOT NULL REFERENCES property_definitions(key),
            sort_order REAL NOT NULL,
            value TEXT NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_pages_type_dir_name ON pages(type, dir_path, name);
        CREATE INDEX IF NOT EXISTS idx_pages_modified ON pages(modified_date DESC);
        CREATE INDEX IF NOT EXISTS idx_pages_parent_id ON pages(parent_id);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_items_page_id_idx ON items(page_id, idx);
        CREATE INDEX IF NOT EXISTS idx_items_parent_id ON items(parent_id);
        CREATE INDEX IF NOT EXISTS idx_items_body ON items(body);
        CREATE INDEX IF NOT EXISTS idx_items_transclude ON items(transclude_item_id) WHERE transclude_item_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_props_key_text ON properties(key, value_text);
        CREATE INDEX IF NOT EXISTS idx_props_key_number ON properties(key, value_number);
        CREATE INDEX IF NOT EXISTS idx_pli_target ON property_list_items(target_type, target_id, key, sort_order);
        CREATE INDEX IF NOT EXISTS idx_pli_value ON property_list_items(key, value);
        "#,
    )?;

    ensure_root_index(conn, ids)?;
    migrate_tag_key_to_tags(conn)?;
    // Before the keys: an old table refuses a system key of the bounds type
    migrate_bounds_value_type(conn)?;
    seed_system_keys(conn)?;
    Ok(())
}

/// The keys the app reserves, with their types, and whether they can be set on a page. The
/// task dates and times and the zoom are item-only; `created` and `modified` are the page's
/// own dates, which a page property write changes in place; `published` is the date-time
/// the user publishes a page at, which the sidebar can sort pages by. The `atlas_` keys are
/// on the items of an atlas page: where a card or a callout lies, the page a card stands
/// for, what an item that is no card is, and the side of a title or a bubble.
pub const SYSTEM_KEYS: [(&str, &str, bool); 15] = [
    ("tags", "tags", true),
    ("begin", "datetime", false),
    ("end", "datetime", false),
    ("plan", "date", false),
    ("due", "date", false),
    ("cancelled", "date", false),
    ("zoom", "number", false),
    ("id", "text", false),
    ("created", "datetime", true),
    ("modified", "datetime", true),
    ("published", "datetime", true),
    ("atlas_bounds", "bounds", false),
    ("atlas_page", "text", false),
    ("atlas_type", "text", false),
    ("atlas_side", "text", false),
];

/// Creates the root index page with one empty item when the database has none.
fn ensure_root_index(conn: &Connection, ids: &IdGenerator) -> rusqlite::Result<()> {
    let exists: Option<String> = conn
        .query_row(
            "SELECT id FROM pages WHERE dir_path = '' AND name = 'index' LIMIT 1",
            [],
            |r| r.get(0),
        )
        .ok();
    if exists.is_some() {
        return Ok(());
    }
    tracing::info!("Creating root index page...");
    let root_id = ids.generate_page_id().to_string();
    let now = now_millis();
    conn.execute(
        "INSERT INTO pages (id, parent_id, dir_path, name, type, created_date, modified_date) \
         VALUES (?, NULL, '', 'index', 'doc', ?, ?)",
        params![root_id, now, now],
    )?;
    let item_id = ids.generate_item_id().to_string();
    conn.execute(
        "INSERT INTO items (id, page_id, parent_id, created_date, modified_date, idx, level, body, ast, collapsed) \
         VALUES (?, ?, NULL, ?, ?, 'a0', 1, '', '', 0)",
        params![item_id, root_id, now, now],
    )?;
    tracing::info!("Root index page created: {root_id}");
    Ok(())
}

/// Defines the system keys and marks them. Runs on every start, so a key that became a
/// system key is flagged too, and gets the reserved type even when a user had defined it
/// with another one.
fn seed_system_keys(conn: &Connection) -> rusqlite::Result<()> {
    for (key, value_type, for_page) in SYSTEM_KEYS {
        conn.execute(
            "INSERT OR IGNORE INTO property_definitions (key, value_type, is_system, for_page) VALUES (?, ?, 1, ?)",
            rusqlite::params![key, value_type, for_page as i64],
        )?;
        conn.execute(
            "UPDATE property_definitions SET is_system = 1, for_page = ?, value_type = ? WHERE key = ?",
            rusqlite::params![for_page as i64, value_type, key],
        )?;
    }
    Ok(())
}

/// Renames the system key `tag` to `tags`, the key other Markdown tools use in the front
/// matter. The rows are renamed in place, so no tag is lost. The old definition goes only
/// when nothing refers to it any more. From 2026-10-27.
fn migrate_tag_key_to_tags(conn: &Connection) -> rusqlite::Result<()> {
    let old: i64 = conn.query_row(
        "SELECT COUNT(*) FROM property_definitions WHERE key = 'tag'",
        [],
        |r| r.get(0),
    )?;
    if old == 0 {
        return Ok(());
    }
    tracing::info!("Renaming the property key 'tag' to 'tags'...");
    conn.execute_batch(
        "BEGIN;
         INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('tags', 'tags');
         UPDATE property_list_items SET key = 'tags' WHERE key = 'tag';
         DELETE FROM property_definitions WHERE key = 'tag'
             AND NOT EXISTS (SELECT 1 FROM properties WHERE key = 'tag');
         COMMIT;",
    )?;
    Ok(())
}

/// Lets a property definition take the `bounds` type. SQLite cannot change the CHECK of a
/// column in place, so the table is built again under the same name. Its rows are copied
/// with an INSERT, not deleted: the properties refer to their keys. From 2026-10-27.
fn migrate_bounds_value_type(conn: &Connection) -> rusqlite::Result<()> {
    let sql: String = conn.query_row(
        "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'property_definitions'",
        [],
        |r| r.get(0),
    )?;
    if sql.contains("'bounds'") {
        return Ok(());
    }
    tracing::info!("Adding the bounds value type to property_definitions...");
    // Off while the tables are swapped: dropping the old table would check the properties
    // that refer to it. The pragma cannot change inside a transaction.
    conn.pragma_update(None, "foreign_keys", "OFF")?;
    let result = conn.execute_batch(
        "BEGIN;
         CREATE TABLE property_definitions_new (
             key TEXT PRIMARY KEY,
             value_type TEXT NOT NULL CHECK(value_type IN (
                 'text', 'number', 'checkbox', 'date', 'datetime', 'bounds', 'list', 'tags'
             )),
             is_system INTEGER NOT NULL DEFAULT 0,
             for_page INTEGER NOT NULL DEFAULT 1
         );
         INSERT INTO property_definitions_new (key, value_type, is_system, for_page)
             SELECT key, value_type, is_system, for_page FROM property_definitions;
         DROP TABLE property_definitions;
         ALTER TABLE property_definitions_new RENAME TO property_definitions;
         COMMIT;",
    );
    if result.is_err() {
        let _ = conn.execute_batch("ROLLBACK");
    }
    conn.pragma_update(None, "foreign_keys", "ON")?;
    result
}

/// Reads or creates `versions.json` in the config directory and warns on a schema mismatch.
pub fn check_schema_version(config_dir: &std::path::Path) {
    let version_file = config_dir.join("versions.json");
    tracing::info!("Schema version (embedded): {SCHEMA_VERSION}");
    if version_file.exists() {
        match std::fs::read(&version_file)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| {
                serde_json::from_slice::<serde_json::Value>(&bytes).map_err(Into::into)
            }) {
            Ok(json) => {
                let file_version = json
                    .get("schemaVersion")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                tracing::info!("Schema version (file):     {file_version}");
                if file_version != SCHEMA_VERSION {
                    tracing::warn!(
                        "Schema version MISMATCH! embedded={SCHEMA_VERSION}, file={file_version}. \
                         A database migration may be required."
                    );
                }
            }
            Err(e) => tracing::error!("Failed to read versions.json: {e}"),
        }
    } else {
        tracing::info!(
            "Schema version file not found. Creating {}",
            version_file.display()
        );
        let json = serde_json::json!({ "schemaVersion": SCHEMA_VERSION });
        let result = std::fs::create_dir_all(config_dir).and_then(|_| {
            std::fs::write(
                &version_file,
                serde_json::to_string_pretty(&json).unwrap_or_default(),
            )
        });
        match result {
            Ok(_) => tracing::info!("Wrote {}", version_file.display()),
            Err(e) => tracing::error!("Failed to write versions.json: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    // tagキーは、タグを失わずにtagsに改名される。
    #[test]
    fn the_tag_key_is_renamed_to_tags_without_losing_a_tag() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::new()).unwrap();
        // A database from before the rename
        conn.execute_batch(
            "INSERT INTO property_definitions (key, value_type, is_system) VALUES ('tag', 'tags', 1);
             INSERT INTO property_list_items (target_type, target_id, key, sort_order, value)
                 VALUES ('page', 'p1', 'tag', 0, 'a'), ('item', 'i1', 'tag', 0, 'b');",
        )
        .unwrap();

        run(&conn, &IdGenerator::new()).unwrap();

        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_list_items WHERE key = 'tags'"
            ),
            2
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_list_items WHERE key = 'tag'"
            ),
            0
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_definitions WHERE key = 'tag'"
            ),
            0
        );
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM property_definitions WHERE key = 'tags' AND is_system = 1 AND for_page = 1"),
            1
        );
    }

    // 古いデータベースはbounds型を受け入れ、プロパティを保つ。
    #[test]
    fn an_old_database_takes_the_bounds_type_and_keeps_its_properties() {
        let conn = Connection::open_in_memory().unwrap();
        // The tables as they were before the bounds type, with values set under the keys
        conn.execute_batch(
            "CREATE TABLE property_definitions (
                 key TEXT PRIMARY KEY,
                 value_type TEXT NOT NULL CHECK(value_type IN (
                     'text', 'number', 'checkbox', 'date', 'datetime', 'list', 'tags'
                 )),
                 is_system INTEGER NOT NULL DEFAULT 0,
                 for_page INTEGER NOT NULL DEFAULT 1
             );
             CREATE TABLE properties (
                 target_type TEXT NOT NULL CHECK(target_type IN ('item', 'page')),
                 target_id TEXT NOT NULL,
                 key TEXT NOT NULL REFERENCES property_definitions(key),
                 value_text TEXT,
                 value_number REAL,
                 value_checkbox INTEGER,
                 PRIMARY KEY (target_type, target_id, key)
             );
             CREATE TABLE property_list_items (
                 id INTEGER PRIMARY KEY,
                 target_type TEXT NOT NULL CHECK(target_type IN ('item', 'page')),
                 target_id TEXT NOT NULL,
                 key TEXT NOT NULL REFERENCES property_definitions(key),
                 sort_order REAL NOT NULL,
                 value TEXT NOT NULL
             );
             INSERT INTO property_definitions (key, value_type) VALUES ('owner', 'text'), ('tags', 'tags');
             INSERT INTO properties (target_type, target_id, key, value_text) VALUES ('page', 'p1', 'owner', 'me');
             INSERT INTO property_list_items (target_type, target_id, key, sort_order, value)
                 VALUES ('page', 'p1', 'tags', 0, 'a');",
        )
        .unwrap();

        run(&conn, &IdGenerator::new()).unwrap();

        conn.execute(
            "INSERT INTO property_definitions (key, value_type) VALUES ('area', 'bounds')",
            [],
        )
        .unwrap();
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM properties WHERE key = 'owner'"),
            1
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_list_items WHERE key = 'tags'"
            ),
            1
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_definitions WHERE key = 'tags' AND is_system = 1"
            ),
            1
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_definitions
                 WHERE key = 'atlas_bounds' AND value_type = 'bounds' AND is_system = 1 AND for_page = 0"
            ),
            1
        );
        let mut check = conn.prepare("PRAGMA foreign_key_check").unwrap();
        assert!(check.query([]).unwrap().next().unwrap().is_none());
        // The foreign keys are checked again after the swap
        assert!(conn
            .execute("INSERT INTO properties (target_type, target_id, key) VALUES ('page', 'p1', 'nokey')", [])
            .is_err());
    }
}
