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

    public Page createPage(String title, String dirPath, List<String> tags) {
        // Validate: reject empty or whitespace-only titles
        if (title == null || title.trim().isEmpty()) {
            throw new IllegalArgumentException("Page title cannot be empty or whitespace-only");
        }

        // Validate: reject titles with leading or trailing whitespace
        if (!title.equals(title.trim())) {
            throw new IllegalArgumentException("Page title cannot have leading or trailing whitespace");
        }

        String dir = (dirPath == null || dirPath.isEmpty()) ? "pages" : dirPath;

        String fileName = sanitizeFileName(title);

        // Check for duplicate by exact title match
        var existing = pageRepository.findByExactTitle(title);
        if (existing.isPresent()) {
            return existing.get();
        }

        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        Page page = new Page(id, dir, fileName, now, now, tags != null ? tags : List.of(), "{}");
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create initial empty block
        String blockId = BlockIdGenerator.generate();
        Block block = new Block(blockId, id, page.dirPath(), page.fileName(), now, now, FractionalIndexer.initial(), 0, "", "", false, "normal", List.of(), "{}");
        blockRepository.save(block);

        // Queue for export
        exportService.queueExport(id);

        return page;
    }

    public Page updatePage(UUID pageId, String title, List<String> tags, String propertiesJson) {
        Page existing = pageRepository.findById(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        // If title is provided, use it as the new fileName (after sanitization)
        String fileName = existing.fileName();
        if (title != null) {
            // Validate and sanitize the new title
            if (title.trim().isEmpty()) {
                throw new IllegalArgumentException("Page title cannot be empty or whitespace-only");
            }
            if (!title.equals(title.trim())) {
                throw new IllegalArgumentException("Page title cannot have leading or trailing whitespace");
            }
            fileName = sanitizeFileName(title);
        }

        Page updated = new Page(
                existing.id(),
                existing.dirPath(),
                fileName,
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

    private static String sanitizeFileName(String title) {
        return title.replaceAll("[<>:\"/\\\\|?*]", "");
    }
}
