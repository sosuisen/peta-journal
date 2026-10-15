package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.export.PageMatcher.PageMatch;
import com.petajournal.server.infrastructure.persistence.DatabaseInitializer;
import com.petajournal.server.infrastructure.persistence.SqliteConnectionProvider;
import com.petajournal.server.infrastructure.persistence.SqliteItemRepository;
import com.petajournal.server.infrastructure.persistence.SqlitePageRepository;
import com.sosuisha.hmtid.HMTID;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Instant;
import java.util.List;
import java.util.Map;

import static org.junit.jupiter.api.Assertions.*;

/**
 * How a markdown file is matched to a page: by frontmatter id, then by path, else a new page.
 * Runs against a real SQLite database in a temp directory; only the search index is a no-op.
 */
class PageMatcherTest {

    private static final HMTID OLDER_ID = HMTID.of("2026-09-01-00-00-00-AAAAAAA");
    private static final HMTID NEWER_ID = HMTID.of("2026-09-02-00-00-00-AAAAAAA");
    private static final Instant CREATED = Instant.parse("2026-09-01T00:00:00Z");
    private static final Instant MODIFIED = Instant.parse("2026-09-03T00:00:00Z");

    @TempDir
    Path markdownRoot;
    @TempDir
    Path dataDir;

    private SqlitePageRepository pageRepository;
    private PageMatcher matcher;

    @BeforeEach
    void setUp() {
        var provider = new SqliteConnectionProvider(dataDir.resolve("journal.db").toString());
        var idGenerator = new IdGenerator();
        new DatabaseInitializer(provider, idGenerator).init(null);

        pageRepository = new SqlitePageRepository(provider);
        var itemRepository = new SqliteItemRepository(provider);
        var searchService = new NoopSearchService();

        // Only ensureAncestorPages is used here; export and properties are not involved
        var pageService = new PageService(pageRepository, itemRepository, searchService, null, null, null, idGenerator);

        matcher = new PageMatcher(pageRepository, itemRepository, searchService, idGenerator, pageService);
    }

    // ---- 1. match by frontmatter id ----

    @Test
    void sameIdAtTheSamePathIsThePageItself() {
        Page page = savedDoc(OLDER_ID, "", "notes");

        PageMatch match = match("docs/notes.md", Map.of("id", OLDER_ID.toString()));

        assertEquals(page.id(), match.page().id());
        assertFalse(match.isNew());
        assertFalse(match.renamed());
        assertFalse(match.writeBack());
    }

    @Test
    void sameIdAtANewPathIsARenameWhenTheOldFileIsGone() {
        savedDoc(OLDER_ID, "", "notes");

        PageMatch match = match("docs/archive/notes.md", Map.of("id", OLDER_ID.toString()));

        assertEquals(OLDER_ID, match.page().id());
        assertTrue(match.renamed());
        assertFalse(match.isNew());
        // The page is moved by the caller, not here
        assertEquals("", pageRepository.findById(OLDER_ID).orElseThrow().dirPath());
    }

    @Test
    void sameIdAtANewPathIsACopyWhenTheOldFileStillExists() throws IOException {
        savedDoc(OLDER_ID, "", "notes");
        touch("docs/notes.md");

        PageMatch match = match("docs/copy.md", Map.of("id", OLDER_ID.toString()));

        assertTrue(match.isNew());
        assertNotEquals(OLDER_ID, match.page().id());
        assertEquals("copy", match.page().name());
        assertTrue(match.writeBack(), "the copy gets a new id, so the file is rewritten");
    }

    // ---- 2. match by path ----

    @Test
    void fileWithoutAnIdTakesThePageAtItsPathAndGetsRewritten() {
        Page page = savedDoc(OLDER_ID, "", "notes");

        PageMatch match = match("docs/notes.md", Map.of());

        assertEquals(page.id(), match.page().id());
        assertFalse(match.isNew());
        assertTrue(match.writeBack());
    }

    @Test
    void olderIdFromAnotherDeviceReplacesOurNewerId() {
        savedDoc(NEWER_ID, "", "notes");

        PageMatch match = match("docs/notes.md", Map.of("id", OLDER_ID.toString()));

        assertEquals(OLDER_ID, match.page().id());
        assertFalse(match.writeBack());
        assertTrue(pageRepository.findById(NEWER_ID).isEmpty(), "the page was rekeyed");
        assertEquals("notes", pageRepository.findById(OLDER_ID).orElseThrow().name());
    }

    @Test
    void newerIdFromAnotherDeviceLosesAndTheFileIsRewrittenWithOurs() {
        savedDoc(OLDER_ID, "", "notes");

        PageMatch match = match("docs/notes.md", Map.of("id", NEWER_ID.toString()));

        assertEquals(OLDER_ID, match.page().id());
        assertTrue(match.writeBack());
        assertTrue(pageRepository.findById(NEWER_ID).isEmpty());
    }

    // ---- 3. new page ----

    @Test
    void unknownFileWithAnIdCreatesThePageUnderThatId() {
        PageMatch match = match("journals/2026-09-29.md",
                Map.of("id", OLDER_ID.toString(), "created", CREATED.toString()));

        assertTrue(match.isNew());
        assertFalse(match.writeBack());
        Page page = pageRepository.findById(OLDER_ID).orElseThrow();
        assertEquals("journal", page.type());
        assertEquals("2026-09-29", page.name());
        assertEquals(CREATED, page.createdDate());
        assertEquals(MODIFIED, page.modifiedDate());
    }

    @Test
    void unknownFileWithoutAnIdGetsAGeneratedIdAndIsRewritten() {
        PageMatch match = match("docs/notes.md", Map.of());

        assertTrue(match.isNew());
        assertTrue(match.writeBack());
        assertTrue(pageRepository.findById(match.page().id()).isPresent());
        assertEquals(MODIFIED, match.page().createdDate(), "without a created date the file time is used");
    }

    @Test
    void newDocInASubdirectoryGetsItsIndexPages() {
        PageMatch match = match("docs/a/b/notes.md", Map.of());

        assertEquals("a/b/", match.page().dirPath());
        assertTrue(pageRepository.findByTypeDirPathFileName("doc", "a/", "index").isPresent());
        assertTrue(pageRepository.findByTypeDirPathFileName("doc", "a/b/", "index").isPresent());
    }

    // ---- helpers ----

    private PageMatch match(String relPath, Map<String, String> frontmatter) {
        var loc = MarkdownPathMapper.fromRelPath(relPath).orElseThrow();
        return matcher.match(markdownRoot, relPath, loc, frontmatter, MODIFIED);
    }

    private Page savedDoc(HMTID id, String dirPath, String name) {
        return pageRepository.save(new Page(id, null, dirPath, name, "doc", CREATED, CREATED));
    }

    private void touch(String relPath) throws IOException {
        Path file = markdownRoot.resolve(relPath);
        Files.createDirectories(file.getParent());
        Files.writeString(file, "");
    }

    /** The search index is not under test; indexing is a no-op and searches find nothing. */
    private static final class NoopSearchService implements SearchService {
        @Override public void indexPage(Page page) {}
        @Override public void indexItem(Item item, String pageTitle) {}
        @Override public void removePage(String pageId) {}
        @Override public void removeItem(String itemId) {}
        @Override public void indexItems(List<Item> items, String pageTitle) {}
        @Override public void removeItems(List<String> itemIds) {}
        @Override public void flush() {}
        @Override public List<SearchResult> searchPageNames(String query, int maxResults) { return List.of(); }
        @Override public List<SearchResult> searchFullText(String query, int maxResults) { return List.of(); }
        @Override public void deleteAll() {}
        @Override public boolean isEmpty() { return true; }
    }
}
