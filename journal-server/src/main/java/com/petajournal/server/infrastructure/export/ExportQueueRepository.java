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
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "INSERT INTO export_queue (page_id, queued_at) VALUES (?, ?) ON CONFLICT(page_id) DO UPDATE SET queued_at = ?")) {
            long now = Instant.now().toEpochMilli();
            ps.setString(1, pageId.toString());
            ps.setLong(2, now);
            ps.setLong(3, now);
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to enqueue export for page: " + pageId, e);
        }
    }

    public List<UUID> dequeueAll() {
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
        } catch (SQLException e) {
            throw new RuntimeException("Failed to dequeue exports", e);
        }
        return pageIds;
    }
}
