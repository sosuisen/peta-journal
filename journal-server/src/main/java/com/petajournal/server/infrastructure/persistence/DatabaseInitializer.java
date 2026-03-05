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
            stmt.execute("DROP TABLE IF EXISTS bookmarks");
            stmt.execute("DROP TABLE IF EXISTS bookmark_groups");

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
                    ast TEXT NOT NULL DEFAULT '',
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
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_props_key_text ON properties(key, value_text)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_props_key_number ON properties(key, value_number)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pli_target ON property_list_items(target_type, target_id, key, sort_order)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pli_value ON property_list_items(key, value)");

            migrateHtmlToAst(stmt);
            migrateProperties(stmt);
            migrateRootIndex(stmt);

            LOG.info("Database initialized");
        } catch (SQLException e) {
            LOG.error("Failed to initialize database", e);
            throw new RuntimeException("Failed to initialize database", e);
        }
    }

    private void migrateHtmlToAst(Statement stmt) throws SQLException {
        // Rename html column to ast if the old column still exists
        try (ResultSet rs = stmt.executeQuery("PRAGMA table_info(items)")) {
            while (rs.next()) {
                if ("html".equals(rs.getString("name"))) {
                    LOG.info("Renaming items.html column to items.ast...");
                    stmt.execute("ALTER TABLE items RENAME COLUMN html TO ast");
                    LOG.info("Column renamed successfully.");
                    return;
                }
            }
        }
    }

    private void backfillTaskPlanProperty(Statement stmt) throws SQLException {
        // Add plan property to all task items on daily journal pages (YYYY-MM-DD) that don't have one yet
        String countSql = """
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
            """;
        try (ResultSet rs = stmt.executeQuery(countSql)) {
            rs.next();
            int count = rs.getInt(1);
            if (count > 0) {
                LOG.infov("Backfilling plan property for {0} task items on daily journals...", count);
                stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('plan', 'date')");
                stmt.execute("""
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
                      )
                    """);
                LOG.info("Plan property backfill complete");
            }
        }
    }

    private void migrateRootIndex(Statement stmt) throws SQLException {
        // Create root index page if it doesn't exist
        try (var rs = stmt.executeQuery(
                "SELECT id FROM pages WHERE dir_path = '' AND name = 'index' LIMIT 1")) {
            if (rs.next()) {
                return;
            }
        }

        LOG.info("Creating root index page...");
        String rootId = java.util.UUID.randomUUID().toString();
        long now = java.time.Instant.now().toEpochMilli();
        try (var ps = stmt.getConnection().prepareStatement(
                "INSERT INTO pages (id, parent_id, dir_path, name, type, created_date, modified_date) "
                + "VALUES (?, NULL, '', 'index', 'doc', ?, ?)")) {
            ps.setString(1, rootId);
            ps.setLong(2, now);
            ps.setLong(3, now);
            ps.executeUpdate();
        }
        // Create initial empty item for the root index page
        String itemId = com.petajournal.server.domain.ItemIdGenerator.generate().toString();
        try (var ps = stmt.getConnection().prepareStatement(
                "INSERT INTO items (id, page_id, parent_id, created_date, modified_date, idx, level, body, ast, collapsed) "
                + "VALUES (?, ?, NULL, ?, ?, 'a0', 1, '', '', 0)")) {
            ps.setString(1, itemId);
            ps.setString(2, rootId);
            ps.setLong(3, now);
            ps.setLong(4, now);
            ps.executeUpdate();
        }
        LOG.info("Root index page created: " + rootId);
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

        // 2. Backfill plan property for tasks on daily journals
        backfillTaskPlanProperty(stmt);

        // 3. Seed property_definitions
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('plan', 'date')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('due', 'date')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('begin', 'datetime')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('end', 'datetime')");
        stmt.execute("INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES ('cancelled', 'date')");

    }
}
