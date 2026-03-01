package com.petajournal.server.infrastructure.persistence;

import io.quarkus.runtime.StartupEvent;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.Connection;
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
                CREATE TABLE IF NOT EXISTS page_tags (
                    page_id TEXT NOT NULL,
                    tag TEXT NOT NULL,
                    PRIMARY KEY (page_id, tag),
                    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
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
                    status TEXT NOT NULL DEFAULT 'normal' CHECK(status IN ('normal', 'todo', 'running', 'done')),
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

            stmt.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_pages_type_dir_name ON pages(type, dir_path, name)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_modified ON pages(modified_date DESC)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_parent_id ON pages(parent_id)");
            stmt.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_items_page_id_idx ON items(page_id, idx)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_items_parent_id ON items(parent_id)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_items_body ON items(body)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_items_status ON items(status)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_bookmark_groups_sort ON bookmark_groups(sort_order)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_bookmarks_group_sort ON bookmarks(group_id, sort_order)");

            LOG.info("Database initialized");
        } catch (SQLException e) {
            LOG.error("Failed to initialize database", e);
            throw new RuntimeException("Failed to initialize database", e);
        }
    }
}
