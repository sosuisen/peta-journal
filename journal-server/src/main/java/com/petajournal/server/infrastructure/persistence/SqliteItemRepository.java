package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.ItemPatch;
import com.petajournal.server.domain.model.ItemPlacement;
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
    /** Not in the index alphabet, so a parked index never equals a real one. */
    private static final String PARKED_IDX_PREFIX = "~";

    /** The item columns {@link #mapItem} reads, joined with the page columns an item carries. */
    private static final String SELECT_ITEMS =
            "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, " +
            "p.dir_path, p.name, p.type " +
            "FROM items b JOIN pages p ON b.page_id = p.id ";

    @Inject
    SqliteConnectionProvider connectionProvider;

    @Override
    public Optional<Item> findById(HMTID id) {
        List<Item> items = queryItems(SELECT_ITEMS + "WHERE b.id = ?", "Failed to find item by id: " + id, id.toString());
        return items.isEmpty() ? Optional.empty() : Optional.of(items.getFirst());
    }

    @Override
    public List<Item> findByPageId(HMTID pageId) {
        LOG.debugv("findByPageId: pageId={0}", pageId);
        List<Item> items = queryItems(SELECT_ITEMS + "WHERE b.page_id = ? ORDER BY b.idx",
                "Failed to find items by page: " + pageId, pageId.toString());
        LOG.debugv("findByPageId: found {0} items", items.size());
        return items;
    }

    @Override
    public List<Item> findByPageIdContainingLink(String pageTitle) {
        String pattern = "%[[" + pageTitle + "]]%";
        return queryItems(SELECT_ITEMS + "WHERE b.body LIKE ?",
                "Failed to find items containing link to: " + pageTitle, pattern);
    }

    @Override
    public List<Item> findByTypes(List<String> types) {
        if (types == null || types.isEmpty()) {
            return List.of();
        }
        String placeholders = String.join(",", Collections.nCopies(types.size(), "?"));
        return queryItems(SELECT_ITEMS +
                        "WHERE b.id IN (SELECT item_id FROM item_types WHERE type IN (" + placeholders + ")) " +
                        "ORDER BY b.created_date DESC",
                "Failed to find items by types", types.toArray(new String[0]));
    }

    @Override
    public List<Item> findTranscluders(HMTID sourceItemId) {
        return queryItems(SELECT_ITEMS + "WHERE b.transclude_item_id = ?",
                "Failed to find transcluders for: " + sourceItemId, sourceItemId.toString());
    }

    @Override
    public List<Item> findItemWithDescendants(HMTID itemId) {
        return queryItems(
                "WITH RECURSIVE descendants AS (" +
                "  SELECT i.* FROM items i WHERE i.id = ? " +
                "  UNION ALL " +
                "  SELECT i.* FROM items i JOIN descendants d ON i.parent_id = d.id" +
                ") " +
                "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.ast, b.collapsed, b.transclude_item_id, " +
                "p.dir_path, p.name, p.type " +
                "FROM descendants b JOIN pages p ON b.page_id = p.id ORDER BY b.idx",
                "Failed to find item with descendants: " + itemId, itemId.toString());
    }

    /**
     * Runs a query whose result rows are items in the shape of {@link #SELECT_ITEMS}.
     *
     * @param sql          the query
     * @param errorMessage the message of the exception thrown when the query fails
     * @param params       the string parameters of the query, in order
     */
    private List<Item> queryItems(String sql, String errorMessage, String... params) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            for (int i = 0; i < params.length; i++) {
                ps.setString(i + 1, params[i]);
            }
            List<Item> items = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    items.add(mapItem(conn, rs));
                }
            }
            return items;
        } catch (SQLException e) {
            throw new RuntimeException(errorMessage, e);
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
                deleteTypes(conn, item.id().toString());
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
    public boolean patch(HMTID id, ItemPatch patch) {
        Objects.requireNonNull(id, "id must not be null");
        Objects.requireNonNull(patch, "patch must not be null");
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                boolean found = patchColumns(conn, id, patch);
                if (found) {
                    patchTypes(conn, id, patch);
                }
                conn.commit();
                return found;
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to patch item: " + id, e);
        }
    }

    private boolean patchColumns(Connection conn, HMTID id, ItemPatch patch) throws SQLException {
        StringBuilder sql = new StringBuilder("UPDATE items SET modified_date = ?");
        if (patch.changesBody()) {
            sql.append(", body = ?, ast = ?");
        }
        if (patch.level() != null) {
            sql.append(", level = ?");
        }
        if (patch.collapsed() != null) {
            sql.append(", collapsed = ?");
        }
        sql.append(" WHERE id = ?");

        try (PreparedStatement ps = conn.prepareStatement(sql.toString())) {
            int n = 1;
            ps.setLong(n++, patch.modifiedDate().toEpochMilli());
            if (patch.changesBody()) {
                ps.setString(n++, patch.body());
                ps.setString(n++, patch.ast());
            }
            if (patch.level() != null) {
                ps.setInt(n++, patch.level());
            }
            if (patch.collapsed() != null) {
                ps.setInt(n++, patch.collapsed() ? 1 : 0);
            }
            ps.setString(n, id.toString());
            return ps.executeUpdate() > 0;
        }
    }

    private void patchTypes(Connection conn, HMTID id, ItemPatch patch) throws SQLException {
        if (patch.changesBody()) {
            try (PreparedStatement ps = conn.prepareStatement(
                    "DELETE FROM item_types WHERE item_id = ? AND type <> ?")) {
                ps.setString(1, id.toString());
                ps.setString(2, ItemPatch.TASK_TYPE);
                ps.executeUpdate();
            }
            for (String type : patch.bodyTypes()) {
                insertType(conn, id, type);
            }
        }
        if (Boolean.TRUE.equals(patch.task())) {
            insertType(conn, id, ItemPatch.TASK_TYPE);
        } else if (Boolean.FALSE.equals(patch.task())) {
            try (PreparedStatement ps = conn.prepareStatement(
                    "DELETE FROM item_types WHERE item_id = ? AND type = ?")) {
                ps.setString(1, id.toString());
                ps.setString(2, ItemPatch.TASK_TYPE);
                ps.executeUpdate();
            }
        }
    }

    private void insertType(Connection conn, HMTID id, String type) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement(
                "INSERT OR IGNORE INTO item_types (item_id, type) VALUES (?, ?)")) {
            ps.setString(1, id.toString());
            ps.setString(2, type);
            ps.executeUpdate();
        }
    }

    private void deleteTypes(Connection conn, String itemId) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement("DELETE FROM item_types WHERE item_id = ?")) {
            ps.setString(1, itemId);
            ps.executeUpdate();
        }
    }

    @Override
    public void shiftLevels(List<HMTID> ids, int delta, Instant modifiedDate) {
        Objects.requireNonNull(ids, "ids must not be null");
        Objects.requireNonNull(modifiedDate, "modifiedDate must not be null");
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try (PreparedStatement ps = conn.prepareStatement(
                    "UPDATE items SET level = MAX(1, level + ?), modified_date = ? WHERE id = ?")) {
                for (HMTID id : ids) {
                    ps.setInt(1, delta);
                    ps.setLong(2, modifiedDate.toEpochMilli());
                    ps.setString(3, id.toString());
                    ps.addBatch();
                }
                ps.executeBatch();
                conn.commit();
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to shift levels of " + ids.size() + " items", e);
        }
    }

    @Override
    public void moveItems(HMTID targetPageId, List<ItemPlacement> placements, int levelDelta, Instant modifiedDate) {
        Objects.requireNonNull(targetPageId, "targetPageId must not be null");
        Objects.requireNonNull(placements, "placements must not be null");
        Objects.requireNonNull(modifiedDate, "modifiedDate must not be null");
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                parkIndices(conn, placements);
                placeItems(conn, targetPageId, placements, levelDelta, modifiedDate);
                conn.commit();
            } catch (SQLException | RuntimeException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to move " + placements.size() + " items to page: " + targetPageId, e);
        }
    }

    /**
     * (page_id, idx) is unique and checked per statement. A moving item may take the index
     * that another moving item still holds, so all of them leave their index first.
     */
    private void parkIndices(Connection conn, List<ItemPlacement> placements) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement("UPDATE items SET idx = ? WHERE id = ?")) {
            for (ItemPlacement placement : placements) {
                String id = placement.itemId().toString();
                ps.setString(1, PARKED_IDX_PREFIX + id);
                ps.setString(2, id);
                if (ps.executeUpdate() == 0) {
                    throw new IllegalArgumentException("Item not found: " + id);
                }
            }
        }
    }

    private void placeItems(Connection conn, HMTID targetPageId, List<ItemPlacement> placements,
                            int levelDelta, Instant modifiedDate) throws SQLException {
        try (PreparedStatement ps = conn.prepareStatement(
                "UPDATE items SET page_id = ?, parent_id = NULL, idx = ?, level = MAX(1, level + ?), "
                        + "modified_date = ? WHERE id = ?")) {
            for (ItemPlacement placement : placements) {
                ps.setString(1, targetPageId.toString());
                ps.setString(2, placement.idx());
                ps.setInt(3, levelDelta);
                ps.setLong(4, modifiedDate.toEpochMilli());
                ps.setString(5, placement.itemId().toString());
                ps.executeUpdate();
            }
        }
    }

    @Override
    public void delete(HMTID id) {
        // Foreign keys are not enforced on these connections, so the types are removed by hand
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                deleteTypes(conn, id.toString());
                try (PreparedStatement ps = conn.prepareStatement("DELETE FROM items WHERE id = ?")) {
                    ps.setString(1, id.toString());
                    ps.executeUpdate();
                }
                conn.commit();
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete item: " + id, e);
        }
    }

    @Override
    public void deleteByPageId(HMTID pageId) {
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                try (PreparedStatement ps = conn.prepareStatement(
                        "DELETE FROM item_types WHERE item_id IN (SELECT id FROM items WHERE page_id = ?)")) {
                    ps.setString(1, pageId.toString());
                    ps.executeUpdate();
                }
                try (PreparedStatement ps = conn.prepareStatement("DELETE FROM items WHERE page_id = ?")) {
                    ps.setString(1, pageId.toString());
                    ps.executeUpdate();
                }
                conn.commit();
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete items by page: " + pageId, e);
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
    public void recalculateParentIds(HMTID pageId) {
        try (Connection conn = connectionProvider.getConnection()) {
            // One transaction: the levels are read and the parents are written against the same state,
            // and the page costs one commit instead of one per item
            conn.setAutoCommit(false);
            try {
                recalculateParentIds(conn, pageId);
                conn.commit();
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to recalculate parent IDs for page: " + pageId, e);
        }
    }

    /** The id and level of an item, all that the parent calculation needs. */
    private record ItemLevel(String id, int level) {}

    private void recalculateParentIds(Connection conn, HMTID pageId) throws SQLException {
        try (PreparedStatement selectPs = conn.prepareStatement(
                     "SELECT id, level FROM items WHERE page_id = ? ORDER BY idx");
             PreparedStatement updatePs = conn.prepareStatement(
                     "UPDATE items SET parent_id = ? WHERE id = ?")) {

            selectPs.setString(1, pageId.toString());
            List<ItemLevel> items = new ArrayList<>();
            try (ResultSet rs = selectPs.executeQuery()) {
                while (rs.next()) {
                    items.add(new ItemLevel(rs.getString("id"), rs.getInt("level")));
                }
            }

            // The parent of an item is the nearest item above it with a smaller level
            for (int i = 0; i < items.size(); i++) {
                int level = items.get(i).level();
                String parentId = null;
                for (int j = i - 1; j >= 0 && level > 1; j--) {
                    if (items.get(j).level() < level) {
                        parentId = items.get(j).id();
                        break;
                    }
                }
                updatePs.setString(1, parentId);
                updatePs.setString(2, items.get(i).id());
                updatePs.addBatch();
            }
            updatePs.executeBatch();
        }
    }

}
