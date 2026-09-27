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

    public enum Outcome { IGNORED, MISSING, UNCHANGED, IMPORTED, CONFLICT, ERROR }

    @Inject PageRepository pageRepository;
    @Inject ItemRepository itemRepository;
    @Inject PropertyRepository propertyRepository;
    @Inject MarkdownParser markdownParser;
    @Inject SearchService searchService;
    @Inject OutlineMarkdownParser parser;
    @Inject IdGenerator idGenerator;
    @Inject PageService pageService;
    @Inject FileSyncStateRepository fileSyncStateRepository;
    @Inject MarkdownExportService exportService;
    @Inject SseBroadcaster sseBroadcaster;

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
        } catch (RuntimeException e) {
            LOG.errorv(e, "Failed to import {0}", relPath);
            return Outcome.ERROR;
        }
    }

    private Outcome importContent(Path root, String relPath, MarkdownPathMapper.PageLocation loc,
                                  String content, String hash, BasicFileAttributes attrs) {
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

        HMTID fileId = parseId(fm.get("id"));
        Instant fileModified = parseInstant(fm.get("modified")).orElse(attrs.lastModifiedTime().toInstant());
        Instant now = Instant.now();

        boolean writeBack = false; // file lacks ids or has a foreign id: rewrite it after import
        boolean renamed = false;
        Page page = null;

        // 1. Match by frontmatter id
        if (fileId != null) {
            Optional<Page> byId = pageRepository.findById(fileId);
            if (byId.isPresent()) {
                Page p = byId.get();
                String currentRel = MarkdownPathMapper.toRelPath(p);
                if (currentRel.equals(relPath)) {
                    page = p;
                } else if (Files.exists(root.resolve(currentRel))) {
                    // Same id in two files: this one is a copy, give it a new identity
                    LOG.infov("{0} is a copy of {1}, creating a new page", relPath, currentRel);
                    fileId = null;
                    writeBack = true;
                } else {
                    page = p;
                    renamed = true;
                    LOG.infov("Page {0} moved: {1} -> {2}", p.id(), currentRel, relPath);
                }
            }
        }

        // 2. Match by path
        if (page == null) {
            Optional<Page> byPath = pageRepository.findByTypeDirPathFileName(loc.type(), loc.dirPath(), loc.name());
            if (byPath.isPresent()) {
                page = byPath.get();
                if (fileId == null) {
                    writeBack = true; // hand-made file: give it our id
                } else if (!page.id().equals(fileId)) {
                    // Two devices created the same page independently (typical for today's journal).
                    // Both must end up with one id, so the older id wins on every device.
                    if (fileId.compareTo(page.id()) < 0) {
                        LOG.infov("Adopting page id {0} from {1} (was {2})", fileId, relPath, page.id());
                        searchService.removePage(page.id().toString());
                        pageRepository.rekey(page.id(), fileId);
                        page = pageRepository.findById(fileId).orElseThrow();
                        searchService.indexPage(page);
                        String title = page.dirPath() + page.name();
                        for (Item item : itemRepository.findByPageId(page.id())) {
                            searchService.indexItem(item, title);
                        }
                    } else {
                        writeBack = true; // our id is older: rewrite the file with it
                    }
                }
            }
        }

        // 3. New page
        boolean isNew = page == null;
        if (isNew) {
            HMTID id = fileId != null ? fileId : idGenerator.generatePageId();
            if (fileId == null) writeBack = true;
            Instant created = parseInstant(fm.get("created")).orElse(fileModified);
            ensureAncestors(loc);
            page = pageRepository.save(new Page(id, null, loc.dirPath(), loc.name(), loc.type(), created, fileModified));
            searchService.indexPage(page);
            LOG.infov("Created page {0} from {1}", page.id(), relPath);
        } else if (fileModified.isBefore(page.modifiedDate())) {
            // DB is newer: keep the DB, keep the file's content as a backup, then overwrite the file.
            // A moved file still moves the page, so the export lands at the new path.
            if (renamed) {
                ensureAncestors(loc);
                page = pageRepository.save(new Page(page.id(), null, loc.dirPath(), loc.name(), loc.type(),
                        page.createdDate(), page.modifiedDate()));
                searchService.indexPage(page);
            }
            backupConflict(root, relPath, loc, content, hash, page, attrs);
            exportService.queueExport(page.id());
            return Outcome.CONFLICT;
        }

        // 4. Merge items
        List<ParsedItem> items = parsed.items().isEmpty()
                ? List.of(new ParsedItem(null, 1, "", false, List.of(), Map.of(), null, null))
                : parsed.items();
        MergeResult merge = mergeItems(page, items, now);
        if (merge.needsWriteBack()) writeBack = true;

        // 5. Page metadata
        if (renamed || merge.changed() || isNew) {
            Instant newModified = isNew ? page.modifiedDate()
                    : fileModified.isAfter(page.modifiedDate()) ? fileModified : now;
            if (renamed) ensureAncestors(loc);
            page = pageRepository.save(new Page(page.id(), null, loc.dirPath(), loc.name(), loc.type(),
                    page.createdDate(), newModified));
            searchService.indexPage(page);
            if (renamed) {
                String title = page.dirPath() + page.name();
                for (Item item : itemRepository.findByPageId(page.id())) {
                    searchService.indexItem(item, title);
                }
            }
        }

        fileSyncStateRepository.upsertSynced(relPath, page.id(), hash, attrs.size(), attrs.lastModifiedTime().toMillis());

        if (writeBack) {
            // Give the file its ids / frontmatter. The export records the new hash, so it won't echo back.
            exportService.queueExport(page.id());
        }
        if (merge.changed() || renamed || isNew) {
            sseBroadcaster.broadcastPagesUpdated(page.id().toString(), "file");
            LOG.infov("Imported {0} ({1} items, {2})", relPath, items.size(),
                    isNew ? "new page" : renamed ? "renamed" : "updated");
        }
        return Outcome.IMPORTED;
    }

    /** @param needsWriteBack the file lacks an id or idx, or its idx comments disagree with the line order */
    private record MergeResult(boolean changed, boolean needsWriteBack) {}

    private MergeResult mergeItems(Page page, List<ParsedItem> parsedItems, Instant now) {
        HMTID pageId = page.id();
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
        Map<String, Map<String, String>> scalars = propertyRepository.findScalarByTargets("item", existingIds);
        boolean changed = false;
        for (Item old : existing) {
            if (seen.contains(old.id())) continue;
            itemRepository.delete(old.id());
            searchService.removeItem(old.id().toString());
            propertyRepository.syncTags("item", old.id().toString(), List.of());
            for (String key : scalars.getOrDefault(old.id().toString(), Map.of()).keySet()) {
                propertyRepository.deleteProperty("item", old.id().toString(), key);
            }
            changed = true;
        }

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

        for (int i = 0; i < n; i++) {
            ParsedItem pi = parsedItems.get(i);
            HMTID id = ids.get(i);
            String idx = idxs.get(i);
            Item old = byId.get(id);

            String body = pi.body() == null ? "" : pi.body();
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
                    propertyRepository.syncTags("item", id.toString(), HashtagExtractor.extractHashtags(ast));
                }
                searchService.indexItem(item, title);
                changed = true;
            }

            // Scalar properties from <!-- key=value --> comments
            Map<String, String> want = new LinkedHashMap<>();
            for (var e : pi.properties().entrySet()) {
                if (RESERVED_KEYS.contains(e.getKey())) continue;
                String type = defs.getOrDefault(e.getKey(), "text");
                if ("list".equals(type) || "tags".equals(type)) continue;
                want.put(e.getKey(), e.getValue());
            }
            Map<String, String> have = scalars.getOrDefault(id.toString(), Map.of());
            for (var e : want.entrySet()) {
                if (!e.getValue().equals(have.get(e.getKey()))) {
                    String type = defs.getOrDefault(e.getKey(), "text");
                    try {
                        propertyRepository.upsertScalar("item", id.toString(), e.getKey(), type, e.getValue());
                        changed = true;
                    } catch (RuntimeException ex) {
                        LOG.warnv("Skipping property {0}={1} on item {2}: {3}", e.getKey(), e.getValue(), id, ex.getMessage());
                    }
                }
            }
            for (String key : have.keySet()) {
                if (!want.containsKey(key)) {
                    propertyRepository.deleteProperty("item", id.toString(), key);
                    changed = true;
                }
            }
        }

        if (changed) {
            itemRepository.recalculateParentIds(pageId);
        }
        return new MergeResult(changed, needsWriteBack);
    }

    private void backupConflict(Path root, String relPath, MarkdownPathMapper.PageLocation loc, String content,
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

    private void ensureAncestors(MarkdownPathMapper.PageLocation loc) {
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
