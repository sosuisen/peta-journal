package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.FlatMarkdownSplitter;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.infrastructure.parser.FlatmarkdownParser;
import org.junit.jupiter.api.Test;

import java.util.List;

import static org.junit.jupiter.api.Assertions.*;

class OutlineMarkdownParserTest {

    private static final String ITEM1 = "2026-09-27-01-02-03-AAAAAA1";
    private static final String ITEM2 = "2026-09-27-01-02-04-AAAAAA2";
    private static final String ITEM3 = "2026-09-27-01-02-05-AAAAAA3";
    private static final String ITEM4 = "2026-09-27-01-02-06-AAAAAA4";

    private final OutlineMarkdownParser parser = new OutlineMarkdownParser(new IdGenerator(), new FlatMarkdownSplitter(new FlatmarkdownParser()));

    @Test
    void parsesFrontmatterItemsAndMetadata() {
        String md = """
            ---
            id: 2026-09-27-01-02-00-PAGE001
            created: 2026-09-27T01:02:03Z
            modified: 2026-09-27T04:05:06.789Z
            ---

            <!-- @ 2026-09-27-01-02-03-AAAAAA1 idx="z0" -->
            # Heading

            <!-- @@ 2026-09-27-01-02-04-AAAAAA2 collapsed="true" end="2026-09-27T05:00:00Z" plan="2026-09-27" -->
            - [x] done task

            <!-- @@ 2026-09-27-01-02-05-AAAAAA3 transclude="2026-09-27-01-02-03-AAAAAA1" -->
            mirror

            <!-- @ 2026-09-27-01-02-06-AAAAAA4 -->
            ```
            code with

            blank line inside fence
            ```
            """;

        var result = parser.parse(md);
        assertEquals("2026-09-27-01-02-00-PAGE001", result.frontmatter().get("id"));
        assertEquals("2026-09-27T04:05:06.789Z", result.frontmatter().get("modified"));

        List<OutlineMarkdownParser.ParsedItem> items = result.items();
        assertEquals(4, items.size());

        var heading = items.get(0);
        assertEquals(ITEM1, heading.id());
        assertEquals(1, heading.level());
        assertEquals("# Heading", heading.body());
        assertTrue(heading.types().contains("heading"));
        assertNull(heading.transcludeItemId());
        assertEquals("z0", heading.idx());
        assertFalse(heading.properties().containsKey("idx"));
        assertNull(items.get(1).idx());

        var task = items.get(1);
        assertEquals(ITEM2, task.id());
        assertEquals("done task", task.body());
        assertEquals(2, task.level());
        assertTrue(task.collapsed());
        assertFalse(task.properties().containsKey("collapsed"));
        assertTrue(task.types().contains("task"));
        assertEquals("2026-09-27T05:00:00Z", task.properties().get("end"));
        assertEquals("2026-09-27", task.properties().get("plan"));

        var mirror = items.get(2);
        assertEquals(ITEM3, mirror.id());
        assertEquals(ITEM1, mirror.transcludeItemId());
        assertFalse(mirror.properties().containsKey("transclude"));

        var code = items.get(3);
        assertEquals(ITEM4, code.id());
        assertTrue(code.body().contains("\n\nblank line inside fence"));
        assertTrue(code.types().contains("code"));
    }

    @Test
    void metadataLineAloneIsAnEmptyItem() {
        var result = parser.parse("---\nid: x\n---\n\n<!-- @ " + ITEM1 + " idx=\"z0\" -->\n\n");
        assertEquals(1, result.items().size());
        assertEquals(ITEM1, result.items().get(0).id());
        assertEquals("", result.items().get(0).body());
    }

    @Test
    void blocksWithoutMetadataAreItemsAtLevelOne() {
        var result = parser.parse("Paragraph 1\n\nParagraph 2\n");
        assertEquals(2, result.items().size());
        assertNull(result.items().get(0).id());
        assertEquals(1, result.items().get(0).level());
        assertEquals("Paragraph 2", result.items().get(1).body());
    }

    @Test
    void unescapesPropertyValues() {
        var result = parser.parse("<!-- @ " + ITEM1 + " title=\"say \\\"hi\\\" \\\\ done\" -->\nbody\n");
        assertEquals("say \"hi\" \\ done", result.items().get(0).properties().get("title"));
    }

    @Test
    void blankContentHasNoItems() {
        assertTrue(parser.parse("").items().isEmpty());
        assertTrue(parser.parse("---\nid: x\n---\n").items().isEmpty());
    }
}
