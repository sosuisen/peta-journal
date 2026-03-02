package com.petajournal.server.infrastructure.persistence;

import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.*;
import java.util.*;

@ApplicationScoped
public class PropertyRepository {

    private static final Logger LOG = Logger.getLogger(PropertyRepository.class);

    @Inject
    SqliteConnectionProvider connectionProvider;

    /**
     * Load scalar properties (text, number, checkbox, date, datetime) for given targets.
     * Returns Map&lt;targetId, Map&lt;key, value&gt;&gt; where value is always a String.
     */
    public Map<String, Map<String, String>> findScalarByTargets(String targetType, Collection<String> targetIds) {
        if (targetIds.isEmpty()) return Map.of();

        Map<String, Map<String, String>> result = new HashMap<>();

        String placeholders = String.join(",", Collections.nCopies(targetIds.size(), "?"));
        String sql = "SELECT p.target_id, p.key, p.value_text, p.value_number, p.value_checkbox, pd.value_type " +
                "FROM properties p JOIN property_definitions pd ON p.key = pd.key " +
                "WHERE p.target_type = ? AND p.target_id IN (" + placeholders + ")";

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            int idx = 1;
            ps.setString(idx++, targetType);
            for (String id : targetIds) {
                ps.setString(idx++, id);
            }

            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    String targetId = rs.getString("target_id");
                    String key = rs.getString("key");
                    String valueType = rs.getString("value_type");
                    String value = extractValue(rs, valueType);
                    if (value != null) {
                        result.computeIfAbsent(targetId, k -> new LinkedHashMap<>()).put(key, value);
                    }
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find scalar properties", e);
        }

        return result;
    }

    /**
     * Load list/tags properties for given targets.
     * Returns Map&lt;targetId, Map&lt;key, List&lt;value&gt;&gt;&gt;.
     */
    public Map<String, Map<String, List<String>>> findListByTargets(String targetType, Collection<String> targetIds) {
        if (targetIds.isEmpty()) return Map.of();

        Map<String, Map<String, List<String>>> result = new HashMap<>();

        String placeholders = String.join(",", Collections.nCopies(targetIds.size(), "?"));
        String sql = "SELECT target_id, key, value FROM property_list_items " +
                "WHERE target_type = ? AND target_id IN (" + placeholders + ") " +
                "ORDER BY target_id, key, sort_order";

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            int idx = 1;
            ps.setString(idx++, targetType);
            for (String id : targetIds) {
                ps.setString(idx++, id);
            }

            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    String targetId = rs.getString("target_id");
                    String key = rs.getString("key");
                    String value = rs.getString("value");
                    result.computeIfAbsent(targetId, k -> new LinkedHashMap<>())
                            .computeIfAbsent(key, k -> new ArrayList<>())
                            .add(value);
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find list properties", e);
        }

        return result;
    }

    /**
     * Load all property definitions.
     * Returns LinkedHashMap&lt;key, valueType&gt; ordered by key.
     */
    public Map<String, String> findAllDefinitions() {
        Map<String, String> result = new LinkedHashMap<>();
        String sql = "SELECT key, value_type FROM property_definitions ORDER BY key";

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql);
             ResultSet rs = ps.executeQuery()) {
            while (rs.next()) {
                result.put(rs.getString("key"), rs.getString("value_type"));
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find property definitions", e);
        }

        return result;
    }

    /**
     * Upsert a scalar property. Creates the property_definition if it doesn't exist,
     * then inserts or replaces the property value.
     */
    public void upsertScalar(String targetType, String targetId, String key, String valueType, String value) {
        String defSql = "INSERT OR IGNORE INTO property_definitions (key, value_type) VALUES (?, ?)";
        String upsertSql = "INSERT OR REPLACE INTO properties (target_type, target_id, key, value_text, value_number, value_checkbox) VALUES (?, ?, ?, ?, ?, ?)";

        try (Connection conn = connectionProvider.getConnection()) {
            try (PreparedStatement ps = conn.prepareStatement(defSql)) {
                ps.setString(1, key);
                ps.setString(2, valueType);
                ps.executeUpdate();
            }
            try (PreparedStatement ps = conn.prepareStatement(upsertSql)) {
                ps.setString(1, targetType);
                ps.setString(2, targetId);
                ps.setString(3, key);
                switch (valueType) {
                    case "number" -> {
                        ps.setNull(4, Types.VARCHAR);
                        ps.setDouble(5, Double.parseDouble(value));
                        ps.setNull(6, Types.INTEGER);
                    }
                    case "checkbox" -> {
                        ps.setNull(4, Types.VARCHAR);
                        ps.setNull(5, Types.DOUBLE);
                        ps.setInt(6, "true".equals(value) ? 1 : 0);
                    }
                    default -> { // text, date, datetime
                        ps.setString(4, value);
                        ps.setNull(5, Types.DOUBLE);
                        ps.setNull(6, Types.INTEGER);
                    }
                }
                ps.executeUpdate();
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to upsert scalar property", e);
        }
    }

    /**
     * Delete a property for a given target.
     */
    public void deleteProperty(String targetType, String targetId, String key) {
        String sql = "DELETE FROM properties WHERE target_type = ? AND target_id = ? AND key = ?";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setString(1, targetType);
            ps.setString(2, targetId);
            ps.setString(3, key);
            ps.executeUpdate();
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete property", e);
        }
    }

    private String extractValue(ResultSet rs, String valueType) throws SQLException {
        return switch (valueType) {
            case "number" -> {
                double num = rs.getDouble("value_number");
                if (rs.wasNull()) yield null;
                // Format as integer if no fractional part
                yield num == Math.floor(num) ? String.valueOf((long) num) : String.valueOf(num);
            }
            case "checkbox" -> {
                int cb = rs.getInt("value_checkbox");
                if (rs.wasNull()) yield null;
                yield cb != 0 ? "true" : "false";
            }
            default -> rs.getString("value_text"); // text, date, datetime
        };
    }
}
