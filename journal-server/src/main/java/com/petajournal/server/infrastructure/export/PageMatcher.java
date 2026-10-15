package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.export.MarkdownPathMapper.PageLocation;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Instant;
import java.util.Map;
import java.util.Optional;

/**
 * Decides which page a markdown file belongs to, and saves pages at the location their file has.
 * <ol>
 *   <li>By frontmatter id: same path is the page itself; a new path is a rename, unless the old
 *       file still exists, in which case this file is a copy and gets a page of its own.</li>
 *   <li>By path: a file without an id gets ours. A file with another id comes from a device that
 *       created the same page independently; the older id wins on every device.</li>
 *   <li>Otherwise a new page is created, keeping the file's id when it has one.</li>
 * </ol>
 */
@ApplicationScoped
public class PageMatcher {

    private static final Logger LOG = Logger.getLogger(PageMatcher.class);

    /**
     * The page a file belongs to.
     *
     * @param page      the page, already saved when it is new
     * @param isNew     the page was created for this file
     * @param renamed   the page exists but its file moved to this path; see {@link #move}
     * @param writeBack the file lacks our id or carries a foreign one and must be rewritten after the import
     */
    public record PageMatch(Page page, boolean isNew, boolean renamed, boolean writeBack) {}

    private final PageRepository pageRepository;
    private final ItemRepository itemRepository;
    private final SearchService searchService;
    private final IdGenerator idGenerator;
    private final PageService pageService;

    @Inject
    public PageMatcher(
            PageRepository pageRepository,
            ItemRepository itemRepository,
            SearchService searchService,
            IdGenerator idGenerator,
            PageService pageService) {
        this.pageRepository = pageRepository;
        this.itemRepository = itemRepository;
        this.searchService = searchService;
        this.idGenerator = idGenerator;
        this.pageService = pageService;
    }

    /**
     * Finds the page a file belongs to, or creates one.
     *
     * @param root         the markdown directory
     * @param relPath      the file's path under root
     * @param loc          the page location decoded from relPath
     * @param frontmatter  the file's front matter
     * @param fileModified the file's modified time, used for a new page
     */
    public PageMatch match(Path root, String relPath, PageLocation loc, Map<String, String> frontmatter,
                           Instant fileModified) {
        HMTID fileId = parseId(frontmatter.get("id"));
        boolean writeBack = false;

        // 1. Match by frontmatter id
        if (fileId != null) {
            Optional<Page> byId = pageRepository.findById(fileId);
            if (byId.isPresent()) {
                Page p = byId.get();
                String currentRel = MarkdownPathMapper.toRelPath(p);
                if (currentRel.equals(relPath)) {
                    return new PageMatch(p, false, false, false);
                }
                if (!Files.exists(root.resolve(currentRel))) {
                    LOG.infov("Page {0} moved: {1} -> {2}", p.id(), currentRel, relPath);
                    return new PageMatch(p, false, true, false);
                }
                // Same id in two files: this one is a copy, give it a new identity
                LOG.infov("{0} is a copy of {1}, creating a new page", relPath, currentRel);
                fileId = null;
                writeBack = true;
            }
        }

        // 2. Match by path
        Optional<Page> byPath = pageRepository.findByTypeDirPathFileName(loc.type(), loc.dirPath(), loc.name());
        if (byPath.isPresent()) {
            Page page = byPath.get();
            if (fileId == null) {
                writeBack = true; // hand-made file: give it our id
            } else if (!page.id().equals(fileId)) {
                // Two devices created the same page independently (typical for today's journal).
                // Both must end up with one id, so the older id wins on every device.
                if (fileId.compareTo(page.id()) < 0) {
                    page = adoptPageId(page, fileId, relPath);
                } else {
                    writeBack = true; // our id is older: rewrite the file with it
                }
            }
            return new PageMatch(page, false, false, writeBack);
        }

        // 3. New page
        HMTID id = fileId != null ? fileId : idGenerator.generatePageId();
        if (fileId == null) writeBack = true;
        Instant created = parseInstant(frontmatter.get("created")).orElse(fileModified);
        Page page = save(new Page(id, null, loc.dirPath(), loc.name(), loc.type(), created, fileModified), loc, fileModified);
        LOG.infov("Created page {0} from {1}", page.id(), relPath);
        return new PageMatch(page, true, false, writeBack);
    }

    /**
     * Saves a page at the location of its file, creating the index pages above it first,
     * and reindexes its items under the new title.
     *
     * @param page     the page to move
     * @param loc      the new location
     * @param modified the modified date to store
     * @return the saved page
     */
    public Page move(Page page, PageLocation loc, Instant modified) {
        Page moved = save(page, loc, modified);
        searchService.indexItems(itemRepository.findByPageId(moved.id()), moved.dirPath() + moved.name());
        return moved;
    }

    /**
     * Saves a page with a new modified date at a location whose index pages exist already.
     *
     * @param page     the page to save
     * @param loc      its location
     * @param modified the modified date to store
     * @return the saved page
     */
    public Page touch(Page page, PageLocation loc, Instant modified) {
        Page saved = pageRepository.save(new Page(page.id(), null, loc.dirPath(), loc.name(), loc.type(),
                page.createdDate(), modified));
        searchService.indexPage(saved);
        return saved;
    }

    private Page save(Page page, PageLocation loc, Instant modified) {
        if ("doc".equals(loc.type()) && !loc.dirPath().isEmpty() && !"index".equals(loc.name())) {
            pageService.ensureAncestorPages(loc.dirPath());
        }
        return touch(page, loc, modified);
    }

    /** Gives a page the id another device chose for it and reindexes it under that id. */
    private Page adoptPageId(Page page, HMTID newId, String relPath) {
        LOG.infov("Adopting page id {0} from {1} (was {2})", newId, relPath, page.id());
        searchService.removePage(page.id().toString());
        pageRepository.rekey(page.id(), newId);
        Page rekeyed = pageRepository.findById(newId).orElseThrow();
        searchService.indexPage(rekeyed);
        searchService.indexItems(itemRepository.findByPageId(rekeyed.id()), rekeyed.dirPath() + rekeyed.name());
        return rekeyed;
    }

    static HMTID parseId(String s) {
        if (s == null || s.isEmpty()) return null;
        try {
            return HMTID.of(s);
        } catch (IllegalArgumentException e) {
            return null;
        }
    }

    static Optional<Instant> parseInstant(String s) {
        if (s == null || s.isEmpty()) return Optional.empty();
        try {
            return Optional.of(Instant.parse(s));
        } catch (Exception e) {
            return Optional.empty();
        }
    }
}
