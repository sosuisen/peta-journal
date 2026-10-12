package com.petajournal.server.infrastructure.export;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class PageLinkConverterTest {

    private static final String NESTED = "docs/Projects/oecha/index.md";
    private static final String JOURNAL = "journals/2026-10-12.md";

    // ---- export: wikilinks ----

    @Test
    void wikilinkToSiblingDocUsesBareFileName() {
        assertEquals("[Projects/oecha/prd](prd.md)",
                PageLinkConverter.toMarkdownLinks("[[Projects/oecha/prd]]", NESTED));
    }

    @Test
    void wikilinkWithLabelKeepsLabel() {
        assertEquals("[PRD](prd.md)",
                PageLinkConverter.toMarkdownLinks("[[Projects/oecha/prd|PRD]]", NESTED));
    }

    @Test
    void wikilinkToJournalClimbsOutOfDocs() {
        assertEquals("[2026-10-12](../../../journals/2026-10-12.md)",
                PageLinkConverter.toMarkdownLinks("[[2026-10-12]]", NESTED));
    }

    @Test
    void wikilinkFromJournalToDoc() {
        assertEquals("[Projects/oecha/prd](../docs/Projects/oecha/prd.md)",
                PageLinkConverter.toMarkdownLinks("[[Projects/oecha/prd]]", JOURNAL));
    }

    @Test
    void blockLinkKeepsFragment() {
        assertEquals("[label](prd.md#2026-01-01-00-00-00-ABCDEFG)",
                PageLinkConverter.toMarkdownLinks("[[Projects/oecha/prd#2026-01-01-00-00-00-ABCDEFG|label]]", NESTED));
    }

    @Test
    void destinationWithSpaceIsWrappedInAngleBrackets() {
        assertEquals("[My Page](<../../My Page.md>)",
                PageLinkConverter.toMarkdownLinks("[[My Page]]", NESTED));
    }

    // ---- export: hashtags ----

    @Test
    void hashtagBecomesLinkToDocsPage() {
        assertEquals("note [#test](../docs/test.md) here",
                PageLinkConverter.toMarkdownLinks("note #test here", JOURNAL));
    }

    @Test
    void hashtagAtLineStartAndWithSlashes() {
        assertEquals("[#a/b](../docs/a/b.md)\n[#x](../docs/x.md)/",
                PageLinkConverter.toMarkdownLinks("#a/b\n#x/", JOURNAL));
    }

    @Test
    void hashNotAfterSpaceIsNotATag() {
        String body = "foo#bar \\#esc https://x/#frag";
        assertEquals(body, PageLinkConverter.toMarkdownLinks(body, JOURNAL));
    }

    // ---- export: code is untouched ----

    @Test
    void codeSpansAndFencesAreLeftAlone() {
        String body = "see `[[a]]` and\n```\n[[b]] #tag\n```\n[[c]]";
        assertEquals("see `[[a]]` and\n```\n[[b]] #tag\n```\n[c](../docs/c.md)",
                PageLinkConverter.toMarkdownLinks(body, JOURNAL));
    }

    // ---- import ----

    @Test
    void relativeLinkBecomesWikilink() {
        assertEquals("[[Projects/oecha/prd]]",
                PageLinkConverter.toWikilinks("[Projects/oecha/prd](prd.md)", NESTED));
    }

    @Test
    void relativeLinkWithOtherLabelKeepsLabel() {
        assertEquals("[[Projects/oecha/prd|PRD]]",
                PageLinkConverter.toWikilinks("[PRD](./prd.md)", NESTED));
    }

    @Test
    void journalLinkFromDocResolvesThroughParentDirs() {
        assertEquals("[[2026-10-12]]",
                PageLinkConverter.toWikilinks("[2026-10-12](../../../journals/2026-10-12.md)", NESTED));
    }

    @Test
    void tagLinkBecomesHashtag() {
        assertEquals("note #test here",
                PageLinkConverter.toWikilinks("note [#test](../docs/test.md) here", JOURNAL));
    }

    @Test
    void fragmentIsPreservedOnImport() {
        assertEquals("[[Projects/oecha/prd#id|label]]",
                PageLinkConverter.toWikilinks("[label](prd.md#id)", NESTED));
    }

    @Test
    void angleBracketDestinationIsAccepted() {
        assertEquals("[[My Page]]",
                PageLinkConverter.toWikilinks("[My Page](<../../My Page.md>)", NESTED));
    }

    @Test
    void foreignLinksAreLeftAlone() {
        String body = "[ext](https://example.com) [img](../../../assets/a.md) ![pic](prd.md) [esc](../../../../x.md) [txt](notes.txt)";
        assertEquals(body, PageLinkConverter.toWikilinks(body, NESTED));
    }

    // ---- round trip ----

    @Test
    void exportThenImportIsIdentity() {
        String body = "[[Projects/oecha/prd|PRD]] [[2026-10-12]] #tag [[a b#x]] `[[k]]` [ext](https://e.com)";
        String onDisk = PageLinkConverter.toMarkdownLinks(body, NESTED);
        assertEquals(body, PageLinkConverter.toWikilinks(onDisk, NESTED));
    }
}
