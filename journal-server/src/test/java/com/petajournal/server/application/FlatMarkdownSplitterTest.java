package com.petajournal.server.application;

import com.petajournal.server.infrastructure.parser.FlatmarkdownParser;
import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class FlatMarkdownSplitterTest {

    private final FlatMarkdownSplitter splitter = new FlatMarkdownSplitter(new FlatmarkdownParser());

    @Test
    void consecutiveListItemsAreOneBlock() {
        var page = splitter.split("# Night\n\n- [ ] Hanon\n\n- [ ] Upload\n");

        assertEquals(2, page.items().size());
        assertEquals("# Night", page.items().get(0).body());
        assertEquals("- [ ] Hanon\n\n- [ ] Upload", page.items().get(1).body());
    }

    @Test
    void metadataLineSeparatesListItems() {
        var page = splitter.split("# Night\n\n<!-- @@ -->\n- [ ] Hanon\n\n<!-- @@ -->\n- [ ] Upload\n\n<!-- @@ -->\n");

        assertEquals(4, page.items().size());
        assertEquals(1, page.items().get(0).level());
        assertEquals(2, page.items().get(1).level());
        assertEquals("- [ ] Hanon", page.items().get(1).body());
        assertEquals("- [ ] Upload", page.items().get(2).body());
        assertEquals("", page.items().get(3).body());
        assertNull(page.items().get(1).id());
    }

    @Test
    void headingDirectlyFollowedByTextIsTwoBlocks() {
        var page = splitter.split("# Title\ntext\n");

        assertEquals(2, page.items().size());
    }

    @Test
    void emptyInputHasNoItems() {
        assertTrue(splitter.split("").items().isEmpty());
        assertNull(splitter.split(null).frontMatter());
    }
}
