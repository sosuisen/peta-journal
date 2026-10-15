package com.petajournal.server.infrastructure.persistence;

import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.sql.*;
import java.time.*;
import java.time.format.DateTimeFormatter;
import java.util.*;

@ApplicationScoped
public class PropertyRepository {

    private static final Logger LOG = Logger.getLogger(PropertyRepository.class);

    private final SqliteConnectionProvider connectionProvider;

    @Inject
    public PropertyRepository(SqliteConnectionProvider connectionProvider) {
        this.connectionProvider = connectionProvider;
    }

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

    /**
     * Deletes every scalar and list property of the given targets, e.g. when items are deleted.
     *
     * @param targetType {@code "item"} or {@code "page"}
     * @param targetIds  the targets whose properties go away; nothing happens for an empty list
     */
    public void deleteAllForTargets(String targetType, Collection<String> targetIds) {
        if (targetIds.isEmpty()) return;
        String placeholders = String.join(",", Collections.nCopies(targetIds.size(), "?"));
        String[] statements = {
                "DELETE FROM properties WHERE target_type = ? AND target_id IN (" + placeholders + ")",
                "DELETE FROM property_list_items WHERE target_type = ? AND target_id IN (" + placeholders + ")",
        };
        try (Connection conn = connectionProvider.getConnection()) {
            for (String sql : statements) {
                try (PreparedStatement ps = conn.prepareStatement(sql)) {
                    int idx = 1;
                    ps.setString(idx++, targetType);
                    for (String id : targetIds) {
                        ps.setString(idx++, id);
                    }
                    ps.executeUpdate();
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to delete properties of " + targetIds.size() + " targets", e);
        }
    }

    /**
     * Replace all tag entries for a target with the given list of tags.
     */
    public void syncTags(String targetType, String targetId, List<String> newTags) {
        String deleteSql = "DELETE FROM property_list_items WHERE target_type = ? AND target_id = ? AND key = 'tag'";
        String insertSql = "INSERT INTO property_list_items (target_type, target_id, key, sort_order, value) VALUES (?, ?, 'tag', ?, ?)";

        try (Connection conn = connectionProvider.getConnection()) {
            try (PreparedStatement ps = conn.prepareStatement(deleteSql)) {
                ps.setString(1, targetType);
                ps.setString(2, targetId);
                ps.executeUpdate();
            }
            if (!newTags.isEmpty()) {
                try (PreparedStatement ps = conn.prepareStatement(insertSql)) {
                    for (int i = 0; i < newTags.size(); i++) {
                        ps.setString(1, targetType);
                        ps.setString(2, targetId);
                        ps.setInt(3, i);
                        ps.setString(4, newTags.get(i));
                        ps.addBatch();
                    }
                    ps.executeBatch();
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to sync tags", e);
        }
    }

    /**
     * Find item IDs that have a specific tag value.
     */
    public List<String> findItemIdsByTagValue(String tagValue, int maxResults) {
        String sql = "SELECT target_id FROM property_list_items WHERE key = 'tag' AND value = ? AND target_type = 'item' LIMIT ?";
        List<String> ids = new ArrayList<>();

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setString(1, tagValue);
            ps.setInt(2, maxResults);
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    ids.add(rs.getString("target_id"));
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find items by tag", e);
        }

        return ids;
    }

    /**
     * Check if any tag entries exist in property_list_items.
     */
    public boolean findAnyTagExists() {
        String sql = "SELECT COUNT(*) FROM property_list_items WHERE key = 'tag' LIMIT 1";
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql);
             ResultSet rs = ps.executeQuery()) {
            return rs.next() && rs.getInt(1) > 0;
        } catch (SQLException e) {
            return false;
        }
    }

    /**
     * Find item IDs whose page path (dir_path || name) starts with the given prefix.
     */
    public List<String> findItemIdsByPagePrefix(String pathPrefix, int maxResults) {
        String sql = "SELECT i.id AS item_id FROM items i " +
                "JOIN pages p ON i.page_id = p.id " +
                "WHERE (p.dir_path || p.name) LIKE ? || '%' " +
                "LIMIT ?";
        List<String> ids = new ArrayList<>();

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setString(1, pathPrefix);
            ps.setInt(2, maxResults);
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    ids.add(rs.getString("item_id"));
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find items by page prefix", e);
        }

        return ids;
    }

    /**
     * Find item IDs that have a specific scalar property key=value.
     */
    public List<String> findItemIdsByScalarProperty(String key, String value, int maxResults) {
        String sql = "SELECT target_id FROM properties " +
                "WHERE target_type = 'item' AND key = ? AND value_text = ? " +
                "LIMIT ?";
        List<String> ids = new ArrayList<>();

        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            ps.setString(1, key);
            ps.setString(2, value);
            ps.setInt(3, maxResults);
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    ids.add(rs.getString("target_id"));
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find items by scalar property", e);
        }

        return ids;
    }

    /**
     * Find item IDs by date range on created_date or modified_date.
     * Supported expr formats:
     *   "2026-03"          → month prefix (month start to next month start)
     *   "2026-03-12"       → single day (day start to next day start)
     *   ">2026-03-01"      → after date
     *   "<2026-03-01"      → before date
     *   "2026-03-01..2026-03-12" → range [start, end+1day)
     */
    public List<String> findItemIdsByDateRange(String column, String expr, int maxResults) {
        // Whitelist columns to prevent SQL injection
        if (!"created_date".equals(column) && !"modified_date".equals(column)) {
            throw new IllegalArgumentException("Invalid column: " + column);
        }

        long startMillis = -1;
        long endMillis = -1;
        String operator = null; // null means range [start, end)

        if (expr.contains("..")) {
            // Range: val1..val2
            String[] parts = expr.split("\\.\\.", 2);
            startMillis = parseDateToEpochMillis(parts[0], false);
            endMillis = parseDateToEpochMillis(parts[1], true);
        } else if (expr.startsWith(">")) {
            operator = ">";
            startMillis = parseDateToEpochMillis(expr.substring(1), true);
        } else if (expr.startsWith("<")) {
            operator = "<";
            endMillis = parseDateToEpochMillis(expr.substring(1), false);
        } else {
            // Prefix match: month or day
            startMillis = parseDateToEpochMillis(expr, false);
            endMillis = parseDateToEpochMillis(expr, true);
        }

        String sql;
        if (operator != null) {
            if (">".equals(operator)) {
                sql = "SELECT id FROM items WHERE " + column + " >= ? LIMIT ?";
            } else {
                sql = "SELECT id FROM items WHERE " + column + " < ? LIMIT ?";
            }
        } else {
            sql = "SELECT id FROM items WHERE " + column + " >= ? AND " + column + " < ? LIMIT ?";
        }

        List<String> ids = new ArrayList<>();
        try (Connection conn = connectionProvider.getConnection();
             PreparedStatement ps = conn.prepareStatement(sql)) {
            int idx = 1;
            if (operator != null) {
                ps.setLong(idx++, ">".equals(operator) ? startMillis : endMillis);
            } else {
                ps.setLong(idx++, startMillis);
                ps.setLong(idx++, endMillis);
            }
            ps.setInt(idx, maxResults);
            try (ResultSet rs = ps.executeQuery()) {
                while (rs.next()) {
                    ids.add(rs.getString("id"));
                }
            }
        } catch (SQLException e) {
            throw new RuntimeException("Failed to find items by date range", e);
        }

        return ids;
    }

    /**
     * Parse a date string to epoch millis.
     * If {@code toEnd} is true, returns the exclusive upper bound (next day/month start).
     */
    private long parseDateToEpochMillis(String dateStr, boolean toEnd) {
        ZoneId zone = ZoneId.of("Asia/Tokyo");
        if (dateStr.length() == 7) {
            // "2026-03" → year-month
            YearMonth ym = YearMonth.parse(dateStr);
            LocalDate date = toEnd ? ym.plusMonths(1).atDay(1) : ym.atDay(1);
            return date.atStartOfDay(zone).toInstant().toEpochMilli();
        } else {
            // "2026-03-12" → full date
            LocalDate date = LocalDate.parse(dateStr);
            if (toEnd) date = date.plusDays(1);
            return date.atStartOfDay(zone).toInstant().toEpochMilli();
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
