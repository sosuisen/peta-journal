package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Bookmark;
import com.petajournal.server.domain.model.BookmarkGroup;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BookmarkRepository;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.*;
import java.time.Instant;
import java.util.*;

@ApplicationScoped
public class SqliteBookmarkRepository implements BookmarkRepository {

    private static final Logger LOG = Logger.getLogger(SqliteBookmarkRepository.class);

    @Inject
    SqliteConnectionProvider connectionProvider;

    @Override
    public List<BookmarkGroup> findAllGroups() {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, name, sort_order, created_at FROM bookmark_groups ORDER BY sort_order")) {
            List<BookmarkGroup> groups = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    groups.add(mapGroup(rs));
                }
            }
            return groups;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find all bookmark groups", e);
        }
    }

    @Override
    public Optional<BookmarkGroup> findGroupById(UUID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, name, sort_order, created_at FROM bookmark_groups WHERE id = ?")) {
            ps.setString(1, id.toString());
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapGroup(rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find bookmark group by id: " + id, e);
        }
    }

    @Override
    public List<Bookmark> findAllBookmarks() {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.group_id, b.target_type, b.target_id, b.sort_order, b.created_at, " +
                     "p.id AS p_id, p.parent_id AS p_parent_id, p.dir_path, p.name, p.type, p.created_date, p.modified_date " +
                     "FROM bookmarks b LEFT JOIN pages p ON b.target_type = 'page' AND b.target_id = p.id " +
                     "ORDER BY b.sort_order")) {
            List<Bookmark> bookmarks = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    bookmarks.add(mapBookmark(conn, rs));
                }
            }
            return bookmarks;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find all bookmarks", e);
        }
    }

    @Override
    public List<Bookmark> findBookmarksByGroupId(UUID groupId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.group_id, b.target_type, b.target_id, b.sort_order, b.created_at, " +
                     "p.id AS p_id, p.parent_id AS p_parent_id, p.dir_path, p.name, p.type, p.created_date, p.modified_date " +
                     "FROM bookmarks b LEFT JOIN pages p ON b.target_type = 'page' AND b.target_id = p.id " +
                     "WHERE b.group_id = ? ORDER BY b.sort_order")) {
            ps.setString(1, groupId.toString());
            List<Bookmark> bookmarks = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    bookmarks.add(mapBookmark(conn, rs));
                }
            }
            return bookmarks;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find bookmarks by group: " + groupId, e);
        }
    }

    @Override
    public Optional<Bookmark> findBookmarkByTarget(String targetType, String targetId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.group_id, b.target_type, b.target_id, b.sort_order, b.created_at, " +
                     "p.id AS p_id, p.parent_id AS p_parent_id, p.dir_path, p.name, p.type, p.created_date, p.modified_date " +
                     "FROM bookmarks b LEFT JOIN pages p ON b.target_type = 'page' AND b.target_id = p.id " +
                     "WHERE b.target_type = ? AND b.target_id = ?")) {
            ps.setString(1, targetType);
            ps.setString(2, targetId);
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapBookmark(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find bookmark by target", e);
        }
    }

    @Override
    public Optional<Bookmark> findBookmarkById(UUID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.group_id, b.target_type, b.target_id, b.sort_order, b.created_at, " +
                     "p.id AS p_id, p.parent_id AS p_parent_id, p.dir_path, p.name, p.type, p.created_date, p.modified_date " +
                     "FROM bookmarks b LEFT JOIN pages p ON b.target_type = 'page' AND b.target_id = p.id " +
                     "WHERE b.id = ?")) {
            ps.setString(1, id.toString());
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapBookmark(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find bookmark by id: " + id, e);
        }
    }

    @Override
    public BookmarkGroup saveGroup(BookmarkGroup group) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "INSERT INTO bookmark_groups (id, name, sort_order, created_at) VALUES (?, ?, ?, ?) " +
                     "ON CONFLICT(id) DO UPDATE SET name = excluded.name, sort_order = excluded.sort_order")) {
            ps.setString(1, group.id().toString());
            ps.setString(2, group.name());
            ps.setString(3, group.sortOrder());
            ps.setLong(4, group.createdAt().toEpochMilli());
            ps.executeUpdate();
            return group;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to save bookmark group", e);
        }
    }

    @Override
    public Bookmark saveBookmark(Bookmark bookmark) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "INSERT INTO bookmarks (id, group_id, target_type, target_id, sort_order, created_at) VALUES (?, ?, ?, ?, ?, ?) " +
                     "ON CONFLICT(id) DO UPDATE SET group_id = excluded.group_id, sort_order = excluded.sort_order")) {
            ps.setString(1, bookmark.id().toString());
            ps.setString(2, bookmark.groupId().toString());
            ps.setString(3, bookmark.targetType());
            ps.setString(4, bookmark.targetId());
            ps.setString(5, bookmark.sortOrder());
            ps.setLong(6, bookmark.createdAt().toEpochMilli());
            ps.executeUpdate();
            return bookmark;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to save bookmark", e);
        }
    }

    @Override
    public void deleteBookmark(UUID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM bookmarks WHERE id = ?")) {
            ps.setString(1, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete bookmark: " + id, e);
        }
    }

    @Override
    public void deleteGroup(UUID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM bookmark_groups WHERE id = ?")) {
            ps.setString(1, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete bookmark group: " + id, e);
        }
    }

    @Override
    public void updateBookmarkSortOrder(UUID id, String sortOrder) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("UPDATE bookmarks SET sort_order = ? WHERE id = ?")) {
            ps.setString(1, sortOrder);
            ps.setString(2, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update bookmark sort order: " + id, e);
        }
    }

    @Override
    public void updateBookmarkGroup(UUID id, UUID newGroupId, String sortOrder) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("UPDATE bookmarks SET group_id = ?, sort_order = ? WHERE id = ?")) {
            ps.setString(1, newGroupId.toString());
            ps.setString(2, sortOrder);
            ps.setString(3, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update bookmark group: " + id, e);
        }
    }

    @Override
    public void updateGroupSortOrder(UUID id, String sortOrder) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("UPDATE bookmark_groups SET sort_order = ? WHERE id = ?")) {
            ps.setString(1, sortOrder);
            ps.setString(2, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update group sort order: " + id, e);
        }
    }

    @Override
    public void updateGroupName(UUID id, String name) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("UPDATE bookmark_groups SET name = ? WHERE id = ?")) {
            ps.setString(1, name);
            ps.setString(2, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update group name: " + id, e);
        }
    }

    private BookmarkGroup mapGroup(ResultSet rs) throws SQLException {
        return new BookmarkGroup(
                UUID.fromString(rs.getString("id")),
                rs.getString("name"),
                rs.getString("sort_order"),
                Instant.ofEpochMilli(rs.getLong("created_at"))
        );
    }

    private Bookmark mapBookmark(Connection conn, ResultSet rs) throws SQLException {
        Page targetPage = null;
        String pId = rs.getString("p_id");
        if (pId != null) {
            targetPage = mapPage(conn, rs, pId);
        }
        return new Bookmark(
                UUID.fromString(rs.getString("id")),
                UUID.fromString(rs.getString("group_id")),
                rs.getString("target_type"),
                rs.getString("target_id"),
                rs.getString("sort_order"),
                Instant.ofEpochMilli(rs.getLong("created_at")),
                targetPage
        );
    }

    private Page mapPage(Connection conn, ResultSet rs, String pageId) throws SQLException {
        String parentIdStr = rs.getString("p_parent_id");
        UUID parentId = parentIdStr != null ? UUID.fromString(parentIdStr) : null;
        return new Page(
                UUID.fromString(pageId),
                parentId,
                rs.getString("dir_path"),
                rs.getString("name"),
                rs.getString("type"),
                Instant.ofEpochMilli(rs.getLong("created_date")),
                Instant.ofEpochMilli(rs.getLong("modified_date"))
        );
    }
}
