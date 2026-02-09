package com.petajournal.server.infrastructure.export;

import com.petajournal.server.infrastructure.persistence.SqliteConnectionProvider;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.sql.*;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;

@ApplicationScoped
public class ExportQueueRepository {

    @Inject
    SqliteConnectionProvider connectionProvider;

    public void enqueue(UUID pageId) {
        int maxRetries = 3;
        int retryDelay = 50; // milliseconds

        for (int attempt = 0; attempt < maxRetries; attempt++) {
            try (Connection conn = connectionProvider.getConnection();
                 PreparedStatement ps = conn.prepareStatement(
                         "INSERT INTO export_queue (page_id, queued_at) VALUES (?, ?) ON CONFLICT(page_id) DO UPDATE SET queued_at = ?")) {
                long now = Instant.now().toEpochMilli();
                ps.setString(1, pageId.toString());
                ps.setLong(2, now);
                ps.setLong(3, now);
                ps.executeUpdate();
                return; // Success
            } catch (SQLException e) {
                // Check if it's a lock error and we have retries left
                if (e.getMessage().contains("database is locked") && attempt < maxRetries - 1) {
                    try {
                        Thread.sleep(retryDelay * (attempt + 1)); // exponential backoff
                    } catch (InterruptedException ie) {
                        Thread.currentThread().interrupt();
                        throw new RuntimeException("Failed to enqueue export for page: " + pageId + " (interrupted)", ie);
                    }
                    continue; // retry
                }
                throw new RuntimeException("Failed to enqueue export for page: " + pageId + " after " + (attempt + 1) + " attempts", e);
            }
        }
    }

    public List<UUID> dequeueAll() {
        int maxRetries = 3;
        int retryDelay = 100; // milliseconds

        for (int attempt = 0; attempt < maxRetries; attempt++) {
            List<UUID> pageIds = new ArrayList<>();
            try (Connection conn = connectionProvider.getConnection()) {
                conn.setAutoCommit(false);
                try (Statement stmt = conn.createStatement();
                     ResultSet rs = stmt.executeQuery("SELECT page_id FROM export_queue ORDER BY queued_at LIMIT 10")) {
                    while (rs.next()) {
                        pageIds.add(UUID.fromString(rs.getString("page_id")));
                    }
                }
                if (!pageIds.isEmpty()) {
                    try (PreparedStatement ps = conn.prepareStatement("DELETE FROM export_queue WHERE page_id = ?")) {
                        for (UUID id : pageIds) {
                            ps.setString(1, id.toString());
                            ps.addBatch();
                        }
                        ps.executeBatch();
                    }
                }
                conn.commit();
                return pageIds;
            } catch (SQLException e) {
                // Check if it's a lock error and we have retries left
                if (e.getMessage().contains("database is locked") && attempt < maxRetries - 1) {
                    try {
                        Thread.sleep(retryDelay * (attempt + 1)); // exponential backoff
                    } catch (InterruptedException ie) {
                        Thread.currentThread().interrupt();
                        throw new RuntimeException("Failed to dequeue exports (interrupted)", ie);
                    }
                    continue; // retry
                }
                throw new RuntimeException("Failed to dequeue exports after " + (attempt + 1) + " attempts", e);
            }
        }
        return new ArrayList<>(); // Should not reach here
    }
}
