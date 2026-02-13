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

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS pages (
                    id TEXT PRIMARY KEY,
                    dir_path TEXT NOT NULL,
                    file_name TEXT NOT NULL,
                    type TEXT NOT NULL DEFAULT 'doc',
                    created_date INTEGER NOT NULL,
                    modified_date INTEGER NOT NULL,
                    properties_json TEXT DEFAULT '{}'
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
                CREATE TABLE IF NOT EXISTS blocks (
                    id TEXT PRIMARY KEY,
                    page_id TEXT NOT NULL,
                    created_date INTEGER NOT NULL,
                    modified_date INTEGER NOT NULL,
                    idx TEXT NOT NULL,
                    level INTEGER NOT NULL DEFAULT 0,
                    body TEXT NOT NULL DEFAULT '',
                    html TEXT NOT NULL DEFAULT '',
                    collapsed INTEGER NOT NULL DEFAULT 0,
                    status TEXT NOT NULL DEFAULT 'normal' CHECK(status IN ('normal', 'todo', 'running', 'done')),
                    properties_json TEXT DEFAULT '{}',
                    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS block_types (
                    block_id TEXT NOT NULL,
                    type TEXT NOT NULL,
                    PRIMARY KEY (block_id, type),
                    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS export_queue (
                    page_id TEXT PRIMARY KEY,
                    queued_at INTEGER NOT NULL,
                    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
                )
                """);

            stmt.execute("DROP INDEX IF EXISTS idx_pages_dir_file");
            stmt.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_pages_type_dir_file ON pages(type, dir_path, file_name)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_modified ON pages(modified_date DESC)");
            stmt.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_blocks_page_id ON blocks(page_id, idx)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_blocks_body_link ON blocks(body)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_blocks_status ON blocks(status)");

            // Migration: Add status column to existing databases
            try {
                stmt.execute("ALTER TABLE blocks ADD COLUMN status TEXT NOT NULL DEFAULT 'normal'");
                LOG.info("Added status column to blocks table");
            } catch (SQLException e) {
                // Column already exists, ignore
            }

            // Migration: Ensure type column is at position 3 (after file_name)
            {
                int typeCid = -1;
                try (Statement checkStmt = conn.createStatement();
                     ResultSet rs = checkStmt.executeQuery(
                             "SELECT cid FROM pragma_table_info('pages') WHERE name = 'type'")) {
                    if (rs.next()) typeCid = rs.getInt(1);
                }
                if (typeCid != 3) {
                    // type column missing or in wrong position — rebuild table
                    stmt.execute("""
                        CREATE TABLE pages_new (
                            id TEXT PRIMARY KEY,
                            dir_path TEXT NOT NULL,
                            file_name TEXT NOT NULL,
                            type TEXT NOT NULL DEFAULT 'doc',
                            created_date INTEGER NOT NULL,
                            modified_date INTEGER NOT NULL,
                            properties_json TEXT DEFAULT '{}'
                        )
                        """);
                    if (typeCid == -1) {
                        // type column didn't exist — derive from dir_path
                        stmt.execute("""
                            INSERT INTO pages_new (id, dir_path, file_name, type, created_date, modified_date, properties_json)
                            SELECT id, '', file_name,
                                   CASE WHEN dir_path = 'journals' THEN 'journal' ELSE 'doc' END,
                                   created_date, modified_date, properties_json
                            FROM pages
                            """);
                    } else {
                        // type column exists but in wrong position — preserve values
                        stmt.execute("""
                            INSERT INTO pages_new (id, dir_path, file_name, type, created_date, modified_date, properties_json)
                            SELECT id, '', file_name, type, created_date, modified_date, properties_json
                            FROM pages
                            """);
                    }
                    stmt.execute("DROP TABLE pages");
                    stmt.execute("ALTER TABLE pages_new RENAME TO pages");
                    stmt.execute("CREATE UNIQUE INDEX IF NOT EXISTS idx_pages_type_dir_file ON pages(type, dir_path, file_name)");
                    stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_modified ON pages(modified_date DESC)");
                    LOG.info("Migrated pages table: type column now at correct position");
                }
            }

            // Migration: Change dir_path from '/' to '' (empty string)
            // Delete newer '' duplicates (keep '/' originals which have the real blocks)
            stmt.execute("""
                DELETE FROM pages WHERE dir_path = '' AND EXISTS (
                    SELECT 1 FROM pages p2 WHERE p2.type = pages.type AND p2.dir_path = '/' AND p2.file_name = pages.file_name
                )
                """);
            stmt.execute("UPDATE pages SET dir_path = '' WHERE dir_path = '/'");
            stmt.execute("UPDATE pages SET dir_path = '' WHERE dir_path IN ('docs', 'journals')");

            LOG.info("Database initialized");
        } catch (SQLException e) {
            LOG.error("Failed to initialize database", e);
            throw new RuntimeException("Failed to initialize database", e);
        }
    }
}
