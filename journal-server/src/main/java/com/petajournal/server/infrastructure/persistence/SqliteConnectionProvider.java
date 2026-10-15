package com.petajournal.server.infrastructure.persistence;

import jakarta.inject.Inject;
import jakarta.enterprise.context.ApplicationScoped;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.SQLException;

@ApplicationScoped
public class SqliteConnectionProvider {

    private static final Logger LOG = Logger.getLogger(SqliteConnectionProvider.class);

    private final String dbPath;

    @Inject
    public SqliteConnectionProvider(@ConfigProperty(name = "petajournal.data.db") String dbPath) {
        this.dbPath = dbPath;
    }

    public Connection getConnection() throws SQLException {
        Path path = Path.of(resolveHome(dbPath));
        boolean dbExists = Files.exists(path);

        try {
            // Ensure parent directory exists
            if (path.getParent() != null) {
                Files.createDirectories(path.getParent());
                LOG.debugv("Database directory: {0}", path.getParent());
            }
        } catch (IOException e) {
            throw new SQLException("Cannot create database directory: " + path.getParent(), e);
        }

        if (!dbExists) {
            LOG.infov("Creating new SQLite database at: {0}", path);
        } else {
            LOG.debugv("Connecting to existing SQLite database: {0}", path);
        }

        // SQLite automatically creates the database file if it doesn't exist
        // Enable WAL mode for better concurrency and set busy timeout
        // transaction_mode=IMMEDIATE: acquire write lock at BEGIN, not at first write.
        // This prevents SQLITE_BUSY_SNAPSHOT when concurrent threads write via separate connections.
        String url = "jdbc:sqlite:" + path + "?journal_mode=WAL&busy_timeout=30000&transaction_mode=IMMEDIATE";
        Connection conn = DriverManager.getConnection(url);

        // Set additional pragmas for better concurrency
        try (var stmt = conn.createStatement()) {
            stmt.execute("PRAGMA synchronous = NORMAL");
            stmt.execute("PRAGMA temp_store = MEMORY");
        }

        return conn;
    }

    private String resolveHome(String path) {
        if (path.startsWith("~")) {
            return System.getProperty("user.home") + path.substring(1);
        }
        return path;
    }
}
