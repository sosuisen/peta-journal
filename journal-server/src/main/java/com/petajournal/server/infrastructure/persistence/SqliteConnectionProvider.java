package com.petajournal.server.infrastructure.persistence;

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

    @ConfigProperty(name = "petajournal.data.db")
    String dbPath;

    public Connection getConnection() throws SQLException {
        Path path = Path.of(resolveHome(dbPath));
        try {
            Files.createDirectories(path.getParent());
        } catch (IOException e) {
            throw new SQLException("Cannot create database directory: " + path.getParent(), e);
        }
        LOG.debugv("Connecting to SQLite database: {0}", path);
        return DriverManager.getConnection("jdbc:sqlite:" + path);
    }

    private String resolveHome(String path) {
        if (path.startsWith("~")) {
            return System.getProperty("user.home") + path.substring(1);
        }
        return path;
    }
}
