package com.petajournal.server.infrastructure.persistence;

import io.quarkus.runtime.Startup;
import jakarta.annotation.PostConstruct;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;

@ApplicationScoped
@Startup
public class DatabaseInitializer {

    private static final Logger LOG = Logger.getLogger(DatabaseInitializer.class);

    @Inject
    SqliteConnectionProvider connectionProvider;

    @PostConstruct
    void init() {
        LOG.info("Initializing database schema...");
        try (Connection conn = connectionProvider.getConnection();
             Statement stmt = conn.createStatement()) {

            stmt.execute("PRAGMA journal_mode=WAL");
            stmt.execute("PRAGMA foreign_keys=ON");

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS pages (
                    id TEXT PRIMARY KEY,
                    dir_path TEXT NOT NULL,
                    file_name TEXT NOT NULL,
                    created_date INTEGER NOT NULL,
                    modified_date INTEGER NOT NULL,
                    properties_json TEXT DEFAULT '{}'
                )
                """);

            stmt.execute("""
                CREATE TABLE IF NOT EXISTS page_titles (
                    page_id TEXT NOT NULL,
                    order_index INTEGER NOT NULL,
                    title TEXT NOT NULL,
                    PRIMARY KEY (page_id, order_index),
                    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
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

            // Indexes
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_dir_file ON pages(dir_path, file_name)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_pages_modified ON pages(modified_date DESC)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_page_titles_title ON page_titles(title)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_blocks_page_id ON blocks(page_id, idx)");
            stmt.execute("CREATE INDEX IF NOT EXISTS idx_blocks_body_link ON blocks(body)");

            LOG.info("Database schema initialized successfully.");
        } catch (SQLException e) {
            throw new RuntimeException("Failed to initialize database", e);
        }
    }
}
