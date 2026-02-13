package com.petajournal.server.application;

import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import com.petajournal.server.domain.BlockIdGenerator;
import com.petajournal.server.domain.FractionalIndexer;
import com.petajournal.server.domain.JournalFileNameMatcher;

import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

@ApplicationScoped
public class PageService {

    @Inject
    PageRepository pageRepository;

    @Inject
    BlockRepository blockRepository;

    @Inject
    SearchService searchService;

    @Inject
    com.petajournal.server.infrastructure.export.MarkdownExportService exportService;

    public Optional<Page> getPage(UUID pageId) {
        return pageRepository.findById(pageId);
    }

    public List<Block> getPageBlocks(UUID pageId) {
        return blockRepository.findByPageId(pageId);
    }

    public record DocResult(Page page, List<Block> blocks, boolean created) {}

    public DocResult getOrCreateDoc(String title, String dirPath, List<String> tags) {
        // Validate: reject empty or whitespace-only titles
        if (title == null || title.trim().isEmpty()) {
            throw new IllegalArgumentException("Page title cannot be empty or whitespace-only");
        }

        // Validate: reject titles with leading or trailing whitespace
        if (!title.equals(title.trim())) {
            throw new IllegalArgumentException("Page title cannot have leading or trailing whitespace");
        }

        String dir = (dirPath == null) ? "" : dirPath;
        String fileName = title;

        validateFileName(fileName);

        // Reject journal-type names as doc titles
        if (JournalFileNameMatcher.isJournalFileName(fileName)) {
            throw new IllegalArgumentException("Cannot create a doc with a journal-type name: " + title);
        }

        // Check for duplicate using (type, dir_path, file_name) index
        var existing = pageRepository.findByTypeDirPathFileName("doc", dir, fileName);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Block> blocks = blockRepository.findByPageId(page.id());
            return new DocResult(page, blocks, false);
        }

        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        Page page = new Page(id, dir, fileName, "doc", now, now, tags != null ? tags : List.of(), "{}");
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create initial empty block
        String blockId = BlockIdGenerator.generate();
        Block block = new Block(blockId, id, page.dirPath(), page.fileName(), page.type(), now, now, FractionalIndexer.initial(), 0, "", "", false, "normal", List.of(), "{}");
        blockRepository.save(block);

        // Queue for export
        exportService.queueExport(id);

        return new DocResult(page, List.of(block), true);
    }

    public Page updatePage(UUID pageId, String title, List<String> tags, String propertiesJson, String dirPath) {
        Page existing = pageRepository.findById(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        String fileName = existing.fileName();
        if (title != null) {
            if (title.trim().isEmpty()) {
                throw new IllegalArgumentException("Page title cannot be empty or whitespace-only");
            }
            if (!title.equals(title.trim())) {
                throw new IllegalArgumentException("Page title cannot have leading or trailing whitespace");
            }
            validateFileName(title);
            fileName = title;
        }

        String newDirPath = existing.dirPath();
        if (dirPath != null) {
            newDirPath = dirPath;
        }

        Page updated = new Page(
                existing.id(),
                newDirPath,
                fileName,
                existing.type(),
                existing.createdDate(),
                Instant.now(),
                tags != null ? tags : existing.tags(),
                propertiesJson != null ? propertiesJson : existing.propertiesJson()
        );

        updated = pageRepository.save(updated);
        searchService.indexPage(updated);

        // Queue for export
        exportService.queueExport(pageId);

        return updated;
    }

    public void deletePage(UUID pageId) {
        searchService.removePage(pageId.toString());
        blockRepository.deleteByPageId(pageId);
        pageRepository.delete(pageId);
    }

    public List<Page> getRecentPages(int maxResults) {
        return pageRepository.findRecent(maxResults);
    }

    public List<BacklinkEntry> getBacklinks(UUID pageId) {
        List<Page> sources = pageRepository.findBacklinkSources(pageId);
        List<BacklinkEntry> entries = new ArrayList<>();

        Page targetPage = pageRepository.findById(pageId).orElse(null);
        if (targetPage == null) return entries;

        for (Page source : sources) {
            // Find the specific blocks that link to this page (using fileName as title)
            List<Block> linkingBlocks = blockRepository.findByPageIdContainingLink(targetPage.fileName());
            for (Block block : linkingBlocks) {
                if (block.pageId().equals(source.id())) {
                    entries.add(new BacklinkEntry(source, block));
                }
            }
        }
        return entries;
    }

    private static final java.util.regex.Pattern FORBIDDEN_CHARS = java.util.regex.Pattern.compile(
            "[<>:\"/\\\\|?*\\x00-\\x1F\\x7F]");
    private static final java.util.regex.Pattern RESERVED_NAMES = java.util.regex.Pattern.compile(
            "^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$", java.util.regex.Pattern.CASE_INSENSITIVE);

    static void validateFileName(String title) {
        var matcher = FORBIDDEN_CHARS.matcher(title);
        if (matcher.find()) {
            String ch = matcher.group();
            if (ch.charAt(0) < 0x20 || ch.charAt(0) == 0x7F) {
                throw new IllegalArgumentException("Title contains a control character (not allowed in filenames)");
            }
            throw new IllegalArgumentException("Title contains '" + ch + "' (not allowed in filenames)");
        }
        if (title.endsWith(".") || title.endsWith(" ")) {
            throw new IllegalArgumentException("Title cannot end with a dot or space");
        }
        if (RESERVED_NAMES.matcher(title).matches()) {
            throw new IllegalArgumentException("Title '" + title + "' is a reserved name on Windows");
        }
    }
}
