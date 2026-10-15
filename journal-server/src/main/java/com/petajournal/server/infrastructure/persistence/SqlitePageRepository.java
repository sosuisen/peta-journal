package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.PageRepository;
import com.sosuisha.hmtid.HMTID;
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

    /** The columns {@link #mapPage} reads, in the order of the pages table. */
    private static final String SELECT_PAGES =
            "SELECT id, parent_id, dir_path, name, type, created_date, modified_date FROM pages ";

    @Inject
    SqliteConnectionProvider connectionProvider;

    @Override
    public Optional<Page> findById(HMTID id) {
        try (Connection conn = connectionProvider.getConnection()) {
            return findByIdInternal(conn, id);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find page by id: " + id, e);
        }
    }

    private Optional<Page> findByIdInternal(Connection conn, HMTID id) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement(SELECT_PAGES + "WHERE id = ?")) {
            ps.setString(1, id.toString());
            return firstPage(ps);
        }
    }

    @Override
    public Optional<Page> findJournal(LocalDate date) {
        String fileName = date.format(DateTimeFormatter.ISO_LOCAL_DATE);

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     SELECT_PAGES + "WHERE type = 'journal' AND dir_path = '' AND name = ?")) {
            ps.setString(1, fileName);
            return firstPage(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find journal for date: " + date, e);
        }
    }

    @Override
    public Optional<Page> findMostRecentJournalBefore(LocalDate beforeDate) {
        String fileName = beforeDate.format(DateTimeFormatter.ISO_LOCAL_DATE);

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     SELECT_PAGES + "WHERE type = 'journal' AND dir_path = '' AND name < ? AND name GLOB '????-??-??' " +
                     "ORDER BY name DESC LIMIT 1")) {
            ps.setString(1, fileName);
            return firstPage(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find most recent journal before: " + beforeDate, e);
        }
    }

    @Override
    public Optional<Page> findMostRecentWeeklyJournalBefore(String beforeWeek) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     SELECT_PAGES + "WHERE type = 'journal' AND dir_path = '' AND name < ? AND name GLOB '????-W??' " +
                     "ORDER BY name DESC LIMIT 1")) {
            ps.setString(1, beforeWeek);
            return firstPage(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find most recent weekly journal before: " + beforeWeek, e);
        }
    }

    @Override
    public Optional<Page> findByTypeDirPathFileName(String type, String dirPath, String fileName) {
        LOG.debugv("findByTypeDirPathFileName: type={0}, dirPath=[{1}], fileName={2}", type, dirPath, fileName);
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     SELECT_PAGES + "WHERE type = ? AND dir_path = ? AND name = ?")) {
            ps.setString(1, type);
            ps.setString(2, dirPath);
            ps.setString(3, fileName);
            Optional<Page> page = firstPage(ps);
            LOG.debugv("findByTypeDirPathFileName: {0}", page.map(p -> "FOUND id=" + p.id()).orElse("NOT FOUND"));
            return page;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find page by type/dirPath/fileName: " + type + ":" + dirPath + "/" + fileName, e);
        }
    }

    @Override
    public List<Page> findRecent(int maxResults, String type) {
        String sql = SELECT_PAGES;
        if (type != null) {
            sql += "WHERE type = ? ";
        }
        sql += "ORDER BY modified_date DESC LIMIT ?";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            int paramIndex = 1;
            if (type != null) {
                ps.setString(paramIndex++, type);
            }
            ps.setInt(paramIndex, maxResults > 0 ? maxResults : 10);
            return mapPages(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find recent pages", e);
        }
    }

    @Override
    public List<Page> findAll() {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(SELECT_PAGES + "ORDER BY modified_date DESC")) {
            return mapPages(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find all pages", e);
        }
    }

    @Override
    public Page save(Page page) {
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                // Resolve parent_id from dir_path
                HMTID parentId = resolveParentId(conn, page.id(), page.dirPath(), page.name());

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

                conn.commit();
                return new Page(page.id(), parentId, page.dirPath(), page.name(), page.type(),
                        page.createdDate(), page.modifiedDate());
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to save page: " + page.id(), e);
        }
    }

    @Override
    public void rekey(HMTID oldId, HMTID newId) {
        String[] updates = {
                "UPDATE pages SET id = ? WHERE id = ?",
                "UPDATE pages SET parent_id = ? WHERE parent_id = ?",
                "UPDATE items SET page_id = ? WHERE page_id = ?",
                "UPDATE export_queue SET page_id = ? WHERE page_id = ?",
                "UPDATE file_sync_state SET page_id = ? WHERE page_id = ?",
                "UPDATE properties SET target_id = ? WHERE target_type = 'page' AND target_id = ?",
                "UPDATE property_list_items SET target_id = ? WHERE target_type = 'page' AND target_id = ?",
        };
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                // Rows reference each other by page id; check constraints at commit, not per statement
                try (var stmt = conn.createStatement()) {
                    stmt.execute("PRAGMA defer_foreign_keys = ON");
                }
                for (String sql : updates) {
                    try (PreparedStatement ps = conn.prepareStatement(sql)) {
                        ps.setString(1, newId.toString());
                        ps.setString(2, oldId.toString());
                        ps.executeUpdate();
                    }
                }
                conn.commit();
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to rekey page " + oldId + " -> " + newId, e);
        }
    }

    @Override
    public void delete(HMTID id) {
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
                     SELECT_PAGES + "WHERE (dir_path || name) LIKE ? || '%' " +
                     "ORDER BY modified_date DESC LIMIT ?")) {
            ps.setString(1, query);
            ps.setInt(2, maxResults);
            return mapPages(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find pages by path prefix: " + query, e);
        }
    }

    @Override
    public List<Page> findByPathContains(String query, int maxResults) {
        if (maxResults <= 0) maxResults = 20;
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     SELECT_PAGES + "WHERE (dir_path || name) LIKE '%' || ? || '%' " +
                     "ORDER BY modified_date DESC LIMIT ?")) {
            ps.setString(1, query);
            ps.setInt(2, maxResults);
            return mapPages(ps);
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
                     SELECT_PAGES + "WHERE name = 'index' AND " +
                     "(dir_path = ? || '/' OR dir_path LIKE '%/' || ? || '/') " +
                     "ORDER BY length(dir_path) ASC LIMIT ?")) {
            ps.setString(1, query);
            ps.setString(2, query);
            ps.setInt(3, maxResults);
            return mapPages(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find index pages by dir segment: " + query, e);
        }
    }

    @Override
    public List<Page> findBacklinkSources(HMTID targetPageId) {
        // Find pages that have items containing [[page title]] links to the target page
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

            // Find pages with items that contain [[pageName]]
            Set<HMTID> sourcePageIds = new LinkedHashSet<>();
            String pattern = "%[[" + pageName + "]]%";
            try (PreparedStatement ps = conn.prepareStatement(
                    "SELECT DISTINCT page_id FROM items WHERE body LIKE ? AND page_id != ?")) {
                ps.setString(1, pattern);
                ps.setString(2, targetPageId.toString());
                try (ResultSet rs = ps.executeQuery()) {
                    while (rs.next()) {
                        sourcePageIds.add(HMTID.of(rs.getString("page_id")));
                    }
                }
            }

            List<Page> result = new ArrayList<>();
            for (HMTID pageId : sourcePageIds) {
                findByIdInternal(conn, pageId).ifPresent(result::add);
            }
            return result;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find backlink sources for: " + targetPageId, e);
        }
    }

    @Override
    public List<Page> findByParentId(HMTID parentId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(SELECT_PAGES + "WHERE parent_id = ? ORDER BY name")) {
            ps.setString(1, parentId.toString());
            return mapPages(ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find pages by parent_id: " + parentId, e);
        }
    }

    /**
     * Resolves the parent page ID based on dir_path and name.
     */
    private HMTID resolveParentId(Connection conn, HMTID pageId, String dirPath, String name) throws SQLException {
        if ("index".equals(name)) {
            // Index page's parent is the index page one level up
            if (dirPath == null || dirPath.isEmpty()) {
                return null; // root index has no parent
            }
            // "a/b/c/" → trim → "a/b/c" → lastSlash → parentDirPath="a/b/"
            String trimmed = dirPath.endsWith("/") ? dirPath.substring(0, dirPath.length() - 1) : dirPath;
            int lastSlash = trimmed.lastIndexOf('/');
            String parentDirPath = lastSlash < 0 ? "" : trimmed.substring(0, lastSlash + 1);
            return findPageId(conn, parentDirPath, "index");
        } else {
            // Non-index page's parent is the index page at the same dirPath
            String dir = (dirPath == null) ? "" : dirPath;
            HMTID found = findPageId(conn, dir, "index");
            // Safety: avoid self-reference (shouldn't happen for non-index pages)
            if (found != null && found.equals(pageId)) {
                return null;
            }
            return found;
        }
    }

    private HMTID findPageId(Connection conn, String dirPath, String name) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement(
                "SELECT id FROM pages WHERE dir_path = ? AND name = ? LIMIT 1")) {
            ps.setString(1, dirPath);
            ps.setString(2, name);
            try (ResultSet rs = ps.executeQuery()) {
                if (rs.next()) {
                    return HMTID.of(rs.getString("id"));
                }
            }
        }
        return null;
    }

    private Page mapPage(ResultSet rs) throws SQLException {
        String parentIdStr = rs.getString("parent_id");
        HMTID parentId = parentIdStr != null ? HMTID.of(parentIdStr) : null;
        return new Page(
                HMTID.of(rs.getString("id")),
                parentId,
                rs.getString("dir_path"),
                rs.getString("name"),
                rs.getString("type"),
                Instant.ofEpochMilli(rs.getLong("created_date")),
                Instant.ofEpochMilli(rs.getLong("modified_date"))
        );
    }

    /** Runs a prepared {@link #SELECT_PAGES} query and maps its first row, if any. */
    private Optional<Page> firstPage(PreparedStatement ps) throws SQLException {
        try (ResultSet rs = ps.executeQuery()) {
            if (!rs.next()) return Optional.empty();
            return Optional.of(mapPage(rs));
        }
    }

    /** Runs a prepared {@link #SELECT_PAGES} query and maps every row. */
    private List<Page> mapPages(PreparedStatement ps) throws SQLException {
        List<Page> pages = new ArrayList<>();
        try (ResultSet rs = ps.executeQuery()) {
            while (rs.next()) {
                pages.add(mapPage(rs));
            }
        }
        return pages;
    }

}
