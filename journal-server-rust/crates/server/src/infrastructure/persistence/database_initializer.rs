//! Creates the tables and indexes, then runs the migrations.
//! Safe to run on an existing database: everything is "if not exists" or checks first.

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

    // Drop removed tables
    conn.execute_batch(
        "DROP TABLE IF EXISTS page_tags;
         DROP TABLE IF EXISTS bookmarks;
         DROP TABLE IF EXISTS bookmark_groups;",
    )?;

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
                'text', 'number', 'checkbox', 'date', 'datetime', 'list', 'tags'
            ))
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
        CREATE INDEX IF NOT EXISTS idx_props_key_text ON properties(key, value_text);
        CREATE INDEX IF NOT EXISTS idx_props_key_number ON properties(key, value_number);
        CREATE INDEX IF NOT EXISTS idx_pli_target ON property_list_items(target_type, target_id, key, sort_order);
        CREATE INDEX IF NOT EXISTS idx_pli_value ON property_list_items(key, value);
        "#,
    )?;

    migrate_html_to_ast(conn)?;
    migrate_properties(conn)?;
    migrate_root_index(conn, ids)?;
    migrate_transclusion(conn)?;
    migrate_datetime_to_seconds(conn)?;
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for name in names {
        if name? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn migrate_html_to_ast(conn: &Connection) -> rusqlite::Result<()> {
    if has_column(conn, "items", "html")? {
        tracing::info!("Renaming items.html column to items.ast...");
        conn.execute_batch("ALTER TABLE items RENAME COLUMN html TO ast")?;
        tracing::info!("Column renamed successfully.");
    }
    Ok(())
}

fn backfill_task_plan_property(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row(
        r#"
        SELECT COUNT(*) FROM item_types it
        JOIN items i ON it.item_id = i.id
        JOIN pages p ON i.page_id = p.id
        WHERE it.type = 'task'
          AND p.type = 'journal'
          AND p.name GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
          AND NOT EXISTS (
            SELECT 1 FROM properties pr
            WHERE pr.target_type = 'item' AND pr.target_id = it.item_id AND pr.key = 'plan'
          )
        "#,
        [],
        |r| r.get(0),
    )?;
    if count > 0 {
        tracing::info!("Backfilling plan property for {count} task items on daily journals...");
        conn.execute_batch(
            r#"
            INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('plan', 'date');
            INSERT OR IGNORE INTO properties (target_type, target_id, key, value_text)
            SELECT 'item', it.item_id, 'plan', p.name
            FROM item_types it
            JOIN items i ON it.item_id = i.id
            JOIN pages p ON i.page_id = p.id
            WHERE it.type = 'task'
              AND p.type = 'journal'
              AND p.name GLOB '[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]'
              AND NOT EXISTS (
                SELECT 1 FROM properties pr
                WHERE pr.target_type = 'item' AND pr.target_id = it.item_id AND pr.key = 'plan'
              );
            "#,
        )?;
        tracing::info!("Plan property backfill complete");
    }
    Ok(())
}

fn migrate_root_index(conn: &Connection, ids: &IdGenerator) -> rusqlite::Result<()> {
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

fn migrate_transclusion(conn: &Connection) -> rusqlite::Result<()> {
    if has_column(conn, "items", "transclude_item_id")? {
        return Ok(());
    }
    tracing::info!("Adding transclude_item_id column to items...");
    conn.execute_batch(
        "ALTER TABLE items ADD COLUMN transclude_item_id TEXT REFERENCES items(id) ON DELETE SET NULL;
         CREATE INDEX IF NOT EXISTS idx_items_transclude ON items(transclude_item_id) WHERE transclude_item_id IS NOT NULL;",
    )?;
    tracing::info!("Transclusion migration complete");
    Ok(())
}

fn migrate_datetime_to_seconds(conn: &Connection) -> rusqlite::Result<()> {
    // Older versions stored datetime properties with nanoseconds ("...:47.387871400Z").
    // New values stop at whole seconds, so cut the fraction off the stored ones too.
    // The pages that change are queued so their markdown files get the same values.
    conn.execute(
        r#"
        INSERT OR IGNORE INTO export_queue (page_id, queued_at)
        SELECT DISTINCT i.page_id, strftime('%s', 'now') * 1000
        FROM properties p
        JOIN property_definitions d ON d.key = p.key AND d.value_type = 'datetime'
        JOIN items i ON p.target_type = 'item' AND i.id = p.target_id
        WHERE p.value_text LIKE '____-__-__T__:__:__.%Z'
        "#,
        [],
    )?;
    let updated = conn.execute(
        r#"
        UPDATE properties
        SET value_text = substr(value_text, 1, 19) || 'Z'
        WHERE key IN (SELECT key FROM property_definitions WHERE value_type = 'datetime')
          AND value_text LIKE '____-__-__T__:__:__.%Z'
        "#,
        [],
    )?;
    if updated > 0 {
        tracing::info!("Truncated {updated} datetime property values to whole seconds");
    }
    Ok(())
}

fn migrate_properties(conn: &Connection) -> rusqlite::Result<()> {
    // 1. Backfill item_types if empty
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM item_types", [], |r| r.get(0))?;
    if count == 0 {
        tracing::info!("Backfilling item_types from existing items...");
        conn.execute_batch(
            "INSERT OR IGNORE INTO item_types (item_id, type) SELECT id, 'heading' FROM items WHERE body LIKE '#%';
             INSERT OR IGNORE INTO item_types (item_id, type) SELECT id, 'code' FROM items WHERE body LIKE '```%';
             INSERT OR IGNORE INTO item_types (item_id, type) SELECT id, 'blockquote' FROM items WHERE body LIKE '>%';",
        )?;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM item_types", [], |r| r.get(0))?;
        tracing::info!("Backfilled {n} item_types rows");
    }

    // 2. Backfill plan property for tasks on daily journals
    backfill_task_plan_property(conn)?;

    // 3. Seed property_definitions
    conn.execute_batch(
        "INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('plan', 'date');
         INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('due', 'date');
         INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('begin', 'datetime');
         INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('end', 'datetime');
         INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('cancelled', 'date');
         INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('tag', 'tags');
         INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('zoom', 'number');",
    )?;
    Ok(())
}

/// Reads or creates `versions.json` in the config directory and warns on a schema mismatch.
pub fn check_schema_version(config_dir: &std::path::Path) {
    let version_file = config_dir.join("versions.json");
    tracing::info!("Schema version (embedded): {SCHEMA_VERSION}");
    if version_file.exists() {
        match std::fs::read(&version_file)
            .map_err(anyhow::Error::from)
            .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).map_err(Into::into))
        {
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
        tracing::info!("Schema version file not found. Creating {}", version_file.display());
        let json = serde_json::json!({ "schemaVersion": SCHEMA_VERSION });
        let result = std::fs::create_dir_all(config_dir).and_then(|_| {
            std::fs::write(&version_file, serde_json::to_string_pretty(&json).unwrap_or_default())
        });
        match result {
            Ok(_) => tracing::info!("Wrote {}", version_file.display()),
            Err(e) => tracing::error!("Failed to write versions.json: {e}"),
        }
    }
}
