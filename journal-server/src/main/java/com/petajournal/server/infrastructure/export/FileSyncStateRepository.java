package com.petajournal.server.infrastructure.export;

import com.petajournal.server.infrastructure.persistence.SqliteConnectionProvider;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.sql.Connection;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

@ApplicationScoped
public class FileSyncStateRepository {

    private static final String COLUMNS =
            "rel_path, page_id, content_hash, file_size, file_mtime, synced_at, status, missing_since";

    @Inject
    SqliteConnectionProvider connectionProvider;

    /** Records that the file at relPath now matches the given content (read or written by us). */
    public void upsertSynced(String relPath, HMTID pageId, String contentHash, long fileSize, long fileMtime) {
        String sql = "INSERT INTO file_sync_state (" + COLUMNS + ") VALUES (?, ?, ?, ?, ?, ?, 'synced', NULL) " +
                "ON CONFLICT(rel_path) DO UPDATE SET page_id=?, content_hash=?, file_size=?, file_mtime=?, " +
                "synced_at=?, status='synced', missing_since=NULL";
        long now = Instant.now().toEpochMilli();
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setString(1, relPath);
            ps.setString(2, pageId.toString());
            ps.setString(3, contentHash);
            ps.setLong(4, fileSize);
            ps.setLong(5, fileMtime);
            ps.setLong(6, now);
            ps.setString(7, pageId.toString());
            ps.setString(8, contentHash);
            ps.setLong(9, fileSize);
            ps.setLong(10, fileMtime);
            ps.setLong(11, now);
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to upsert file sync state: " + relPath, e);
        }
    }

    /** Marks the file as missing. Keeps the original missing_since if already missing. */
    public void markMissing(String relPath) {
        String sql = "UPDATE file_sync_state SET status='missing', synced_at=?, " +
                "missing_since=COALESCE(missing_since, ?) WHERE rel_path = ?";
        long now = Instant.now().toEpochMilli();
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setLong(1, now);
            ps.setLong(2, now);
            ps.setString(3, relPath);
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to mark file missing: " + relPath, e);
        }
    }

    public Optional<FileSyncState> findByPath(String relPath) {
        String sql = "SELECT " + COLUMNS + " FROM file_sync_state WHERE rel_path = ?";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setString(1, relPath);
            try (ResultSet rs = ps.executeQuery()) {
                if (rs.next()) return Optional.of(map(rs));
            }
            return Optional.empty();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find file sync state: " + relPath, e);
        }
    }

    public List<FileSyncState> findByPageId(HMTID pageId) {
        String sql = "SELECT " + COLUMNS + " FROM file_sync_state WHERE page_id = ?";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setString(1, pageId.toString());
            return mapAll(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find file sync state by page: " + pageId, e);
        }
    }

    public List<FileSyncState> findAll() {
        String sql = "SELECT " + COLUMNS + " FROM file_sync_state";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            return mapAll(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to list file sync state", e);
        }
    }

    public List<FileSyncState> findMissing() {
        String sql = "SELECT " + COLUMNS + " FROM file_sync_state WHERE status = 'missing' ORDER BY missing_since";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            return mapAll(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to list missing files", e);
        }
    }

    public void deleteByPath(String relPath) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM file_sync_state WHERE rel_path = ?")) {
            ps.setString(1, relPath);
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete file sync state: " + relPath, e);
        }
    }

    public void deleteByPageId(HMTID pageId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM file_sync_state WHERE page_id = ?")) {
            ps.setString(1, pageId.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete file sync state by page: " + pageId, e);
        }
    }

    private List<FileSyncState> mapAll(PreparedStatement ps) throws SQLException {
        List<FileSyncState> list = new ArrayList<>();
        try (ResultSet rs = ps.executeQuery()) {
            while (rs.next()) list.add(map(rs));
        }
        return list;
    }

    private FileSyncState map(ResultSet rs) throws SQLException {
        long missingSince = rs.getLong("missing_since");
        return new FileSyncState(
                rs.getString("rel_path"),
                HMTID.of(rs.getString("page_id")),
                rs.getString("content_hash"),
                rs.getLong("file_size"),
                rs.getLong("file_mtime"),
                rs.getLong("synced_at"),
                rs.getString("status"),
                rs.wasNull() ? null : missingSince);
    }
}
