package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.HashtagExtractor;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.export.MarkdownPathMapper.PageLocation;
import com.petajournal.server.infrastructure.export.OutlineMarkdownParser.ParseResult;
import com.petajournal.server.infrastructure.export.OutlineMarkdownParser.ParsedItem;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.petajournal.server.presentation.rest.SseBroadcaster;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.nio.file.attribute.BasicFileAttributes;
import java.time.Instant;
import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.*;

/**
 * Brings changes made to markdown files (by hand, by another tool, or by Dropbox
 * from another device) into the database.
 *
 * Rules:
 * <ul>
 *   <li>A file whose content hash matches {@code file_sync_state} is our own write: skipped.</li>
 *   <li>Pages are matched by frontmatter id first, then by path. Same id at a new path is a rename;
 *       same id at two paths is a copy (the copy becomes a new page).</li>
 *   <li>Last writer wins by {@code modified}: when the DB is newer than the file, the file is backed
 *       up as {@code name.conflict-yyyyMMdd-HHmmss.md} and overwritten from the DB.</li>
 *   <li>Items are merged by id. Unchanged items are left alone (including their index).</li>
 *   <li>Nothing is deleted from the DB because a file disappeared; that goes through the
 *       pending-deletes page.</li>
 * </ul>
 */
@ApplicationScoped
public class MarkdownImportService {

    private static final Logger LOG = Logger.getLogger(MarkdownImportService.class);
    private static final DateTimeFormatter CONFLICT_TS = DateTimeFormatter.ofPattern("yyyyMMdd-HHmmss");
    private static final Set<String> RESERVED_KEYS = Set.of("level", "collapsed", "transclude", "tag", "idx");
    private static final String ITEM_TARGET = "item";

    public enum Outcome { IGNORED, MISSING, UNCHANGED, IMPORTED, CONFLICT, ERROR }

    private final PageRepository pageRepository;
    private final ItemRepository itemRepository;
    private final PropertyRepository propertyRepository;
    private final MarkdownParser markdownParser;
    private final SearchService searchService;
    private final OutlineMarkdownParser parser;
    private final IdGenerator idGenerator;
    private final PageService pageService;
    private final FileSyncStateRepository fileSyncStateRepository;
    private final MarkdownExportService exportService;
    private final SseBroadcaster sseBroadcaster;

    @Inject
    public MarkdownImportService(
            PageRepository pageRepository,
            ItemRepository itemRepository,
            PropertyRepository propertyRepository,
            MarkdownParser markdownParser,
            SearchService searchService,
            OutlineMarkdownParser parser,
            IdGenerator idGenerator,
            PageService pageService,
            FileSyncStateRepository fileSyncStateRepository,
            MarkdownExportService exportService,
            SseBroadcaster sseBroadcaster) {
        this.pageRepository = pageRepository;
        this.itemRepository = itemRepository;
        this.propertyRepository = propertyRepository;
        this.markdownParser = markdownParser;
        this.searchService = searchService;
        this.parser = parser;
        this.idGenerator = idGenerator;
        this.pageService = pageService;
        this.fileSyncStateRepository = fileSyncStateRepository;
        this.exportService = exportService;
        this.sseBroadcaster = sseBroadcaster;
    }

    /**
     * Syncs one file from the folder into the DB. Safe to call repeatedly.
     *
     * @param root    the markdown directory
     * @param relPath path relative to root, '/' separated
     */
    public synchronized Outcome syncFile(Path root, String relPath) {
        if (MarkdownPathMapper.isIgnored(relPath)) return Outcome.IGNORED;
        var locOpt = MarkdownPathMapper.fromRelPath(relPath);
        if (locOpt.isEmpty()) return Outcome.IGNORED;

        Path file = root.resolve(relPath);
        if (!Files.isRegularFile(file)) return Outcome.MISSING;

        String content;
        BasicFileAttributes attrs;
        try {
            content = readStable(file);
            attrs = Files.readAttributes(file, BasicFileAttributes.class);
        } catch (IOException e) {
            LOG.warnv("Cannot read {0}: {1}", file, e.getMessage());
            return Outcome.ERROR;
        }

        String hash = MarkdownPathMapper.sha256(content);
        Optional<FileSyncState> state = fileSyncStateRepository.findByPath(relPath);
        if (state.isPresent() && hash.equals(state.get().contentHash())) {
            // Our own write (or an already imported version) — just refresh the bookkeeping
            if (state.get().isMissing() || state.get().fileSize() != attrs.size()
                    || state.get().fileMtime() != attrs.lastModifiedTime().toMillis()) {
                fileSyncStateRepository.upsertSynced(relPath, state.get().pageId(), hash,
                        attrs.size(), attrs.lastModifiedTime().toMillis());
            }
            return Outcome.UNCHANGED;
        }

        try {
            return importContent(root, relPath, locOpt.get(), content, hash, attrs);
        } catch (RuntimeException | Error e) {
            // Error too: a native library that fails to load (UnsatisfiedLinkError) must not kill the scan silently
            LOG.errorv(e, "Failed to import {0}", relPath);
            return Outcome.ERROR;
        }
    }

    /**
     * The page a file belongs to, as decided from its frontmatter id and its path.
     *
     * @param page      the page, already saved when it is new
     * @param isNew     the page was created for this file
     * @param renamed   the page exists but its file moved to this path
     * @param writeBack the file lacks our id or carries a foreign one and must be rewritten after the import
     */
    record PageMatch(Page page, boolean isNew, boolean renamed, boolean writeBack) {}

    private Outcome importContent(Path root, String relPath, PageLocation loc,
                                  String content, String hash, BasicFileAttributes attrs) {
        long startedAt = System.currentTimeMillis();
        if (content.isBlank()) {
            // Sync tools and some editors create an empty file first and fill it in later.
            // Don't create or wipe a page for that; the watcher retries and the scan picks it up later.
            LOG.debugv("{0} is empty, waiting for content", relPath);
            return Outcome.ERROR;
        }

        // Editors on Windows may save CRLF; the metadata comments are matched per line
        content = content.replace("\r\n", "\n");
        ParseResult parsed = parser.parse(content);
        Map<String, String> fm = parsed.frontmatter();

        Instant fileModified = parseInstant(fm.get("modified")).orElse(attrs.lastModifiedTime().toInstant());
        Instant now = Instant.now();

        PageMatch match = matchPage(root, relPath, loc, fm, fileModified);
        Page page = match.page();

        if (!match.isNew() && fileModified.isBefore(page.modifiedDate())
                && !fileSyncStateRepository.findByPageId(page.id()).isEmpty()) {
            // DB is newer: keep the DB, keep the file's content as a backup, then overwrite the file.
            // A moved file still moves the page, so the export lands at the new path.
            // A page that never had a file on this device (e.g. the root index a fresh install creates)
            // is not "newer" in any meaningful way: its modified date is just its creation time,
            // so the file wins and the check above is skipped.
            if (match.renamed()) {
                page = savePageAt(page, loc, page.modifiedDate());
            }
            backupConflict(root, relPath, loc, content, hash, page, attrs);
            exportService.queueExport(page.id());
            return Outcome.CONFLICT;
        }

        // Merge items
        List<ParsedItem> items = parsed.items().isEmpty()
                ? List.of(new ParsedItem(null, 1, "", false, List.of(), Map.of(), null, null))
                : parsed.items();
        MergeResult merge = mergeItems(page, items, now);

        // Page metadata
        boolean pageChanged = match.renamed() || merge.changed() || match.isNew();
        if (pageChanged) {
            Instant newModified = match.isNew() ? page.modifiedDate()
                    : fileModified.isAfter(page.modifiedDate()) ? fileModified : now;
            page = match.renamed() ? savePageAt(page, loc, newModified) : savePage(page, loc, newModified);
            if (match.renamed()) {
                searchService.indexItems(itemRepository.findByPageId(page.id()), page.dirPath() + page.name());
            }
        }

        fileSyncStateRepository.upsertSynced(relPath, page.id(), hash, attrs.size(), attrs.lastModifiedTime().toMillis());

        if (match.writeBack() || merge.needsWriteBack()) {
            // Give the file its ids / frontmatter. The export records the new hash, so it won't echo back.
            exportService.queueExport(page.id());
        }
        if (pageChanged) {
            sseBroadcaster.broadcastPagesUpdated(page.id().toString(), "file");
            LOG.infov("Imported {0} ({1} items, {2}, {3} ms)", relPath, items.size(),
                    match.isNew() ? "new page" : match.renamed() ? "renamed" : "updated", System.currentTimeMillis() - startedAt);
        }
        return Outcome.IMPORTED;
    }

    /**
     * Finds the page a file belongs to, or creates one.
     * <ol>
     *   <li>By frontmatter id: same path is the page itself; a new path is a rename, unless the old
     *       file still exists, in which case this file is a copy and gets a page of its own.</li>
     *   <li>By path: a file without an id gets ours. A file with another id comes from a device that
     *       created the same page independently; the older id wins on every device.</li>
     *   <li>Otherwise a new page is created, keeping the file's id when it has one.</li>
     * </ol>
     * Package-private for tests.
     *
     * @param root         the markdown directory
     * @param relPath      the file's path under root
     * @param loc          the page location decoded from relPath
     * @param fm           the file's front matter
     * @param fileModified the file's modified time, used for a new page
     */
    PageMatch matchPage(Path root, String relPath, PageLocation loc, Map<String, String> fm, Instant fileModified) {
        HMTID fileId = parseId(fm.get("id"));
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
        Instant created = parseInstant(fm.get("created")).orElse(fileModified);
        Page page = savePageAt(new Page(id, null, loc.dirPath(), loc.name(), loc.type(), created, fileModified),
                loc, fileModified);
        LOG.infov("Created page {0} from {1}", page.id(), relPath);
        return new PageMatch(page, true, false, writeBack);
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

    /** Saves the page at a (possibly new) location, creating the index pages above it first. */
    private Page savePageAt(Page page, PageLocation loc, Instant modified) {
        ensureAncestors(loc);
        return savePage(page, loc, modified);
    }

    private Page savePage(Page page, PageLocation loc, Instant modified) {
        Page saved = pageRepository.save(new Page(page.id(), null, loc.dirPath(), loc.name(), loc.type(),
                page.createdDate(), modified));
        searchService.indexPage(saved);
        return saved;
    }

    /** @param needsWriteBack the file lacks an id or idx, or its idx comments disagree with the line order */
    private record MergeResult(boolean changed, boolean needsWriteBack) {}

    private MergeResult mergeItems(Page page, List<ParsedItem> parsedItems, Instant now) {
        HMTID pageId = page.id();
        String pageRelPath = MarkdownPathMapper.toRelPath(page);
        String title = page.dirPath() + page.name();
        List<Item> existing = itemRepository.findByPageId(pageId);
        Map<HMTID, Item> byId = new HashMap<>();
        for (Item item : existing) byId.put(item.id(), item);

        // Resolve an id for every parsed item
        int n = parsedItems.size();
        List<HMTID> ids = new ArrayList<>(n);
        Set<HMTID> seen = new HashSet<>();
        boolean anyMissingId = false;
        for (ParsedItem pi : parsedItems) {
            HMTID id = parseId(pi.id());
            if (id != null && !seen.contains(id) && !byId.containsKey(id) && itemRepository.findById(id).isPresent()) {
                // Belongs to another page (copy-pasted between files): don't steal it
                id = null;
            }
            if (id == null || seen.contains(id)) {
                id = idGenerator.generateItemId();
                anyMissingId = true;
            }
            seen.add(id);
            ids.add(id);
        }

        // Decide ordering. The idx written in the file is the writer's DB value and is taken as-is,
        // so every device ends up with the same idx (needed for direct DB sync later). The line
        // order in the file still wins: idx comments that disagree with it are recomputed.
        List<String> wantedIdx = new ArrayList<>(n);
        for (int i = 0; i < n; i++) {
            String fileIdx = parsedItems.get(i).idx();
            Item old = byId.get(ids.get(i));
            wantedIdx.add(fileIdx != null ? fileIdx : old != null ? old.idx() : null);
        }
        List<String> idxs;
        try {
            idxs = ItemOrderPlanner.plan(wantedIdx);
        } catch (IllegalStateException e) {
            LOG.warnv("Fractional index exhausted on page {0}, reindexing all items", pageId);
            idxs = ItemOrderPlanner.reindexAll(n);
        }
        boolean needsWriteBack = anyMissingId;
        for (int i = 0; i < n; i++) {
            if (!idxs.get(i).equals(parsedItems.get(i).idx())) needsWriteBack = true;
        }

        // Items no longer in the file go first, so their idx values are free for the survivors
        List<String> existingIds = existing.stream().map(i -> i.id().toString()).toList();
        Map<String, Map<String, String>> scalars = propertyRepository.findScalarByTargets(ITEM_TARGET, existingIds);
        boolean changed = false;
        List<String> removedIds = new ArrayList<>();
        for (Item old : existing) {
            if (seen.contains(old.id())) continue;
            itemRepository.delete(old.id());
            removedIds.add(old.id().toString());
            changed = true;
        }
        propertyRepository.deleteAllForTargets(ITEM_TARGET, removedIds);
        searchService.removeItems(removedIds); // one Lucene commit for all of them

        // (page_id, idx) is unique: if a surviving item is about to take another survivor's current idx,
        // park every survivor on a temporary idx first. Rare (two devices inserted at the same spot).
        Map<String, HMTID> idxOwner = new HashMap<>();
        for (Item old : existing) {
            if (seen.contains(old.id())) idxOwner.put(old.idx(), old.id());
        }
        boolean collision = false;
        for (int i = 0; i < n; i++) {
            HMTID owner = idxOwner.get(idxs.get(i));
            if (owner != null && !owner.equals(ids.get(i))) collision = true;
        }
        if (collision) {
            for (Item old : existing) {
                if (seen.contains(old.id())) itemRepository.updateIndex(old.id(), "\u007f" + old.idx());
            }
        }

        Map<String, String> defs = propertyRepository.findAllDefinitions();
        List<Item> toIndex = new ArrayList<>(); // indexed in one commit at the end

        for (int i = 0; i < n; i++) {
            ParsedItem pi = parsedItems.get(i);
            HMTID id = ids.get(i);
            String idx = idxs.get(i);
            Item old = byId.get(id);

            // The file holds relative Markdown links; the app stores wikilinks and hashtags
            String body = PageLinkConverter.toWikilinks(pi.body() == null ? "" : pi.body(), pageRelPath);
            HMTID transclude = parseId(pi.transcludeItemId());
            if (transclude == null && old != null && old.transcludeItemId() != null && body.equals(old.body())) {
                transclude = old.transcludeItemId(); // file written before transclude comments existed
            }
            List<String> types = pi.types();

            boolean same = old != null
                    && old.body().equals(body)
                    && old.level() == pi.level()
                    && old.collapsed() == pi.collapsed()
                    && sameTypes(old.types(), types)
                    && Objects.equals(old.transcludeItemId(), transclude)
                    && old.idx().equals(idx)
                    && !collision;

            if (!same) {
                boolean bodyChanged = old == null || !old.body().equals(body);
                String ast = bodyChanged ? markdownParser.parseToAst(body) : old.ast();
                Item item = new Item(id, pageId, null, page.dirPath(), page.name(), page.type(),
                        old != null ? old.createdDate() : now,
                        bodyChanged || old == null ? now : old.modifiedDate(),
                        idx, pi.level(), body, ast, pi.collapsed(), types, transclude);
                itemRepository.save(item);
                if (bodyChanged) {
                    propertyRepository.syncTags(ITEM_TARGET, id.toString(), HashtagExtractor.extractHashtags(ast));
                }
                toIndex.add(item);
                changed = true;
            }

            if (syncScalarProperties(id.toString(), pi.properties(), scalars.getOrDefault(id.toString(), Map.of()), defs)) {
                changed = true;
            }
        }

        searchService.indexItems(toIndex, title);
        if (changed) {
            itemRepository.recalculateParentIds(pageId);
        }
        return new MergeResult(changed, needsWriteBack);
    }

    /**
     * Makes the scalar properties of an item match the key="value" pairs of its metadata line.
     * Reserved keys and list-typed keys are not properties here.
     *
     * @return true if any property was written or removed
     */
    private boolean syncScalarProperties(String itemId, Map<String, String> fileProperties,
                                         Map<String, String> have, Map<String, String> defs) {
        Map<String, String> want = new LinkedHashMap<>();
        for (var e : fileProperties.entrySet()) {
            if (RESERVED_KEYS.contains(e.getKey())) continue;
            String type = defs.getOrDefault(e.getKey(), "text");
            if ("list".equals(type) || "tags".equals(type)) continue;
            want.put(e.getKey(), e.getValue());
        }
        boolean changed = false;
        for (var e : want.entrySet()) {
            if (!e.getValue().equals(have.get(e.getKey()))) {
                String type = defs.getOrDefault(e.getKey(), "text");
                try {
                    propertyRepository.upsertScalar(ITEM_TARGET, itemId, e.getKey(), type, e.getValue());
                    changed = true;
                } catch (RuntimeException ex) {
                    LOG.warnv("Skipping property {0}={1} on item {2}: {3}", e.getKey(), e.getValue(), itemId, ex.getMessage());
                }
            }
        }
        for (String key : have.keySet()) {
            if (!want.containsKey(key)) {
                propertyRepository.deleteProperty(ITEM_TARGET, itemId, key);
                changed = true;
            }
        }
        return changed;
    }

    private void backupConflict(Path root, String relPath, PageLocation loc, String content,
                                String hash, Page page, BasicFileAttributes attrs) {
        Path file = root.resolve(relPath);
        String ts = CONFLICT_TS.format(Instant.now().atZone(ZoneId.systemDefault()));
        Path backup = file.resolveSibling(MarkdownPathMapper.conflictFileName(loc.name(), ts));
        try {
            Files.writeString(backup, content, StandardCharsets.UTF_8,
                    StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING);
            LOG.warnv("{0} is older than the database version; kept it as {1} and restored from DB",
                    relPath, backup.getFileName());
        } catch (IOException e) {
            LOG.errorv("Could not write conflict backup {0}: {1}", backup, e.getMessage());
        }
        // Remember this content so we don't back it up again before the export overwrites it
        fileSyncStateRepository.upsertSynced(relPath, page.id(), hash, attrs.size(), attrs.lastModifiedTime().toMillis());
    }

    private void ensureAncestors(PageLocation loc) {
        if ("doc".equals(loc.type()) && !loc.dirPath().isEmpty() && !"index".equals(loc.name())) {
            pageService.ensureAncestorPages(loc.dirPath());
        }
    }

    /**
     * Reads the file, retrying briefly if it is still being written
     * (Dropbox and editors may create the file first and fill it in afterwards).
     */
    private String readStable(Path file) throws IOException {
        IOException last = null;
        for (int attempt = 0; attempt < 3; attempt++) {
            try {
                long size1 = Files.size(file);
                String content = Files.readString(file, StandardCharsets.UTF_8);
                if (Files.size(file) == size1) return content;
            } catch (IOException e) {
                last = e;
            }
            try {
                Thread.sleep(200L * (attempt + 1));
            } catch (InterruptedException ie) {
                Thread.currentThread().interrupt();
                break;
            }
        }
        if (last != null) throw last;
        return Files.readString(file, StandardCharsets.UTF_8);
    }

    private static boolean sameTypes(List<String> a, List<String> b) {
        return new HashSet<>(a == null ? List.of() : a).equals(new HashSet<>(b == null ? List.of() : b));
    }

    private static HMTID parseId(String s) {
        if (s == null || s.isEmpty()) return null;
        try {
            return HMTID.of(s);
        } catch (IllegalArgumentException e) {
            return null;
        }
    }

    private static Optional<Instant> parseInstant(String s) {
        if (s == null || s.isEmpty()) return Optional.empty();
        try {
            return Optional.of(Instant.parse(s));
        } catch (Exception e) {
            return Optional.empty();
        }
    }
}
