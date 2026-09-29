package com.petajournal.server.infrastructure.persistence;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.ItemPatch;
import com.sosuisha.hmtid.HMTID;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Path;
import java.sql.Connection;
import java.sql.SQLException;
import java.sql.Statement;
import java.time.Instant;
import java.util.List;
import java.util.Set;

import static org.junit.jupiter.api.Assertions.*;

class SqliteItemRepositoryTest {

    private static final HMTID PAGE_ID = HMTID.of("2026-09-29-00-00-00-AAAAAAA");
    private static final HMTID ITEM_A = HMTID.of("2026-09-29-00-00-01-AAAAAAA");
    private static final HMTID ITEM_B = HMTID.of("2026-09-29-00-00-02-AAAAAAA");
    private static final HMTID UNKNOWN_ITEM = HMTID.of("2026-09-29-00-00-09-AAAAAAA");
    private static final Instant CREATED = Instant.ofEpochMilli(1_000);
    private static final Instant LATER = Instant.ofEpochMilli(2_000);

    @TempDir
    Path tempDir;

    private SqliteItemRepository repository;

    @BeforeEach
    void setUp() throws SQLException {
        var provider = new SqliteConnectionProvider();
        provider.dbPath = tempDir.resolve("journal.db").toString();
        repository = new SqliteItemRepository();
        repository.connectionProvider = provider;

        try (Connection conn = provider.getConnection();
             Statement stmt = conn.createStatement()) {
            stmt.execute("""
                CREATE TABLE pages (
                    id TEXT PRIMARY KEY,
                    parent_id TEXT,
                    dir_path TEXT NOT NULL,
                    name TEXT NOT NULL,
                    type TEXT NOT NULL DEFAULT 'doc',
                    created_date INTEGER NOT NULL,
                    modified_date INTEGER NOT NULL
                )
                """);
            stmt.execute("""
                CREATE TABLE items (
                    id TEXT PRIMARY KEY,
                    page_id TEXT NOT NULL,
                    parent_id TEXT,
                    created_date INTEGER NOT NULL,
                    modified_date INTEGER NOT NULL,
                    idx TEXT NOT NULL,
                    level INTEGER NOT NULL DEFAULT 0,
                    body TEXT NOT NULL DEFAULT '',
                    ast TEXT NOT NULL DEFAULT '',
                    collapsed INTEGER NOT NULL DEFAULT 0,
                    transclude_item_id TEXT
                )
                """);
            stmt.execute("""
                CREATE TABLE item_types (
                    item_id TEXT NOT NULL,
                    type TEXT NOT NULL,
                    PRIMARY KEY (item_id, type)
                )
                """);
            stmt.execute("INSERT INTO pages (id, dir_path, name, type, created_date, modified_date) "
                    + "VALUES ('" + PAGE_ID + "', '', 'test', 'doc', 1000, 1000)");
        }
        repository.save(item(ITEM_A, "a0", 1, "parent", List.of()));
        repository.save(item(ITEM_B, "a1", 2, "child", List.of("task", "heading")));
    }

    private static Item item(HMTID id, String idx, int level, String body, List<String> types) {
        return new Item(id, PAGE_ID, null, "", "test", "doc", CREATED, CREATED, idx, level, body, "ast:" + body,
                false, types, null);
    }

    private Item find(HMTID id) {
        return repository.findById(id).orElseThrow();
    }

    @Test
    void bodyPatchKeepsLevelAndCollapsed() {
        repository.patch(ITEM_B, new ItemPatch(null, null, null, null, 5, true, LATER));

        // A body save that was prepared before the level change must not bring the old level back
        repository.patch(ITEM_B, new ItemPatch("new body", "new ast", List.of(), null, null, null, LATER));

        Item item = find(ITEM_B);
        assertEquals(5, item.level());
        assertTrue(item.collapsed());
        assertEquals("new body", item.body());
        assertEquals("new ast", item.ast());
    }

    @Test
    void levelPatchKeepsBodyAndTypes() {
        repository.patch(ITEM_B, new ItemPatch(null, null, null, null, 3, null, LATER));

        Item item = find(ITEM_B);
        assertEquals(3, item.level());
        assertEquals("child", item.body());
        assertEquals(Set.of("task", "heading"), Set.copyOf(item.types()));
        assertEquals(LATER, item.modifiedDate());
    }

    @Test
    void bodyPatchReplacesBodyTypesButKeepsTask() {
        repository.patch(ITEM_B, new ItemPatch("> quote", "ast", List.of("blockquote"), null, null, null, LATER));

        assertEquals(Set.of("task", "blockquote"), Set.copyOf(find(ITEM_B).types()));
    }

    @Test
    void taskFlagAddsAndRemovesOnlyTheTaskType() {
        repository.patch(ITEM_B, new ItemPatch(null, null, null, false, null, null, LATER));
        assertEquals(Set.of("heading"), Set.copyOf(find(ITEM_B).types()));

        repository.patch(ITEM_B, new ItemPatch(null, null, null, true, null, null, LATER));
        assertEquals(Set.of("task", "heading"), Set.copyOf(find(ITEM_B).types()));
    }

    @Test
    void patchReportsUnknownItem() {
        assertFalse(repository.patch(UNKNOWN_ITEM, new ItemPatch(null, null, null, null, 2, null, LATER)));
        assertTrue(repository.patch(ITEM_A, new ItemPatch(null, null, null, null, 2, null, LATER)));
    }

    @Test
    void shiftLevelsAddsDeltaToTheCurrentLevel() {
        repository.shiftLevels(List.of(ITEM_A, ITEM_B), 1, LATER);
        repository.shiftLevels(List.of(ITEM_A, ITEM_B), 1, LATER);

        assertEquals(3, find(ITEM_A).level());
        assertEquals(4, find(ITEM_B).level());
    }

    @Test
    void shiftLevelsNeverGoesBelowOne() {
        repository.shiftLevels(List.of(ITEM_A, ITEM_B), -1, LATER);

        assertEquals(1, find(ITEM_A).level());
        assertEquals(1, find(ITEM_B).level());
    }

    @Test
    void recalculateParentIdsFollowsTheLevels() {
        repository.recalculateParentIds(PAGE_ID);
        assertNull(find(ITEM_A).parentItemId());
        assertEquals(ITEM_A, find(ITEM_B).parentItemId());

        repository.shiftLevels(List.of(ITEM_B), -1, LATER);
        repository.recalculateParentIds(PAGE_ID);
        assertNull(find(ITEM_B).parentItemId());
    }
}
