package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.PageTitle;
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
                "SELECT id, dir_path, file_name, created_date, modified_date, properties_json FROM pages WHERE id = ?")) {
            ps.setString(1, id.toString());
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapPage(conn, rs));
            }
        }
    }

    @Override
    public Optional<Page> findJournal(LocalDate date) {
        String dirPath = "journals";
        String fileName = date.format(DateTimeFormatter.ISO_LOCAL_DATE);

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, dir_path, file_name, created_date, modified_date, properties_json FROM pages " +
                     "WHERE dir_path = ? AND file_name = ?")) {
            ps.setString(1, dirPath);
            ps.setString(2, fileName);
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapPage(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find journal for date: " + date, e);
        }
    }

    @Override
    public List<Page> findByTag(String tag) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT p.id, p.dir_path, p.file_name, p.created_date, p.modified_date, p.properties_json " +
                     "FROM pages p JOIN page_tags t ON p.id = t.page_id WHERE t.tag = ? ORDER BY p.modified_date DESC")) {
            ps.setString(1, tag);
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find pages by tag: " + tag, e);
        }
    }

    @Override
    public Optional<Page> findByDirPathAndFileName(String dirPath, String fileName) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, dir_path, file_name, created_date, modified_date, properties_json FROM pages WHERE dir_path = ? AND file_name = ?")) {
            ps.setString(1, dirPath);
            ps.setString(2, fileName);
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapPage(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find page by dirPath/fileName: " + dirPath + "/" + fileName, e);
        }
    }

    @Override
    public List<Page> findRecent(int maxResults) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, dir_path, file_name, created_date, modified_date, properties_json FROM pages ORDER BY modified_date DESC LIMIT ?")) {
            ps.setInt(1, maxResults > 0 ? maxResults : 20);
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find recent pages", e);
        }
    }

    @Override
    public List<Page> findAll() {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, dir_path, file_name, created_date, modified_date, properties_json FROM pages ORDER BY modified_date DESC")) {
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find all pages", e);
        }
    }

    @Override
    public List<Page> findByTitle(String titleQuery) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT DISTINCT p.id, p.dir_path, p.file_name, p.created_date, p.modified_date, p.properties_json " +
                     "FROM pages p JOIN page_titles t ON p.id = t.page_id WHERE t.title LIKE ? ORDER BY p.modified_date DESC LIMIT 20")) {
            ps.setString(1, "%" + titleQuery + "%");
            return mapPages(conn, ps);
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find pages by title: " + titleQuery, e);
        }
    }

    @Override
    public Optional<Page> findByExactTitle(String title) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT p.id, p.dir_path, p.file_name, p.created_date, p.modified_date, p.properties_json " +
                     "FROM pages p JOIN page_titles t ON p.id = t.page_id WHERE t.title = ? LIMIT 1")) {
            ps.setString(1, title);
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapPage(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find page by exact title: " + title, e);
        }
    }

    @Override
    public Page save(Page page) {
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                // Upsert page
                try (PreparedStatement ps = conn.prepareStatement(
                        "INSERT INTO pages (id, dir_path, file_name, created_date, modified_date, properties_json) " +
                        "VALUES (?, ?, ?, ?, ?, ?) " +
                        "ON CONFLICT(id) DO UPDATE SET dir_path=?, file_name=?, modified_date=?, properties_json=?")) {
                    ps.setString(1, page.id().toString());
                    ps.setString(2, page.dirPath());
                    ps.setString(3, page.fileName());
                    ps.setLong(4, page.createdDate().toEpochMilli());
                    ps.setLong(5, page.modifiedDate().toEpochMilli());
                    ps.setString(6, page.propertiesJson());
                    ps.setString(7, page.dirPath());
                    ps.setString(8, page.fileName());
                    ps.setLong(9, page.modifiedDate().toEpochMilli());
                    ps.setString(10, page.propertiesJson());
                    ps.executeUpdate();
                }

                // Replace titles
                try (PreparedStatement ps = conn.prepareStatement("DELETE FROM page_titles WHERE page_id = ?")) {
                    ps.setString(1, page.id().toString());
                    ps.executeUpdate();
                }
                if (page.titles() != null) {
                    try (PreparedStatement ps = conn.prepareStatement(
                            "INSERT INTO page_titles (page_id, order_index, title) VALUES (?, ?, ?)")) {
                        for (PageTitle title : page.titles()) {
                            ps.setString(1, page.id().toString());
                            ps.setInt(2, title.orderIndex());
                            ps.setString(3, title.title());
                            ps.addBatch();
                        }
                        ps.executeBatch();
                    }
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
                return page;
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
    public List<Page> findBacklinkSources(UUID targetPageId) {
        // Find pages that have blocks containing [[page title]] links to the target page
        try (Connection conn = connectionProvider.getConnection()) {
            // First get the target page's titles
            List<String> titles = new ArrayList<>();
            try (PreparedStatement ps = conn.prepareStatement("SELECT title FROM page_titles WHERE page_id = ?")) {
                ps.setString(1, targetPageId.toString());
                try (ResultSet rs = ps.executeQuery()) {
                    while (rs.next()) {
                        titles.add(rs.getString("title"));
                    }
                }
            }

            if (titles.isEmpty()) return List.of();

            // Find pages with blocks that contain [[title]]
            Set<UUID> sourcePageIds = new LinkedHashSet<>();
            for (String title : titles) {
                String pattern = "%[[" + title + "]]%";
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

    private Page mapPage(Connection conn, ResultSet rs) throws SQLException {
        UUID id = UUID.fromString(rs.getString("id"));
        return new Page(
                id,
                rs.getString("dir_path"),
                rs.getString("file_name"),
                Instant.ofEpochMilli(rs.getLong("created_date")),
                Instant.ofEpochMilli(rs.getLong("modified_date")),
                loadTags(conn, id),
                loadTitles(conn, id),
                rs.getString("properties_json")
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

    private List<PageTitle> loadTitles(Connection conn, UUID pageId) throws SQLException {
        List<PageTitle> titles = new ArrayList<>();
        try (PreparedStatement ps = conn.prepareStatement(
                "SELECT page_id, order_index, title FROM page_titles WHERE page_id = ? ORDER BY order_index")) {
            ps.setString(1, pageId.toString());
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    titles.add(new PageTitle(
                            UUID.fromString(rs.getString("page_id")),
                            rs.getInt("order_index"),
                            rs.getString("title")
                    ));
                }
            }
        }
        return titles;
    }
}
