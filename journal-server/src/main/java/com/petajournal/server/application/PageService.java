package com.petajournal.server.application;

import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.PageTitle;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.domain.repository.SyncService;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import com.petajournal.server.infrastructure.persistence.BlockIdGenerator;
import com.petajournal.server.infrastructure.persistence.FractionalIndexer;

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
    SyncService syncService;

    public Optional<Page> getPage(UUID pageId) {
        return pageRepository.findById(pageId);
    }

    public List<Block> getPageBlocks(UUID pageId) {
        return blockRepository.findByPageId(pageId);
    }

    public Page createPage(String title, String dirPath, List<String> tags) {
        String dir = (dirPath == null || dirPath.isEmpty()) ? "pages" : dirPath;
        String fileName = title.toLowerCase().replaceAll("[^a-z0-9\\-]", "-").replaceAll("-+", "-");

        // Check for duplicate by exact title match
        var existing = pageRepository.findByExactTitle(title);
        if (existing.isPresent()) {
            return existing.get();
        }

        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        List<PageTitle> titles = new ArrayList<>();
        titles.add(new PageTitle(id, 0, title));

        Page page = new Page(id, dir, fileName, now, now, tags != null ? tags : List.of(), titles, "{}");
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create initial empty block
        String blockId = BlockIdGenerator.generate();
        Block block = new Block(blockId, id, now, now, FractionalIndexer.initial(), 0, "", "", false, List.of(), "{}");
        blockRepository.save(block);

        syncService.notifyChange("page", id.toString());
        return page;
    }

    public Page updatePage(UUID pageId, String title, List<String> tags, String propertiesJson) {
        Page existing = pageRepository.findById(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        List<PageTitle> titles = existing.titles();
        if (title != null) {
            titles = new ArrayList<>();
            titles.add(new PageTitle(pageId, 0, title));
        }

        Page updated = new Page(
                existing.id(),
                existing.dirPath(),
                existing.fileName(),
                existing.createdDate(),
                Instant.now(),
                tags != null ? tags : existing.tags(),
                titles,
                propertiesJson != null ? propertiesJson : existing.propertiesJson()
        );

        updated = pageRepository.save(updated);
        searchService.indexPage(updated);
        syncService.notifyChange("page", pageId.toString());
        return updated;
    }

    public void deletePage(UUID pageId) {
        searchService.removePage(pageId.toString());
        blockRepository.deleteByPageId(pageId);
        pageRepository.delete(pageId);
        syncService.notifyChange("page", pageId.toString());
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
            // Find the specific blocks that link to this page
            for (String title : targetPage.titles().stream().map(PageTitle::title).toList()) {
                List<Block> linkingBlocks = blockRepository.findByPageIdContainingLink(title);
                for (Block block : linkingBlocks) {
                    if (block.pageId().equals(source.id())) {
                        entries.add(new BacklinkEntry(source, block));
                    }
                }
            }
        }
        return entries;
    }
}
