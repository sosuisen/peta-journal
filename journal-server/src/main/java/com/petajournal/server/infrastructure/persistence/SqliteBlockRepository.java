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
    public Optional<Block> findById(String id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, page_id, created_date, modified_date, idx, level, body, html, collapsed, properties_json FROM blocks WHERE id = ?")) {
            ps.setString(1, id);
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
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "SELECT id, page_id, created_date, modified_date, idx, level, body, html, collapsed, properties_json FROM blocks WHERE page_id = ? ORDER BY idx")) {
            ps.setString(1, pageId.toString());
            List<Block> blocks = new ArrayList<>();
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    blocks.add(mapBlock(conn, rs));
                }
            }
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
                        "INSERT INTO blocks (id, page_id, created_date, modified_date, idx, level, body, html, collapsed, properties_json) " +
                        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) " +
                        "ON CONFLICT(id) DO UPDATE SET page_id=?, modified_date=?, idx=?, level=?, body=?, html=?, collapsed=?, properties_json=?")) {
                    ps.setString(1, block.id());
                    ps.setString(2, block.pageId().toString());
                    ps.setLong(3, block.createdDate().toEpochMilli());
                    ps.setLong(4, block.modifiedDate().toEpochMilli());
                    ps.setString(5, block.idx());
                    ps.setInt(6, block.level());
                    ps.setString(7, block.body());
                    ps.setString(8, block.html());
                    ps.setInt(9, block.collapsed() ? 1 : 0);
                    ps.setString(10, block.propertiesJson());
                    // ON CONFLICT UPDATE values
                    ps.setString(11, block.pageId().toString());
                    ps.setLong(12, block.modifiedDate().toEpochMilli());
                    ps.setString(13, block.idx());
                    ps.setInt(14, block.level());
                    ps.setString(15, block.body());
                    ps.setString(16, block.html());
                    ps.setInt(17, block.collapsed() ? 1 : 0);
                    ps.setString(18, block.propertiesJson());
                    ps.executeUpdate();
                }

                // Replace types
                try (PreparedStatement ps = conn.prepareStatement("DELETE FROM block_types WHERE block_id = ?")) {
                    ps.setString(1, block.id());
                    ps.executeUpdate();
                }
                if (block.types() != null && !block.types().isEmpty()) {
                    try (PreparedStatement ps = conn.prepareStatement(
                            "INSERT INTO block_types (block_id, type) VALUES (?, ?)")) {
                        for (String type : block.types()) {
                            ps.setString(1, block.id());
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
    public void updateIndex(String blockId, String newIdx) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "UPDATE blocks SET idx = ?, modified_date = ? WHERE id = ?")) {
            ps.setString(1, newIdx);
            ps.setLong(2, Instant.now().toEpochMilli());
            ps.setString(3, blockId);
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update block index: " + blockId, e);
        }
    }

    @Override
    public void updateLevel(String blockId, int newLevel) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(
                     "UPDATE blocks SET level = ?, modified_date = ? WHERE id = ?")) {
            ps.setInt(1, newLevel);
            ps.setLong(2, Instant.now().toEpochMilli());
            ps.setString(3, blockId);
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to update block level: " + blockId, e);
        }
    }

    @Override
    public void delete(String id) {
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement("DELETE FROM blocks WHERE id = ?")) {
            ps.setString(1, id);
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
                     "SELECT id, page_id, created_date, modified_date, idx, level, body, html, collapsed, properties_json FROM blocks WHERE body LIKE ?")) {
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

    private Block mapBlock(Connection conn, ResultSet rs) throws SQLException {
        String blockId = rs.getString("id");
        return new Block(
                blockId,
                UUID.fromString(rs.getString("page_id")),
                Instant.ofEpochMilli(rs.getLong("created_date")),
                Instant.ofEpochMilli(rs.getLong("modified_date")),
                rs.getString("idx"),
                rs.getInt("level"),
                rs.getString("body"),
                rs.getString("html"),
                rs.getInt("collapsed") != 0,
                loadTypes(conn, blockId),
                rs.getString("properties_json")
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
}
