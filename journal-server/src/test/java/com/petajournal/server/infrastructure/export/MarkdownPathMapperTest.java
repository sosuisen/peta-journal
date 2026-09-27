package com.petajournal.server.infrastructure.export;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.*;

class MarkdownPathMapperTest {

    @Test
    void docWithDirRoundTrips() {
        String rel = MarkdownPathMapper.toRelPath("doc", "a/b/", "x");
        assertEquals("docs/a/b/x.md", rel);
        var loc = MarkdownPathMapper.fromRelPath(rel).orElseThrow();
        assertEquals("doc", loc.type());
        assertEquals("a/b/", loc.dirPath());
        assertEquals("x", loc.name());
    }

    @Test
    void journalAtRoot() {
        var loc = MarkdownPathMapper.fromRelPath("journals/2026-09-27.md").orElseThrow();
        assertEquals("journal", loc.type());
        assertEquals("", loc.dirPath());
        assertEquals("2026-09-27", loc.name());
    }

    @Test
    void unknownTopFolderIsNotAPage() {
        assertTrue(MarkdownPathMapper.fromRelPath("assets/a.md").isEmpty());
        assertTrue(MarkdownPathMapper.fromRelPath("docs/a.txt").isEmpty());
        assertTrue(MarkdownPathMapper.fromRelPath("docs/.md").isEmpty());
    }

    @Test
    void ignoresTempAndConflictFiles() {
        assertTrue(MarkdownPathMapper.isIgnored("docs/.a.md.tmp"));
        assertTrue(MarkdownPathMapper.isIgnored("docs/.dropbox.cache/a.md"));
        assertTrue(MarkdownPathMapper.isIgnored("docs/~$a.md"));
        assertTrue(MarkdownPathMapper.isIgnored("docs/a.conflict.md"));
        assertTrue(MarkdownPathMapper.isIgnored("docs/a.conflict-20260927-101500.md"));
        assertTrue(MarkdownPathMapper.isIgnored("docs/a (Alice's conflicted copy 2026-09-27).md"));
        assertTrue(MarkdownPathMapper.isIgnored("docs/a.txt"));
        assertFalse(MarkdownPathMapper.isIgnored("docs/a.md"));
        assertFalse(MarkdownPathMapper.isIgnored("journals/2026-09-27.md"));
    }

    @Test
    void hashIsStable() {
        assertEquals(MarkdownPathMapper.sha256("abc"), MarkdownPathMapper.sha256("abc"));
        assertNotEquals(MarkdownPathMapper.sha256("abc"), MarkdownPathMapper.sha256("abd"));
    }
}
