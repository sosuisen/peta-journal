package com.petajournal.server.application;

import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.infrastructure.export.OutlineMarkdownParser;
import com.petajournal.server.infrastructure.parser.FlatmarkdownParser;
import com.sosuisha.hmtid.HMTID;
import org.junit.jupiter.api.Test;

import java.time.Instant;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

class FlatMarkdownSerializerTest {

    private static final HMTID PAGE = HMTID.of("2026-09-27-01-02-00-PAGE001");
    private static final HMTID ITEM1 = HMTID.of("2026-09-27-01-02-03-AAAAAA1");
    private static final HMTID ITEM2 = HMTID.of("2026-09-27-01-02-04-AAAAAA2");
    private static final HMTID ITEM3 = HMTID.of("2026-09-27-01-02-05-AAAAAA3");

    private final FlatMarkdownSerializer serializer = new FlatMarkdownSerializer();

    private static Item item(HMTID id, String idx, int level, String body, boolean collapsed,
                             List<String> types, HMTID transclude) {
        Instant t = Instant.parse("2026-09-27T01:02:03Z");
        return new Item(id, PAGE, null, "", "Page", "doc", t, t, idx, level, body, "", collapsed, types, transclude);
    }

    @Test
    void writesMetadataLineBeforeBody() {
        var items = List.of(
                item(ITEM1, "z0", 1, "# Heading", false, List.of("heading"), null),
                item(ITEM2, "z1", 2, "done task", true, List.of("task"), null),
                item(ITEM3, "z2", 2, "", false, List.of(), ITEM1));
        var scalars = Map.of(ITEM2.toString(), Map.of("plan", "2026-09-27", "end", "2026-09-27T05:00:00Z"));
        var lists = Map.of(ITEM2.toString(), Map.of("tag", List.of("a", "b")));

        String md = serializer.serialize(items, scalars, lists);

        assertEquals("""
                <!-- @ 2026-09-27-01-02-03-AAAAAA1 idx="z0" -->
                # Heading

                <!-- @@ 2026-09-27-01-02-04-AAAAAA2 idx="z1" collapsed="true" end="2026-09-27T05:00:00Z" plan="2026-09-27" tag="a,b" -->
                - [x] done task

                <!-- @@ 2026-09-27-01-02-05-AAAAAA3 idx="z2" transclude="2026-09-27-01-02-03-AAAAAA1" -->
                """, md);
    }

    @Test
    void escapesQuotesAndBackslashesInValues() {
        var items = List.of(item(ITEM1, "z0", 1, "body", false, List.of(), null));
        var scalars = Map.of(ITEM1.toString(), Map.of("title", "say \"hi\" \\ done"));

        String line = serializer.metadataLine(items.get(0), scalars.get(ITEM1.toString()), Map.of());

        assertEquals("<!-- @ 2026-09-27-01-02-03-AAAAAA1 idx=\"z0\" title=\"say \\\"hi\\\" \\\\ done\" -->", line);
    }

    @Test
    void roundTripsThroughTheParser() {
        var items = List.of(
                item(ITEM1, "z0", 1, "Paragraph\nsecond line", false, List.of(), null),
                item(ITEM2, "z1", 3, "open task", true, List.of("task"), null),
                item(ITEM3, "z2", 2, "```\ncode\n\nmore\n```", false, List.of("code"), ITEM1));
        var scalars = Map.of(ITEM2.toString(), Map.of("title", "a \"quoted\" value"));

        String md = serializer.serialize(items, scalars, Map.of());
        var parser = new OutlineMarkdownParser(new IdGenerator(), new FlatMarkdownSplitter(new FlatmarkdownParser()));
        var parsed = parser.parse(md).items();

        assertEquals(3, parsed.size());
        for (int i = 0; i < items.size(); i++) {
            assertEquals(items.get(i).id().toString(), parsed.get(i).id());
            assertEquals(items.get(i).idx(), parsed.get(i).idx());
            assertEquals(items.get(i).level(), parsed.get(i).level());
            assertEquals(items.get(i).body(), parsed.get(i).body());
            assertEquals(items.get(i).collapsed(), parsed.get(i).collapsed());
        }
        assertTrue(parsed.get(1).types().contains("task"));
        assertEquals("a \"quoted\" value", parsed.get(1).properties().get("title"));
        assertEquals(ITEM1.toString(), parsed.get(2).transcludeItemId());
    }
}
