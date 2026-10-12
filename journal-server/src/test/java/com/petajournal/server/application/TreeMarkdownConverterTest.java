package com.petajournal.server.application;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

class TreeMarkdownConverterTest {

    private final TreeMarkdownConverter converter = new TreeMarkdownConverter();

    @Test
    void treeToFlatWritesLevelInMetadataLine() {
        String tree = """
                - Top
                  - Child
                    continued
                    - [ ] Grandchild task
                - Second top""";

        String flat = converter.treeMarkdownToFlatMarkdown(tree);

        assertEquals("""
                Top

                <!-- @@ -->
                Child
                continued

                <!-- @@@ -->
                - [ ] Grandchild task

                Second top""", flat);
    }
}
