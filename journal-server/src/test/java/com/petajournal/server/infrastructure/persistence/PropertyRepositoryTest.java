package com.petajournal.server.infrastructure.persistence;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class PropertyRepositoryTest {

    private static final String ITEM = "item";
    private static final String ITEM_A = "2026-09-29-00-00-01-AAAAAAA";
    private static final String ITEM_B = "2026-09-29-00-00-02-AAAAAAA";

    @TempDir
    Path tempDir;

    private PropertyRepository repository;

    @BeforeEach
    void setUp() throws SQLException {
        var provider = new SqliteConnectionProvider();
        provider.dbPath = tempDir.resolve("journal.db").toString();
        repository = new PropertyRepository();
        repository.connectionProvider = provider;

        try (Connection conn = provider.getConnection();
             Statement stmt = conn.createStatement()) {
            stmt.execute("""
                CREATE TABLE property_definitions (
                    key TEXT PRIMARY KEY,
                    value_type TEXT NOT NULL
                )
                """);
            stmt.execute("""
                CREATE TABLE properties (
                    target_type TEXT NOT NULL,
                    target_id TEXT NOT NULL,
                    key TEXT NOT NULL REFERENCES property_definitions(key),
                    value_text TEXT,
                    value_number REAL,
                    value_checkbox INTEGER,
                    PRIMARY KEY (target_type, target_id, key)
                )
                """);
            stmt.execute("""
                CREATE TABLE property_list_items (
                    id INTEGER PRIMARY KEY,
                    target_type TEXT NOT NULL,
                    target_id TEXT NOT NULL,
                    key TEXT NOT NULL,
                    sort_order REAL NOT NULL,
                    value TEXT NOT NULL
                )
                """);
            stmt.execute("INSERT INTO property_definitions (key, value_type) VALUES ('tag', 'tags')");
        }
    }

    // ---- scalar properties ----

    @Test
    void scalarValuesComeBackAsText() {
        repository.upsertScalar(ITEM, ITEM_A, "plan", "date", "2026-09-29");
        repository.upsertScalar(ITEM, ITEM_A, "zoom", "number", "1.5");
        repository.upsertScalar(ITEM, ITEM_A, "count", "number", "3");
        repository.upsertScalar(ITEM, ITEM_A, "done", "checkbox", "true");

        Map<String, String> scalars = repository.findScalarByTargets(ITEM, List.of(ITEM_A)).get(ITEM_A);

        assertEquals("2026-09-29", scalars.get("plan"));
        assertEquals("1.5", scalars.get("zoom"));
        assertEquals("3", scalars.get("count"));
        assertEquals("true", scalars.get("done"));
    }

    @Test
    void upsertReplacesTheOldValue() {
        repository.upsertScalar(ITEM, ITEM_A, "plan", "date", "2026-09-29");
        repository.upsertScalar(ITEM, ITEM_A, "plan", "date", "2026-09-30");

        assertEquals("2026-09-30", repository.findScalarByTargets(ITEM, List.of(ITEM_A)).get(ITEM_A).get("plan"));
    }

    // ---- tags ----

    @Test
    void syncTagsReplacesTheWholeList() {
        repository.syncTags(ITEM, ITEM_A, List.of("a", "b"));
        repository.syncTags(ITEM, ITEM_A, List.of("c"));

        assertEquals(List.of("c"), repository.findListByTargets(ITEM, List.of(ITEM_A)).get(ITEM_A).get("tag"));
        assertEquals(List.of(ITEM_A), repository.findItemIdsByTagValue("c", 10));
        assertTrue(repository.findItemIdsByTagValue("a", 10).isEmpty());
    }

    // ---- deleteAllForTargets ----

    @Test
    void deleteAllForTargetsRemovesScalarsAndListsOfThoseTargetsOnly() {
        repository.upsertScalar(ITEM, ITEM_A, "plan", "date", "2026-09-29");
        repository.syncTags(ITEM, ITEM_A, List.of("a"));
        repository.upsertScalar(ITEM, ITEM_B, "plan", "date", "2026-09-29");
        repository.syncTags(ITEM, ITEM_B, List.of("b"));

        repository.deleteAllForTargets(ITEM, List.of(ITEM_A));

        assertFalse(repository.findScalarByTargets(ITEM, List.of(ITEM_A)).containsKey(ITEM_A));
        assertFalse(repository.findListByTargets(ITEM, List.of(ITEM_A)).containsKey(ITEM_A));
        assertEquals("2026-09-29", repository.findScalarByTargets(ITEM, List.of(ITEM_B)).get(ITEM_B).get("plan"));
        assertEquals(List.of("b"), repository.findListByTargets(ITEM, List.of(ITEM_B)).get(ITEM_B).get("tag"));
    }

    @Test
    void deleteAllForTargetsWithNoTargetsDoesNothing() {
        repository.upsertScalar(ITEM, ITEM_A, "plan", "date", "2026-09-29");

        repository.deleteAllForTargets(ITEM, List.of());

        assertTrue(repository.findScalarByTargets(ITEM, List.of(ITEM_A)).containsKey(ITEM_A));
    }
}
