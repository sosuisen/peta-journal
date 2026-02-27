package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.PageRepository;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.*;
import java.time.Instant;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.*;

@ApplicationScoped
public class SqlitePageRepository implements PageRepository {

    private static final Logger LOG = Logger.getLogger(SqlitePageRepository.class);

    @Inject
    SqliteConnectionProvider connectionProvider;

    @Override
    public Optional<Page> findById(UUID id) {
        try (Connection conn = connectionProvider.getConnection()) {
            return findByIdInternal(conn, id);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find page by id: " + id, e);
        }
    }

    private Optional<Page> findByIdInternal(Connection conn, UUID id) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement(
                "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages WHERE id = ?")) {
            ps.setString(1, id.toString());
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapPage(conn, rs));
            }
        }
    }

    @Override
    public Optional<Page> findJournal(LocalDate date) {
        String fileName = date.format(DateTimeFormatter.ISO_LOCAL_DATE);

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages " +
                     "WHERE type = 'journal' AND dir_path = '' AND name = ?")) {
            ps.setString(1, fileName);
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapPage(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find journal for date: " + date, e);
        }
    }

    @Override
    public Optional<Page> findMostRecentJournalBefore(LocalDate beforeDate) {
        String fileName = beforeDate.format(DateTimeFormatter.ISO_LOCAL_DATE);

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages " +
                     "WHERE type = 'journal' AND dir_path = '' AND name < ? AND name GLOB '????-??-??' " +
                     "ORDER BY name DESC LIMIT 1")) {
            ps.setString(1, fileName);
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapPage(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find most recent journal before: " + beforeDate, e);
        }
    }

    @Override
    public Optional<Page> findByTypeDirPathFileName(String type, String dirPath, String fileName) {
        LOG.debugv("findByTypeDirPathFileName: type={0}, dirPath=[{1}], fileName={2}", type, dirPath, fileName);
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages " +
                     "WHERE type = ? AND dir_path = ? AND name = ?")) {
            ps.setString(1, type);
            ps.setString(2, dirPath);
            ps.setString(3, fileName);
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) {
                    LOG.debugv("findByTypeDirPathFileName: NOT FOUND");
                    return Optional.empty();
                }
                Page page = mapPage(conn, rs);
                LOG.debugv("findByTypeDirPathFileName: FOUND id={0}", page.id());
                return Optional.of(page);
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find page by type/dirPath/fileName: " + type + ":" + dirPath + "/" + fileName, e);
        }
    }

    @Override
    public List<Page> findRecent(int maxResults, String type) {
        String sql = "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages";
        if (type != null) {
            sql += " WHERE type = ?";
        }
        sql += " ORDER BY modified_date DESC LIMIT ?";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            int paramIndex = 1;
            if (type != null) {
                ps.setString(paramIndex++, type);
            }
            ps.setInt(paramIndex, maxResults > 0 ? maxResults : 10);
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find recent pages", e);
        }
    }

    @Override
    public List<Page> findAll() {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages ORDER BY modified_date DESC")) {
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find all pages", e);
        }
    }

    @Override
    public List<Page> findByTitle(String titleQuery) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date " +
                     "FROM pages WHERE name LIKE ? ORDER BY modified_date DESC LIMIT 20")) {
            ps.setString(1, "%" + titleQuery + "%");
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find pages by title: " + titleQuery, e);
        }
    }



    @Override
    public Page save(Page page) {
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                // Resolve parent_id from dir_path
                UUID parentId = resolveParentId(conn, page.id(), page.dirPath());

                // Upsert page
                try (PreparedStatement ps = conn.prepareStatement(
                        "INSERT INTO pages (id, parent_id, dir_path, name, type, created_date, modified_date) " +
                        "VALUES (?, ?, ?, ?, ?, ?, ?) " +
                        "ON CONFLICT(id) DO UPDATE SET parent_id=?, dir_path=?, name=?, type=?, modified_date=?")) {
                    ps.setString(1, page.id().toString());
                    ps.setString(2, parentId != null ? parentId.toString() : null);
                    ps.setString(3, page.dirPath());
                    ps.setString(4, page.name());
                    ps.setString(5, page.type());
                    ps.setLong(6, page.createdDate().toEpochMilli());
                    ps.setLong(7, page.modifiedDate().toEpochMilli());
                    ps.setString(8, parentId != null ? parentId.toString() : null);
                    ps.setString(9, page.dirPath());
                    ps.setString(10, page.name());
                    ps.setString(11, page.type());
                    ps.setLong(12, page.modifiedDate().toEpochMilli());
                    ps.executeUpdate();
                }

                // Replace tags
                try (PreparedStatement ps = conn.prepareStatement("DELETE FROM page_tags WHERE page_id = ?")) {
                    ps.setString(1, page.id().toString());
                    ps.executeUpdate();
                }
                if (page.tags() != null) {
                    try (PreparedStatement ps = conn.prepareStatement(
                            "INSERT INTO page_tags (page_id, tag) VALUES (?, ?)")) {
                        for (String tag : page.tags()) {
                            ps.setString(1, page.id().toString());
                            ps.setString(2, tag);
                            ps.addBatch();
                        }
                        ps.executeBatch();
                    }
                }

                conn.commit();
                return new Page(page.id(), parentId, page.dirPath(), page.name(), page.type(),
                        page.createdDate(), page.modifiedDate(), page.tags());
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to save page: " + page.id(), e);
        }
    }

    @Override
    public void delete(UUID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM pages WHERE id = ?")) {
            ps.setString(1, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete page: " + id, e);
        }
    }

    @Override
    public List<Page> findByPathPrefix(String query, int maxResults) {
        if (maxResults <= 0) maxResults = 20;
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date " +
                     "FROM pages WHERE (dir_path || name) LIKE ? || '%' " +
                     "ORDER BY modified_date DESC LIMIT ?")) {
            ps.setString(1, query);
            ps.setInt(2, maxResults);
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find pages by path prefix: " + query, e);
        }
    }

    @Override
    public List<Page> findByPathContains(String query, int maxResults) {
        if (maxResults <= 0) maxResults = 20;
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date " +
                     "FROM pages WHERE (dir_path || name) LIKE '%' || ? || '%' " +
                     "ORDER BY modified_date DESC LIMIT ?")) {
            ps.setString(1, query);
            ps.setInt(2, maxResults);
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find pages by path contains: " + query, e);
        }
    }

    @Override
    public List<Page> findIndexPagesByDirSegment(String query, int maxResults) {
        if (maxResults <= 0) maxResults = 20;
        // Match dir_path ending with "query/" — i.e. last segment is query
        // Pattern: %query/ matches "query/" and "dir1/query/"
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, parent_id, dir_path, name, type, created_date, modified_date " +
                     "FROM pages WHERE name = 'index' AND " +
                     "(dir_path = ? || '/' OR dir_path LIKE '%/' || ? || '/') " +
                     "ORDER BY length(dir_path) ASC LIMIT ?")) {
            ps.setString(1, query);
            ps.setString(2, query);
            ps.setInt(3, maxResults);
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find index pages by dir segment: " + query, e);
        }
    }

    @Override
    public List<Page> findBacklinkSources(UUID targetPageId) {
        // Find pages that have blocks containing [[page title]] links to the target page
        try (Connection conn = connectionProvider.getConnection()) {
            // First get the target page's name (used as title)
            String pageName = null;
            try (PreparedStatement ps = conn.prepareStatement("SELECT name FROM pages WHERE id = ?")) {
                ps.setString(1, targetPageId.toString());
                try (ResultSet rs = ps.executeQuery()) {
                    if (rs.next()) {
                        pageName = rs.getString("name");
                    }
                }
            }

            if (pageName == null) return List.of();

            // Find pages with blocks that contain [[pageName]]
            Set<UUID> sourcePageIds = new LinkedHashSet<>();
            String pattern = "%[[" + pageName + "]]%";
            try (PreparedStatement ps = conn.prepareStatement(
                    "SELECT DISTINCT page_id FROM blocks WHERE body LIKE ? AND page_id != ?")) {
                ps.setString(1, pattern);
                ps.setString(2, targetPageId.toString());
                try (ResultSet rs = ps.executeQuery()) {
                    while (rs.next()) {
                        sourcePageIds.add(UUID.fromString(rs.getString("page_id")));
                    }
                }
            }

            List<Page> result = new ArrayList<>();
            for (UUID pageId : sourcePageIds) {
                findByIdInternal(conn, pageId).ifPresent(result::add);
            }
            return result;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find backlink sources for: " + targetPageId, e);
        }
    }

    /**
     * Resolves the parent page ID from a dir_path.
     * "" → null (root)
     * "a/" → search name='a', dir_path=''; fallback: name='index', dir_path='a/'
     * "a/b/" → search name='b', dir_path='a/'; fallback: name='index', dir_path='a/b/'
     * If result is the page's own ID → null (avoid self-reference)
     */
    private UUID resolveParentId(Connection conn, UUID pageId, String dirPath) throws SQLException {
        if (dirPath == null || dirPath.isEmpty()) {
            return null;
        }

        // Remove trailing slash and split: "a/b/" → ["a", "b"]
        String trimmed = dirPath.endsWith("/") ? dirPath.substring(0, dirPath.length() - 1) : dirPath;
        int lastSlash = trimmed.lastIndexOf('/');
        String parentDirPath;
        String lastSegment;
        if (lastSlash < 0) {
            parentDirPath = "";
            lastSegment = trimmed;
        } else {
            parentDirPath = trimmed.substring(0, lastSlash + 1);
            lastSegment = trimmed.substring(lastSlash + 1);
        }

        // Primary lookup: page with name=lastSegment in parentDirPath
        UUID found = findPageId(conn, parentDirPath, lastSegment);
        if (found == null) {
            // Fallback: index page at dirPath
            found = findPageId(conn, dirPath, "index");
        }

        // Avoid self-reference
        if (found != null && found.equals(pageId)) {
            return null;
        }
        return found;
    }

    private UUID findPageId(Connection conn, String dirPath, String name) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement(
                "SELECT id FROM pages WHERE dir_path = ? AND name = ? LIMIT 1")) {
            ps.setString(1, dirPath);
            ps.setString(2, name);
            try (ResultSet rs = ps.executeQuery()) {
                if (rs.next()) {
                    return UUID.fromString(rs.getString("id"));
                }
            }
        }
        return null;
    }

    private Page mapPage(Connection conn, ResultSet rs) throws SQLException {
        UUID id = UUID.fromString(rs.getString("id"));
        String parentIdStr = rs.getString("parent_id");
        UUID parentId = parentIdStr != null ? UUID.fromString(parentIdStr) : null;
        return new Page(
                id,
                parentId,
                rs.getString("dir_path"),
                rs.getString("name"),
                rs.getString("type"),
                Instant.ofEpochMilli(rs.getLong("created_date")),
                Instant.ofEpochMilli(rs.getLong("modified_date")),
                loadTags(conn, id)
        );
    }

    private List<Page> mapPages(Connection conn, PreparedStatement ps) throws SQLException {
        List<Page> pages = new ArrayList<>();
        try (ResultSet rs = ps.executeQuery()) {
            while (rs.next()) {
                pages.add(mapPage(conn, rs));
            }
        }
        return pages;
    }

    private List<String> loadTags(Connection conn, UUID pageId) throws SQLException {
        List<String> tags = new ArrayList<>();
        try (PreparedStatement ps = conn.prepareStatement("SELECT tag FROM page_tags WHERE page_id = ?")) {
            ps.setString(1, pageId.toString());
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    tags.add(rs.getString("tag"));
                }
            }
        }
        return tags;
    }
}
