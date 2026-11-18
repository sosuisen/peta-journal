//! Brings the database up to the schema of this server, then seeds what every database has.
//!
//! The schema version is the `user_version` of the database file (see ADR 005). The
//! migrations are a list; the position of each is the version it brings the database to. A
//! migration runs once, in a transaction of its own that also writes the new version, so a
//! crash leaves no half-done version. A migration is never changed once it is in `main`: a
//! mistake is put right by the next one.
//!
//! The seeds (the root index page, the system keys) are no migrations: they run on every
//! start and keep the database as the server expects it.

use anyhow::Context;
use rusqlite::{params, Connection};

use crate::domain::id_generator::IdGenerator;
use crate::domain::nfc::to_nfc;
use crate::domain::number_text::format_number;

use super::db::Db;

/// The migrations, in the order they run. The version of a database is the number of them
/// it has had.
const MIGRATIONS: &[fn(&Connection) -> rusqlite::Result<()>] = &[
    create_tables,
    create_deleted_items,
    put_names_in_nfc,
    add_last_synced,
    drop_item_asts,
    use_one_value_column,
    add_definition_dates,
];

pub fn initialize(db: &Db, ids: &IdGenerator) -> anyhow::Result<()> {
    let conn = db.conn()?;
    run(&conn, ids).context("Failed to initialize database")?;
    tracing::info!("Database initialized");
    Ok(())
}

fn run(conn: &Connection, ids: &IdGenerator) -> anyhow::Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(conn)?;
    ensure_root_index(conn, ids)?;
    seed_system_keys(conn)?;
    type_user_keys_by_name(conn)?;
    Ok(())
}

/// Gives each user definition the type its name gives (ADR 047). A device before ADR 047
/// could define `rating` as a number; its values stay as they are, as text. A list and a
/// scalar keep their types, as their values are in other tables; ADR 046 already made the
/// names of the lists end with `_list`.
fn type_user_keys_by_name(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt =
        conn.prepare("SELECT key, value_type FROM property_definitions WHERE is_system = 0")?;
    let definitions = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (key, value_type) in definitions {
        let by_name = property_names::type_of(&key).as_str();
        if by_name != value_type && by_name != "list" && value_type != "list" {
            conn.execute(
                "UPDATE property_definitions SET value_type = ? WHERE key = ?",
                rusqlite::params![by_name, key],
            )?;
        }
    }
    Ok(())
}

/// Runs the migrations the database has not had yet. A database of a newer server is
/// refused: this server does not know its schema.
fn migrate(conn: &Connection) -> anyhow::Result<()> {
    let version = user_version(conn)?;
    let known = MIGRATIONS.len();
    if version > known {
        anyhow::bail!(
            "The database is at schema version {version}, which is newer than the version \
             {known} of this server. Update the server."
        );
    }
    for (i, migration) in MIGRATIONS.iter().enumerate().skip(version) {
        let next = i + 1;
        tracing::info!("Migrating the database to schema version {next}...");
        let tx = conn.unchecked_transaction()?;
        migration(&tx)?;
        tx.pragma_update(None, "user_version", next as i64)?;
        tx.commit()?;
    }
    Ok(())
}

fn user_version(conn: &Connection) -> rusqlite::Result<usize> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    Ok(version as usize)
}

/// Version 7: what the sync of the definitions through `settings/properties.json` needs
/// (ADR 046). Each definition has the date it was defined, and may be provisional: made here
/// only to read values that came without a definition. The definitions deleted here are kept
/// with the date, as `deleted_items` keeps the items. The user definitions there are now
/// are real ones of date 0, so a definition made on any device later wins over them.
fn add_definition_dates(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "ALTER TABLE property_definitions ADD COLUMN modified INTEGER NOT NULL DEFAULT 0;
         ALTER TABLE property_definitions ADD COLUMN provisional INTEGER NOT NULL DEFAULT 0;
         CREATE TABLE deleted_property_definitions (
             key TEXT PRIMARY KEY,
             deleted_date INTEGER NOT NULL
         );",
    )
}

/// Version 6: one column for the value of a property, as the text written in the files
/// (ADR 046). A type is how the text is read, so the value stays when the type changes. The
/// numbers and checkboxes come in as the export writes them.
fn use_one_value_column(conn: &Connection) -> rusqlite::Result<()> {
    let numbers: Vec<(i64, f64)> = conn
        .prepare("SELECT rowid, value_number FROM properties WHERE value_number IS NOT NULL")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (rowid, number) in numbers {
        conn.execute(
            "UPDATE properties SET value_text = ? WHERE rowid = ?",
            params![format_number(number), rowid],
        )?;
    }
    conn.execute_batch(
        "UPDATE properties SET value_text = CASE WHEN value_checkbox != 0 THEN 'true' ELSE 'false' END
             WHERE value_checkbox IS NOT NULL;
         DROP INDEX idx_props_key_number;
         DROP INDEX idx_props_key_text;
         ALTER TABLE properties DROP COLUMN value_number;
         ALTER TABLE properties DROP COLUMN value_checkbox;
         ALTER TABLE properties RENAME COLUMN value_text TO value;
         CREATE INDEX idx_props_key_value ON properties(key, value);",
    )
}

/// Version 5: no AST of an item. The journal parses a body when it needs what is in it, and
/// the REST API makes the AST of an item from its body (ADR 041).
fn drop_item_asts(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch("ALTER TABLE items DROP COLUMN ast;")
}

/// Version 4: until when each file and its page were in step. It was the mtime of the file
/// before, so the rows start with that.
fn add_last_synced(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "ALTER TABLE file_sync_state ADD COLUMN last_synced INTEGER NOT NULL DEFAULT 0;
         UPDATE file_sync_state SET last_synced = file_mtime;",
    )
}

/// Version 3: the names of pages and their files in NFC, see ADR 007. Rows are only updated.
/// Deleting a page would delete its items too. `OR IGNORE` leaves a row as it is when its NFC
/// name is taken, as a failure here would keep the server from starting.
fn put_names_in_nfc(conn: &Connection) -> rusqlite::Result<()> {
    let pages: Vec<(String, String, String)> = conn
        .prepare("SELECT id, dir_path, name FROM pages")?
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, dir_path, name) in pages {
        conn.execute(
            "UPDATE OR IGNORE pages SET dir_path = ?, name = ? WHERE id = ?",
            params![to_nfc(&dir_path), to_nfc(&name), id],
        )?;
    }
    let paths: Vec<String> = conn
        .prepare("SELECT rel_path FROM file_sync_state")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    for path in paths {
        conn.execute(
            "UPDATE OR IGNORE file_sync_state SET rel_path = ? WHERE rel_path = ?",
            params![to_nfc(&path), path],
        )?;
    }
    Ok(())
}

/// Version 2: the items deleted here, so that taking in a file can tell them from items
/// this device has not seen yet.
fn create_deleted_items(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE deleted_items (id TEXT PRIMARY KEY, deleted_date INTEGER NOT NULL);",
    )
}

/// Version 1: the tables and the indexes.
fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        r#"
        CREATE TABLE pages (
            id TEXT PRIMARY KEY,
            parent_id TEXT,
            dir_path TEXT NOT NULL,
            name TEXT NOT NULL,
            type TEXT NOT NULL DEFAULT 'doc',
            created_date INTEGER NOT NULL,
            modified_date INTEGER NOT NULL
        );

        CREATE TABLE items (
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

        CREATE TABLE item_types (
            item_id TEXT NOT NULL,
            type TEXT NOT NULL,
            PRIMARY KEY (item_id, type),
            FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE
        );

        CREATE TABLE export_queue (
            page_id TEXT PRIMARY KEY,
            queued_at INTEGER NOT NULL,
            FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
        );

        CREATE TABLE file_sync_state (
            rel_path TEXT PRIMARY KEY,
            page_id TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            file_size INTEGER NOT NULL,
            file_mtime INTEGER NOT NULL,
            synced_at INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'synced' CHECK(status IN ('synced', 'missing')),
            missing_since INTEGER
        );
        CREATE INDEX idx_fss_page_id ON file_sync_state(page_id);

        CREATE TABLE property_definitions (
            key TEXT PRIMARY KEY,
            value_type TEXT NOT NULL CHECK(value_type IN (
                'text', 'number', 'checkbox', 'date', 'datetime', 'bounds', 'list', 'tags'
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

        CREATE UNIQUE INDEX idx_pages_type_dir_name ON pages(type, dir_path, name);
        CREATE INDEX idx_pages_modified ON pages(modified_date DESC);
        CREATE INDEX idx_pages_parent_id ON pages(parent_id);
        CREATE UNIQUE INDEX idx_items_page_id_idx ON items(page_id, idx);
        CREATE INDEX idx_items_parent_id ON items(parent_id);
        CREATE INDEX idx_items_body ON items(body);
        CREATE INDEX idx_items_transclude ON items(transclude_item_id) WHERE transclude_item_id IS NOT NULL;
        CREATE INDEX idx_props_key_text ON properties(key, value_text);
        CREATE INDEX idx_props_key_number ON properties(key, value_number);
        CREATE INDEX idx_pli_target ON property_list_items(target_type, target_id, key, sort_order);
        CREATE INDEX idx_pli_value ON property_list_items(key, value);
        "#,
    )
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
    let now = ids.now_millis();
    conn.execute(
        "INSERT INTO pages (id, parent_id, dir_path, name, type, created_date, modified_date) \
         VALUES (?, NULL, '', 'index', 'doc', ?, ?)",
        params![root_id, now, now],
    )?;
    let item_id = ids.generate_item_id().to_string();
    conn.execute(
        "INSERT INTO items (id, page_id, parent_id, created_date, modified_date, idx, level, body, collapsed) \
         VALUES (?, ?, NULL, ?, ?, 'a0', 1, '', 0)",
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

#[cfg(test)]
mod tests {
    use super::*;

    fn count(conn: &Connection, sql: &str) -> i64 {
        conn.query_row(sql, [], |r| r.get(0)).unwrap()
    }

    // 新しいデータベースは、すべてのマイグレーションを通り、最新の版になる。
    #[test]
    fn a_new_database_goes_through_every_migration_to_the_latest_version() {
        let conn = Connection::open_in_memory().unwrap();

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM pages WHERE dir_path = '' AND name = 'index'"
            ),
            1
        );
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_definitions WHERE is_system = 1"
            ),
            SYSTEM_KEYS.len() as i64
        );
    }

    // 最新の版のデータベースを開き直しても、マイグレーションは走らない。
    #[test]
    fn a_database_at_the_latest_version_is_opened_again_without_a_migration() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        conn.execute(
            "INSERT INTO property_definitions (key, value_type) VALUES ('owner', 'text')",
            [],
        )
        .unwrap();

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM property_definitions WHERE key = 'owner'"
            ),
            1
        );
    }

    // 起動するたびに、名前と型の合わないユーザー定義は、名前から決まる型になる。ADR 047より前の端末で定義されたキーである。
    #[test]
    fn a_user_definition_whose_type_does_not_fit_its_name_takes_the_type_of_its_name_on_start() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        conn.execute(
            "INSERT INTO property_definitions (key, value_type) VALUES ('rating', 'number'), ('score_n', 'text')",
            [],
        )
        .unwrap();

        run(&conn, &IdGenerator::default()).unwrap();

        let type_of = |key: &str| -> String {
            conn.query_row(
                "SELECT value_type FROM property_definitions WHERE key = ?",
                [key],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(type_of("rating"), "text");
        assert_eq!(type_of("score_n"), "number");
    }

    // 起動しても、リストとスカラーの間では型を変えない。値は別の表にあるからである。ADR 046より前の端末では、名前が_listで終わらないリストのキーがありうる。
    #[test]
    fn a_list_and_a_scalar_keep_their_types_on_start() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        conn.execute(
            "INSERT INTO property_definitions (key, value_type) VALUES ('members', 'list'), ('owner_list', 'text')",
            [],
        )
        .unwrap();

        run(&conn, &IdGenerator::default()).unwrap();

        let type_of = |key: &str| -> String {
            conn.query_row(
                "SELECT value_type FROM property_definitions WHERE key = ?",
                [key],
                |r| r.get(0),
            )
            .unwrap()
        };
        assert_eq!(type_of("members"), "list");
        assert_eq!(type_of("owner_list"), "text");
    }

    // 版1のデータベースを開くと、消した項目の表ができ、版が上がる。
    #[test]
    fn a_database_at_version_1_gets_the_table_of_deleted_items() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        conn.execute_batch("DROP TABLE deleted_items").unwrap();
        set_version(&conn, 1);

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM deleted_items"), 0);
    }

    fn add_page_with_file(conn: &Connection, id: &str, dir_path: &str, name: &str) {
        conn.execute(
            "INSERT INTO pages (id, dir_path, name, type, created_date, modified_date) \
             VALUES (?, ?, ?, 'doc', 1, 2)",
            params![id, dir_path, name],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO items (id, page_id, created_date, modified_date, idx, body) \
             VALUES (?, ?, 1, 2, 'a0', ?)",
            params![format!("item-{id}"), id, name],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO file_sync_state (rel_path, page_id, content_hash, file_size, file_mtime, synced_at) \
             VALUES (?, ?, 'hash', 1, 1, 1)",
            params![format!("docs/{dir_path}{name}.md"), id],
        )
        .unwrap();
    }

    /// Puts a database of the latest version back to an older one, of 1 to 5. The value
    /// columns of the properties before version 6 come back. The column of version 5 comes
    /// back for a version below 5, as its migration drops it. The column of version 4 goes
    /// for a version below 4, as its migration cannot add a column that is there.
    fn set_version(conn: &Connection, version: i64) {
        if version < 7 {
            conn.execute_batch(
                "ALTER TABLE property_definitions DROP COLUMN modified;
                 ALTER TABLE property_definitions DROP COLUMN provisional;
                 DROP TABLE deleted_property_definitions;",
            )
            .unwrap();
        }
        if version < 6 {
            conn.execute_batch(
                "DROP INDEX idx_props_key_value;
                 ALTER TABLE properties RENAME COLUMN value TO value_text;
                 ALTER TABLE properties ADD COLUMN value_number REAL;
                 ALTER TABLE properties ADD COLUMN value_checkbox INTEGER;
                 CREATE INDEX idx_props_key_text ON properties(key, value_text);
                 CREATE INDEX idx_props_key_number ON properties(key, value_number);",
            )
            .unwrap();
        }
        if version < 5 {
            conn.execute_batch("ALTER TABLE items ADD COLUMN ast TEXT NOT NULL DEFAULT ''")
                .unwrap();
        }
        if version < 4 {
            conn.execute_batch("ALTER TABLE file_sync_state DROP COLUMN last_synced")
                .unwrap();
        }
        conn.pragma_update(None, "user_version", version).unwrap();
    }

    // 版5のデータベースを開くと、数と真偽値のプロパティの値は、書き出しと同じ形の文字列としてvalueの列に移り、値の列はvalueだけになる。
    #[test]
    fn a_database_at_version_5_moves_numbers_and_checkboxes_into_the_one_value_column_as_text() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        set_version(&conn, 5);
        conn.execute_batch(
            "INSERT INTO property_definitions (key, value_type) VALUES
                 ('score', 'number'), ('done', 'checkbox'), ('owner', 'text');
             INSERT INTO properties (target_type, target_id, key, value_text, value_number, value_checkbox) VALUES
                 ('page', 'p1', 'score', NULL, 12.0, NULL),
                 ('page', 'p2', 'score', NULL, 1.5, NULL),
                 ('item', 'i1', 'done', NULL, NULL, 1),
                 ('item', 'i2', 'done', NULL, NULL, 0),
                 ('item', 'i3', 'owner', 'me', NULL, NULL);",
        )
        .unwrap();

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        let columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('properties') ORDER BY cid")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(columns, ["target_type", "target_id", "key", "value"]);
        let value = |id: &str| {
            text(
                &conn,
                "SELECT value FROM properties WHERE target_id = ?",
                id,
            )
        };
        assert_eq!(value("p1"), "12");
        assert_eq!(value("p2"), "1.5");
        assert_eq!(value("i1"), "true");
        assert_eq!(value("i2"), "false");
        assert_eq!(value("i3"), "me");
    }

    // 版6のデータベースを開くと、定義は日時と仮の定義かどうかを持ち、消した定義の表ができる。今あるユーザー定義は、日時0の本物の定義になる。
    #[test]
    fn a_database_at_version_6_keeps_its_user_definitions_as_real_ones_of_date_0() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        set_version(&conn, 6);
        conn.execute(
            "INSERT INTO property_definitions (key, value_type) VALUES ('owner', 'text')",
            [],
        )
        .unwrap();

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        let (modified, provisional): (i64, i64) = conn
            .query_row(
                "SELECT modified, provisional FROM property_definitions WHERE key = 'owner'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((modified, provisional), (0, 0));
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM deleted_property_definitions"),
            0
        );
    }

    // 版4のデータベースを開くと、項目のASTの列がなくなり、本文はそのままである。
    #[test]
    fn a_database_at_version_4_loses_the_ast_of_its_items_and_keeps_their_bodies() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        set_version(&conn, 4);
        add_page_with_file(&conn, "p1", "", "a");

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM pragma_table_info('items') WHERE name = 'ast'"
            ),
            0
        );
        assert_eq!(
            text(&conn, "SELECT body FROM items WHERE id = ?", "item-p1"),
            "a"
        );
    }

    // 版3のデータベースを開くと、前回の同期の日時は、ファイルの更新時刻で始まる。
    #[test]
    fn a_database_at_version_3_starts_the_last_sync_of_a_file_at_its_mtime() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        set_version(&conn, 3);
        add_page_with_file(&conn, "p1", "", "a");
        conn.execute("UPDATE file_sync_state SET file_mtime = 7", [])
            .unwrap();

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        assert_eq!(count(&conn, "SELECT last_synced FROM file_sync_state"), 7);
    }

    fn text(conn: &Connection, sql: &str, id: &str) -> String {
        conn.query_row(sql, [id], |r| r.get(0)).unwrap()
    }

    // 版2のデータベースを開くと、NFDのページ名と同期パスはNFCになり、項目、日付、idはそのままである。
    #[test]
    fn a_database_at_version_2_gets_nfc_page_names_and_paths_and_keeps_the_rest() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        add_page_with_file(&conn, "p1", "\u{30c6}\u{3099}/", "\u{30ab}\u{3099}");
        set_version(&conn, 2);

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        let name_of = "SELECT name FROM pages WHERE id = ?";
        assert_eq!(text(&conn, name_of, "p1"), "\u{30ac}");
        let dir_of = "SELECT dir_path FROM pages WHERE id = ?";
        assert_eq!(text(&conn, dir_of, "p1"), "\u{30c7}/");
        let path_of = "SELECT rel_path FROM file_sync_state WHERE page_id = ?";
        assert_eq!(text(&conn, path_of, "p1"), "docs/\u{30c7}/\u{30ac}.md");
        // The body is what the user wrote, so it keeps its form
        let body_of = "SELECT body FROM items WHERE page_id = ?";
        assert_eq!(text(&conn, body_of, "p1"), "\u{30ab}\u{3099}");
        assert_eq!(
            count(&conn, "SELECT modified_date FROM pages WHERE id = 'p1'"),
            2
        );
    }

    // NFCにすると別の行と同じ名前になる行は、項目も含めてそのまま残り、データベースは開ける。
    #[test]
    fn a_row_that_would_take_the_name_of_another_row_stays_as_it_is() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        add_page_with_file(&conn, "p1", "", "\u{30ac}");
        add_page_with_file(&conn, "p2", "", "\u{30ab}\u{3099}");
        set_version(&conn, 2);

        run(&conn, &IdGenerator::default()).unwrap();

        assert_eq!(user_version(&conn).unwrap(), MIGRATIONS.len());
        let name_of = "SELECT name FROM pages WHERE id = ?";
        assert_eq!(text(&conn, name_of, "p1"), "\u{30ac}");
        assert_eq!(text(&conn, name_of, "p2"), "\u{30ab}\u{3099}");
        let path_of = "SELECT rel_path FROM file_sync_state WHERE page_id = ?";
        assert_eq!(text(&conn, path_of, "p1"), "docs/\u{30ac}.md");
        assert_eq!(text(&conn, path_of, "p2"), "docs/\u{30ab}\u{3099}.md");
        assert_eq!(
            count(
                &conn,
                "SELECT COUNT(*) FROM items WHERE page_id IN ('p1', 'p2')"
            ),
            2
        );
    }

    // 新しいサーバーのデータベースは、開かれない。
    #[test]
    fn a_database_of_a_newer_server_is_refused() {
        let conn = Connection::open_in_memory().unwrap();
        run(&conn, &IdGenerator::default()).unwrap();
        conn.pragma_update(None, "user_version", MIGRATIONS.len() as i64 + 1)
            .unwrap();

        assert!(run(&conn, &IdGenerator::default()).is_err());
    }

    // 途中で失敗したマイグレーションは、版を進めない。
    #[test]
    fn a_migration_that_fails_leaves_the_version_as_it_was() {
        let conn = Connection::open_in_memory().unwrap();
        // A table in the way of the first migration
        conn.execute("CREATE TABLE pages (id TEXT)", []).unwrap();

        assert!(run(&conn, &IdGenerator::default()).is_err());

        assert_eq!(user_version(&conn).unwrap(), 0);
    }
}
