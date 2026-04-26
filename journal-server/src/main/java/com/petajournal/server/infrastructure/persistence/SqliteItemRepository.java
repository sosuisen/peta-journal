package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.repository.ItemRepository;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.*;
import java.time.Instant;
import java.util.*;

@ApplicationScoped
public class SqliteItemRepository implements ItemRepository {

    private static final Logger LOG = Logger.getLogger(SqliteItemRepository.class);

    @Inject
    SqliteConnectionProvider connectionProvider;

    @Override
    public Optional<Item> findById(HMTID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM items b JOIN pages p ON b.page_id = p.id WHERE b.id = ?")) {
            ps.setString(1, id.toString());
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapItem(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find item by id: " + id, e);
        }
    }

    @Override
    public List<Item> findByPageId(HMTID pageId) {
        LOG.debugv("findByPageId: pageId={0}", pageId);
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM items b JOIN pages p ON b.page_id = p.id WHERE b.page_id = ? ORDER BY b.idx")) {
            ps.setString(1, pageId.toString());
            List<Item> items = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    items.add(mapItem(conn, rs));
                }
            }
            LOG.debugv("findByPageId: found {0} items", items.size());
            return items;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find items by page: " + pageId, e);
        }
    }

    @Override
    public Item save(Item item) {
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                try (PreparedStatement ps = conn.prepareStatement(
                        "INSERT INTO items (id, page_id, parent_id, created_date, modified_date, idx, level, body, ast, collapsed, transclude_item_id) " +
                        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) " +
                        "ON CONFLICT(id) DO UPDATE SET page_id=?, parent_id=?, modified_date=?, idx=?, level=?, body=?, ast=?, collapsed=?, transclude_item_id=?")) {
                    ps.setString(1, item.id().toString());
                    ps.setString(2, item.pageId().toString());
                    ps.setString(3, item.parentItemId() != null ? item.parentItemId().toString() : null);
                    ps.setLong(4, item.createdDate().toEpochMilli());
                    ps.setLong(5, item.modifiedDate().toEpochMilli());
                    ps.setString(6, item.idx());
                    ps.setInt(7, item.level());
                    ps.setString(8, item.body());
                    ps.setString(9, item.ast());
                    ps.setInt(10, item.collapsed() ? 1 : 0);
                    ps.setString(11, item.transcludeItemId() != null ? item.transcludeItemId().toString() : null);
                    // ON CONFLICT UPDATE values
                    ps.setString(12, item.pageId().toString());
                    ps.setString(13, item.parentItemId() != null ? item.parentItemId().toString() : null);
                    ps.setLong(14, item.modifiedDate().toEpochMilli());
                    ps.setString(15, item.idx());
                    ps.setInt(16, item.level());
                    ps.setString(17, item.body());
                    ps.setString(18, item.ast());
                    ps.setInt(19, item.collapsed() ? 1 : 0);
                    ps.setString(20, item.transcludeItemId() != null ? item.transcludeItemId().toString() : null);
                    ps.executeUpdate();
                }

                // Replace types
                try (PreparedStatement ps = conn.prepareStatement("DELETE FROM item_types WHERE item_id = ?")) {
                    ps.setString(1, item.id().toString());
                    ps.executeUpdate();
                }
                if (item.types() != null && !item.types().isEmpty()) {
                    try (PreparedStatement ps = conn.prepareStatement(
                            "INSERT INTO item_types (item_id, type) VALUES (?, ?)")) {
                        for (String type : item.types()) {
                            ps.setString(1, item.id().toString());
                            ps.setString(2, type);
                            ps.addBatch();
                        }
                        ps.executeBatch();
                    }
                }

                conn.commit();
                return item;
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to save item: " + item.id(), e);
        }
    }

    @Override
    public void delete(HMTID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM items WHERE id = ?")) {
            ps.setString(1, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete item: " + id, e);
        }
    }

    @Override
    public void deleteByPageId(HMTID pageId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM items WHERE page_id = ?")) {
            ps.setString(1, pageId.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete items by page: " + pageId, e);
        }
    }

    @Override
    public List<Item> findByPageIdContainingLink(String pageTitle) {
        String pattern = "%[[" + pageTitle + "]]%";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM items b JOIN pages p ON b.page_id = p.id WHERE b.body LIKE ?")) {
            ps.setString(1, pattern);
            List<Item> items = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    items.add(mapItem(conn, rs));
                }
            }
            return items;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find items containing link to: " + pageTitle, e);
        }
    }

    @Override
    public List<Item> findByTypes(List<String> types) {
        if (types == null || types.isEmpty()) {
            return List.of();
        }
        StringBuilder placeholders = new StringBuilder();
        for (int i = 0; i < types.size(); i++) {
            if (i > 0) placeholders.append(",");
            placeholders.append("?");
        }
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM items b JOIN pages p ON b.page_id = p.id " +
                     "WHERE b.id IN (SELECT item_id FROM item_types WHERE type IN (" + placeholders + ")) " +
                     "ORDER BY b.created_date DESC")) {
            for (int i = 0; i < types.size(); i++) {
                ps.setString(i + 1, types.get(i));
            }
            List<Item> items = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    items.add(mapItem(conn, rs));
                }
            }
            return items;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find items by types", e);
        }
    }

    @Override
    public void updateIndex(HMTID itemId, String newIdx) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("UPDATE items SET idx = ? WHERE id = ?")) {
            ps.setString(1, newIdx);
            ps.setString(2, itemId.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update index for item: " + itemId, e);
        }
    }

    private Item mapItem(Connection conn, ResultSet rs) throws SQLException {
        String itemIdStr = rs.getString("id");
        String parentIdStr = rs.getString("parent_id");
        String transcludeIdStr = rs.getString("transclude_item_id");
        return new Item(
                HMTID.of(itemIdStr),
                HMTID.of(rs.getString("page_id")),
                parentIdStr != null ? HMTID.of(parentIdStr) : null,
                rs.getString("dir_path"),
                rs.getString("name"),
                rs.getString("type"),
                Instant.ofEpochMilli(rs.getLong("created_date")),
                Instant.ofEpochMilli(rs.getLong("modified_date")),
                rs.getString("idx"),
                rs.getInt("level"),
                rs.getString("body"),
                rs.getString("ast"),
                rs.getInt("collapsed") != 0,
                loadTypes(conn, itemIdStr),
                transcludeIdStr != null ? HMTID.of(transcludeIdStr) : null
        );
    }

    private List<String> loadTypes(Connection conn, String itemId) throws SQLException {
        List<String> types = new ArrayList<>();
        try (PreparedStatement ps = conn.prepareStatement("SELECT type FROM item_types WHERE item_id = ?")) {
            ps.setString(1, itemId);
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    types.add(rs.getString("type"));
                }
            }
        }
        return types;
    }

    @Override
    public List<Item> findTranscluders(HMTID sourceItemId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM items b JOIN pages p ON b.page_id = p.id WHERE b.transclude_item_id = ?")) {
            ps.setString(1, sourceItemId.toString());
            List<Item> items = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    items.add(mapItem(conn, rs));
                }
            }
            return items;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find transcluders for: " + sourceItemId, e);
        }
    }

    @Override
    public List<Item> findItemWithDescendants(HMTID itemId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "WITH RECURSIVE descendants AS (" +
                     "  SELECT i.* FROM items i WHERE i.id = ? " +
                     "  UNION ALL " +
                     "  SELECT i.* FROM items i JOIN descendants d ON i.parent_id = d.id" +
                     ") " +
                     "SELECT d.id, d.page_id, d.parent_id, d.created_date, d.modified_date, d.idx, d.level, d.body, d.ast, d.collapsed, d.transclude_item_id, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM descendants d JOIN pages p ON d.page_id = p.id ORDER BY d.idx")) {
            ps.setString(1, itemId.toString());
            List<Item> items = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    items.add(mapItem(conn, rs));
                }
            }
            return items;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find item with descendants: " + itemId, e);
        }
    }

    @Override
    public void recalculateParentIds(HMTID pageId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement selectPs = conn.prepareStatement(
                     "SELECT id, level FROM items WHERE page_id = ? ORDER BY idx");
             PreparedStatement updatePs = conn.prepareStatement(
                     "UPDATE items SET parent_id = ? WHERE id = ?")) {

            selectPs.setString(1, pageId.toString());
            List<String[]> items = new ArrayList<>(); // [id, level]
            try (ResultSet rs = selectPs.executeQuery()) {
                while (rs.next()) {
                    items.add(new String[]{rs.getString("id"), String.valueOf(rs.getInt("level"))});
                }
            }

            for (int i = 0; i < items.size(); i++) {
                int level = Integer.parseInt(items.get(i)[1]);
                String parentId = null;
                if (level > 1) {
                    for (int j = i - 1; j >= 0; j--) {
                        if (Integer.parseInt(items.get(j)[1]) < level) {
                            parentId = items.get(j)[0];
                            break;
                        }
                    }
                }
                updatePs.setString(1, parentId);
                updatePs.setString(2, items.get(i)[0]);
                updatePs.addBatch();
            }
            updatePs.executeBatch();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to recalculate parent IDs for page: " + pageId, e);
        }
    }

}
