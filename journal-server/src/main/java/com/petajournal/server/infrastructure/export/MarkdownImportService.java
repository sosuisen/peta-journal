package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.model.Page;
import com.petajournal.server.infrastructure.export.ItemMerger.MergeResult;
import com.petajournal.server.infrastructure.export.MarkdownPathMapper.PageLocation;
import com.petajournal.server.infrastructure.export.OutlineMarkdownParser.ParseResult;
import com.petajournal.server.infrastructure.export.OutlineMarkdownParser.ParsedItem;
import com.petajournal.server.infrastructure.export.PageMatcher.PageMatch;
import com.petajournal.server.presentation.rest.SseBroadcaster;
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
import java.util.List;
import java.util.Map;
import java.util.Optional;

/**
 * Brings changes made to markdown files (by hand, by another tool, or by Dropbox
 * from another device) into the database.
 *
 * Rules:
 * <ul>
 *   <li>A file whose content hash matches {@code file_sync_state} is our own write: skipped.</li>
 *   <li>Pages are matched by frontmatter id first, then by path; see {@link PageMatcher}.</li>
 *   <li>Last writer wins by {@code modified}: when the DB is newer than the file, the file is backed
 *       up as {@code name.conflict-yyyyMMdd-HHmmss.md} and overwritten from the DB.</li>
 *   <li>Items are merged by id; see {@link ItemMerger}.</li>
 *   <li>Nothing is deleted from the DB because a file disappeared; that goes through the
 *       pending-deletes page.</li>
 * </ul>
 */
@ApplicationScoped
public class MarkdownImportService {

    private static final Logger LOG = Logger.getLogger(MarkdownImportService.class);
    private static final DateTimeFormatter CONFLICT_TS = DateTimeFormatter.ofPattern("yyyyMMdd-HHmmss");

    public enum Outcome { IGNORED, MISSING, UNCHANGED, IMPORTED, CONFLICT, ERROR }

    private final OutlineMarkdownParser parser;
    private final PageMatcher pageMatcher;
    private final ItemMerger itemMerger;
    private final FileSyncStateRepository fileSyncStateRepository;
    private final MarkdownExportService exportService;
    private final SseBroadcaster sseBroadcaster;

    @Inject
    public MarkdownImportService(
            OutlineMarkdownParser parser,
            PageMatcher pageMatcher,
            ItemMerger itemMerger,
            FileSyncStateRepository fileSyncStateRepository,
            MarkdownExportService exportService,
            SseBroadcaster sseBroadcaster) {
        this.parser = parser;
        this.pageMatcher = pageMatcher;
        this.itemMerger = itemMerger;
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

        Instant fileModified = PageMatcher.parseInstant(fm.get("modified")).orElse(attrs.lastModifiedTime().toInstant());
        Instant now = Instant.now();

        PageMatch match = pageMatcher.match(root, relPath, loc, fm, fileModified);
        Page page = match.page();

        if (!match.isNew() && fileModified.isBefore(page.modifiedDate())
                && !fileSyncStateRepository.findByPageId(page.id()).isEmpty()) {
            // DB is newer: keep the DB, keep the file's content as a backup, then overwrite the file.
            // A moved file still moves the page, so the export lands at the new path.
            // A page that never had a file on this device (e.g. the root index a fresh install creates)
            // is not "newer" in any meaningful way: its modified date is just its creation time,
            // so the file wins and the check above is skipped.
            if (match.renamed()) {
                page = pageMatcher.move(page, loc, page.modifiedDate());
            }
            backupConflict(root, relPath, loc, content, hash, page, attrs);
            exportService.queueExport(page.id());
            return Outcome.CONFLICT;
        }

        List<ParsedItem> items = parsed.items().isEmpty()
                ? List.of(new ParsedItem(null, 1, "", false, List.of(), Map.of(), null, null))
                : parsed.items();
        MergeResult merge = itemMerger.merge(page, items, now);

        boolean pageChanged = match.renamed() || merge.changed() || match.isNew();
        if (pageChanged) {
            Instant newModified = match.isNew() ? page.modifiedDate()
                    : fileModified.isAfter(page.modifiedDate()) ? fileModified : now;
            page = match.renamed() ? pageMatcher.move(page, loc, newModified) : pageMatcher.touch(page, loc, newModified);
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
}
