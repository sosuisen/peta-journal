package com.petajournal.server.infrastructure.persistence;

import io.quarkus.runtime.StartupEvent;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.Connection;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;

@ApplicationScoped
public class DatabaseInitializer {

    private static final Logger LOG = Logger.getLogger(DatabaseInitializer.class);

    @Inject
    SqliteConnectionProvider connectionProvider;

    void init(@Observes @Priority(1) StartupEvent event) {
        try (Connection conn = connectionProvider.getConnection();
             Statement stmt = conn.createStatement()) {

            stmt.execute("PRAGMA journal_mode=WAL");
            stmt.execute("PRAGMA foreign_keys=ON");

            // Drop removed tables
            stmt.execute("DROP TABLE IF EXISTS page_tags");

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS pages (
                    id TEXT PRIMARY KEY,
                    parent_id TEXT,
                    dir_path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    type TEXT NOT NULL DEFAULT 'doc',
                    created_date INTEGER NOT NULL,
                    modified_date INTEGER NOT NULL
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS items (
                    id TEXT PRIMARY KEY,
                    page_id TEXT NOT NULL,
                    parent_id TEXT,
                    created_date INTEGER NOT NULL,
                    modified_date INTEGER NOT NULL,
                    idx TEXT NOT NULL,
                    level INTEGER NOT NULL DEFAULT 0,
                    body TEXT NOT NULL DEFAULT '',
                    html TEXT NOT NULL DEFAULT '',
                    collapsed INTEGER NOT NULL DEFAULT 0,
                    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS item_types (
                    item_id TEXT NOT NULL,
                    type TEXT NOT NULL,
                    PRIMARY KEY (item_id, type),
                    FOREIGN KEY (item_id) REFERENCES items(id) ON DELETE CASCADE
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS export_queue (
                    page_id TEXT PRIMARY KEY,
                    queued_at INTEGER NOT NULL,
                    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS bookmark_groups (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL DEFAULT '',
                    sort_order TEXT NOT NULL,
                    created_at INTEGER NOT NULL
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS bookmarks (
                    id TEXT PRIMARY KEY,
                    group_id TEXT NOT NULL REFERENCES bookmark_groups(id) ON DELETE CASCADE,
                    target_type TEXT NOT NULL CHECK(target_type IN ('item', 'page')),
                    target_id TEXT NOT NULL,
                    sort_order TEXT NOT NULL,
                    created_at INTEGER NOT NULL,
                    UNIQUE(target_type, target_id)
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS property_definitions (
                    key TEXT PRIMARY KEY,
                    value_type TEXT NOT NULL CHECK(value_type IN (
                        'text', 'number', 'checkbox', 'date', 'datetime', 'list', 'tags'
                    ))
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS properties (
                    target_type TEXT NOT NULL CHECK(target_type IN ('item', 'page')),
                    target_id TEXT NOT NULL,
                    key TEXT NOT NULL REFERENCES property_definitions(key),
                    value_text TEXT,
                    value_number REAL,
                    value_checkbox INTEGER,
                    PRIMARY KEY (target_type, target_id, key)
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS property_list_items (
                    id INTEGER PRIMARY KEY,
                    target_type TEXT NOT NULL CHECK(target_type IN ('item', 'page')),
                    target_id TEXT NOT NULL,
                    key TEXT NOT NULL REFERENCES property_definitions(key),
                    sort_order REAL NOT NULL,
                    value TEXT NOT NULL
                )
                """);

            stmt.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_pages_type_dir_name ON pages(type, dir_path, name)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_modified ON pages(modified_date DESC)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_parent_id ON pages(parent_id)");
            stmt.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_items_page_id_idx ON items(page_id, idx)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_items_parent_id ON items(parent_id)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_items_body ON items(body)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_bookmark_groups_sort ON bookmark_groups(sort_order)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_bookmarks_group_sort ON bookmarks(group_id, sort_order)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_props_key_text ON properties(key, value_text)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_props_key_number ON properties(key, value_number)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pli_target ON property_list_items(target_type, target_id, key, sort_order)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pli_value ON property_list_items(key, value)");

            migrateProperties(stmt);

            LOG.info("Database initialized");
        } catch (SQLException e) {
            LOG.error("Failed to initialize database", e);
            throw new RuntimeException("Failed to initialize database", e);
        }
    }

    private void migrateProperties(Statement stmt) throws SQLException {
        // 1. Backfill item_types if empty
        try (ResultSet rs = stmt.executeQuery("SELECT COUNT(*) FROM item_types")) {
            rs.next();
            if (rs.getInt(1) == 0) {
                LOG.info("Backfilling item_types from existing items...");
                stmt.execute("""
                    INSERT OR IGNORE INTO item_types (item_id, type)
                    SELECT id, 'heading' FROM items WHERE body LIKE '#%'
                    """);
                stmt.execute("""
                    INSERT OR IGNORE INTO item_types (item_id, type)
                    SELECT id, 'code' FROM items WHERE body LIKE '```%'
                    """);
                stmt.execute("""
                    INSERT OR IGNORE INTO item_types (item_id, type)
                    SELECT id, 'blockquote' FROM items WHERE body LIKE '>%'
                    """);
                try (ResultSet count = stmt.executeQuery("SELECT COUNT(*) FROM item_types")) {
                    count.next();
                    LOG.infov("Backfilled {0} item_types rows", count.getInt(1));
                }
            }
        }

        // 2. Seed property_definitions
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('plan', 'date')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('due', 'date')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('begin', 'datetime')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('end', 'datetime')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('cancelled', 'date')");

    }
}
