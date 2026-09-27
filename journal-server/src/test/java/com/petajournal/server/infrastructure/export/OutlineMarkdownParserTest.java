package com.petajournal.server.infrastructure.export;

import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class OutlineMarkdownParserTest {

    private final OutlineMarkdownParser parser = new OutlineMarkdownParser();

    @Test
    void parsesFrontmatterItemsAndMetadata() {
        String md = """
            ---
            id: 01ABCDEFGH-page
            created: 2026-09-27T01:02:03Z
            modified: 2026-09-27T04:05:06.789Z
            ---

            # Heading
            <!-- 01ABCDEFGH-item1 -->
            <!-- idx=z0 -->

            - [x] done task
            <!-- 01ABCDEFGH-item2 -->
            <!-- level=2 -->
            <!-- collapsed=true -->
            <!-- end=2026-09-27T05:00:00Z -->
            <!-- plan=2026-09-27 -->

            mirror
            <!-- 01ABCDEFGH-item3 -->
            <!-- transclude=01ABCDEFGH-item1 -->

            ```
            code with

            blank line inside fence
            ```
            <!-- 01ABCDEFGH-item4 -->
            """;

        var result = parser.parse(md);
        assertEquals("01ABCDEFGH-page", result.frontmatter().get("id"));
        assertEquals("2026-09-27T04:05:06.789Z", result.frontmatter().get("modified"));

        List<OutlineMarkdownParser.ParsedItem> items = result.items();
        assertEquals(4, items.size());

        var heading = items.get(0);
        assertEquals("01ABCDEFGH-item1", heading.id());
        assertEquals("# Heading", heading.body());
        assertTrue(heading.types().contains("heading"));
        assertNull(heading.transcludeItemId());
        assertEquals("z0", heading.idx());
        assertFalse(heading.properties().containsKey("idx"));
        assertNull(items.get(1).idx());

        var task = items.get(1);
        assertEquals("done task", task.body());
        assertEquals(2, task.level());
        assertTrue(task.collapsed());
        assertTrue(task.types().contains("task"));
        assertEquals("2026-09-27T05:00:00Z", task.properties().get("end"));
        assertEquals("2026-09-27", task.properties().get("plan"));

        var mirror = items.get(2);
        assertEquals("01ABCDEFGH-item1", mirror.transcludeItemId());
        assertFalse(mirror.properties().containsKey("transclude"));

        var code = items.get(3);
        assertTrue(code.body().contains("\n\nblank line inside fence"));
        assertTrue(code.types().contains("code"));
    }

    @Test
    void emptyBodyMarkerBecomesEmptyString() {
        var result = parser.parse("---\nid: x\n---\n\n \n<!-- 01ABCDEFGH-item1 -->\n");
        assertEquals(1, result.items().size());
        assertEquals("", result.items().get(0).body());
    }

    @Test
    void blankContentHasNoItems() {
        assertTrue(parser.parse("").items().isEmpty());
        assertTrue(parser.parse("---\nid: x\n---\n").items().isEmpty());
    }
}
