package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.repository.BlockRepository;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.*;
import java.time.Instant;
import java.util.*;

@ApplicationScoped
public class SqliteBlockRepository implements BlockRepository {

    private static final Logger LOG = Logger.getLogger(SqliteBlockRepository.class);

    @Inject
    SqliteConnectionProvider connectionProvider;

    @Override
    public Optional<Block> findById(UUID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.html, b.collapsed, b.status, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM blocks b JOIN pages p ON b.page_id = p.id WHERE b.id = ?")) {
            ps.setString(1, id.toString());
            try (ResultSet rs = ps.executeQuery()) {
                if (!rs.next()) return Optional.empty();
                return Optional.of(mapBlock(conn, rs));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find block by id: " + id, e);
        }
    }

    @Override
    public List<Block> findByPageId(UUID pageId) {
        LOG.debugv("findByPageId: pageId={0}", pageId);
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.html, b.collapsed, b.status, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM blocks b JOIN pages p ON b.page_id = p.id WHERE b.page_id = ? ORDER BY b.idx")) {
            ps.setString(1, pageId.toString());
            List<Block> blocks = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    blocks.add(mapBlock(conn, rs));
                }
            }
            LOG.debugv("findByPageId: found {0} blocks", blocks.size());
            return blocks;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find blocks by page: " + pageId, e);
        }
    }

    @Override
    public Block save(Block block) {
        try (Connection conn = connectionProvider.getConnection()) {
            conn.setAutoCommit(false);
            try {
                try (PreparedStatement ps = conn.prepareStatement(
                        "INSERT INTO blocks (id, page_id, parent_id, created_date, modified_date, idx, level, body, html, collapsed, status) " +
                        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) " +
                        "ON CONFLICT(id) DO UPDATE SET page_id=?, parent_id=?, modified_date=?, idx=?, level=?, body=?, html=?, collapsed=?, status=?")) {
                    ps.setString(1, block.id().toString());
                    ps.setString(2, block.pageId().toString());
                    ps.setString(3, block.parentBlockId() != null ? block.parentBlockId().toString() : null);
                    ps.setLong(4, block.createdDate().toEpochMilli());
                    ps.setLong(5, block.modifiedDate().toEpochMilli());
                    ps.setString(6, block.idx());
                    ps.setInt(7, block.level());
                    ps.setString(8, block.body());
                    ps.setString(9, block.html());
                    ps.setInt(10, block.collapsed() ? 1 : 0);
                    ps.setString(11, block.status());
                    // ON CONFLICT UPDATE values
                    ps.setString(12, block.pageId().toString());
                    ps.setString(13, block.parentBlockId() != null ? block.parentBlockId().toString() : null);
                    ps.setLong(14, block.modifiedDate().toEpochMilli());
                    ps.setString(15, block.idx());
                    ps.setInt(16, block.level());
                    ps.setString(17, block.body());
                    ps.setString(18, block.html());
                    ps.setInt(19, block.collapsed() ? 1 : 0);
                    ps.setString(20, block.status());
                    ps.executeUpdate();
                }

                // Replace types
                try (PreparedStatement ps = conn.prepareStatement("DELETE FROM block_types WHERE block_id = ?")) {
                    ps.setString(1, block.id().toString());
                    ps.executeUpdate();
                }
                if (block.types() != null && !block.types().isEmpty()) {
                    try (PreparedStatement ps = conn.prepareStatement(
                            "INSERT INTO block_types (block_id, type) VALUES (?, ?)")) {
                        for (String type : block.types()) {
                            ps.setString(1, block.id().toString());
                            ps.setString(2, type);
                            ps.addBatch();
                        }
                        ps.executeBatch();
                    }
                }

                conn.commit();
                return block;
            } catch (SQLException e) {
                conn.rollback();
                throw e;
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to save block: " + block.id(), e);
        }
    }

    @Override
    public void delete(UUID id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM blocks WHERE id = ?")) {
            ps.setString(1, id.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete block: " + id, e);
        }
    }

    @Override
    public void deleteByPageId(UUID pageId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM blocks WHERE page_id = ?")) {
            ps.setString(1, pageId.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete blocks by page: " + pageId, e);
        }
    }

    @Override
    public List<Block> findByPageIdContainingLink(String pageTitle) {
        String pattern = "%[[" + pageTitle + "]]%";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.html, b.collapsed, b.status, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM blocks b JOIN pages p ON b.page_id = p.id WHERE b.body LIKE ?")) {
            ps.setString(1, pattern);
            List<Block> blocks = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    blocks.add(mapBlock(conn, rs));
                }
            }
            return blocks;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find blocks containing link to: " + pageTitle, e);
        }
    }

    @Override
    public List<Block> findByStatuses(List<String> statuses) {
        if (statuses == null || statuses.isEmpty()) {
            return List.of();
        }
        StringBuilder placeholders = new StringBuilder();
        for (int i = 0; i < statuses.size(); i++) {
            if (i > 0) placeholders.append(",");
            placeholders.append("?");
        }
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT b.id, b.page_id, b.parent_id, b.created_date, b.modified_date, b.idx, b.level, b.body, b.html, b.collapsed, b.status, " +
                     "p.dir_path, p.name, p.type " +
                     "FROM blocks b JOIN pages p ON b.page_id = p.id WHERE b.status IN (" + placeholders + ") ORDER BY b.created_date DESC")) {
            for (int i = 0; i < statuses.size(); i++) {
                ps.setString(i + 1, statuses.get(i));
            }
            List<Block> blocks = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    blocks.add(mapBlock(conn, rs));
                }
            }
            return blocks;
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find blocks by statuses", e);
        }
    }

    @Override
    public void updateIndex(UUID blockId, String newIdx) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("UPDATE blocks SET idx = ? WHERE id = ?")) {
            ps.setString(1, newIdx);
            ps.setString(2, blockId.toString());
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update index for block: " + blockId, e);
        }
    }

    private Block mapBlock(Connection conn, ResultSet rs) throws SQLException {
        String blockIdStr = rs.getString("id");
        UUID blockId = UUID.fromString(blockIdStr);
        String parentIdStr = rs.getString("parent_id");
        return new Block(
                blockId,
                UUID.fromString(rs.getString("page_id")),
                parentIdStr != null ? UUID.fromString(parentIdStr) : null,
                rs.getString("dir_path"),
                rs.getString("name"),
                rs.getString("type"),
                Instant.ofEpochMilli(rs.getLong("created_date")),
                Instant.ofEpochMilli(rs.getLong("modified_date")),
                rs.getString("idx"),
                rs.getInt("level"),
                rs.getString("body"),
                rs.getString("html"),
                rs.getInt("collapsed") != 0,
                rs.getString("status"),
                loadTypes(conn, blockIdStr)
        );
    }

    private List<String> loadTypes(Connection conn, String blockId) throws SQLException {
        List<String> types = new ArrayList<>();
        try (PreparedStatement ps = conn.prepareStatement("SELECT type FROM block_types WHERE block_id = ?")) {
            ps.setString(1, blockId);
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    types.add(rs.getString("type"));
                }
            }
        }
        return types;
    }

    @Override
    public void recalculateParentIds(UUID pageId) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement selectPs = conn.prepareStatement(
                     "SELECT id, level FROM blocks WHERE page_id = ? ORDER BY idx");
             PreparedStatement updatePs = conn.prepareStatement(
                     "UPDATE blocks SET parent_id = ? WHERE id = ?")) {

            selectPs.setString(1, pageId.toString());
            List<String[]> blocks = new ArrayList<>(); // [id, level]
            try (ResultSet rs = selectPs.executeQuery()) {
                while (rs.next()) {
                    blocks.add(new String[]{rs.getString("id"), String.valueOf(rs.getInt("level"))});
                }
            }

            for (int i = 0; i < blocks.size(); i++) {
                int level = Integer.parseInt(blocks.get(i)[1]);
                String parentId = null;
                if (level > 1) {
                    for (int j = i - 1; j >= 0; j--) {
                        if (Integer.parseInt(blocks.get(j)[1]) < level) {
                            parentId = blocks.get(j)[0];
                            break;
                        }
                    }
                }
                updatePs.setString(1, parentId);
                updatePs.setString(2, blocks.get(i)[0]);
                updatePs.addBatch();
            }
            updatePs.executeBatch();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to recalculate parent IDs for page: " + pageId, e);
        }
    }
}
