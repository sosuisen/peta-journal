package com.petajournal.server.application;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.domain.repository.SyncService;
import com.petajournal.server.infrastructure.persistence.BlockIdGenerator;
import com.petajournal.server.infrastructure.persistence.FractionalIndexer;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

@ApplicationScoped
public class BlockService {

    @Inject
    BlockRepository blockRepository;

    @Inject
    PageRepository pageRepository;

    @Inject
    MarkdownParser markdownParser;

    @Inject
    SearchService searchService;

    @Inject
    SyncService syncService;

    public Block createBlock(UUID pageId, String afterBlockId, int level, String body) {
        String id = BlockIdGenerator.generate();
        Instant now = Instant.now();
        String html = parseBody(body);

        // Calculate fractional index
        List<Block> existingBlocks = blockRepository.findByPageId(pageId);
        String idx;
        if (afterBlockId == null || afterBlockId.isEmpty()) {
            // Insert at beginning
            if (existingBlocks.isEmpty()) {
                idx = FractionalIndexer.initial();
            } else {
                idx = FractionalIndexer.between(null, existingBlocks.getFirst().idx());
            }
        } else {
            // Insert after specified block
            String afterIdx = null;
            String nextIdx = null;
            for (int i = 0; i < existingBlocks.size(); i++) {
                if (existingBlocks.get(i).id().equals(afterBlockId)) {
                    afterIdx = existingBlocks.get(i).idx();
                    if (i + 1 < existingBlocks.size()) {
                        nextIdx = existingBlocks.get(i + 1).idx();
                    }
                    break;
                }
            }
            idx = FractionalIndexer.between(afterIdx, nextIdx);
        }

        List<String> types = detectBlockTypes(body);

        Block block = new Block(id, pageId, now, now, idx, level, body, html, false, types, "{}");
        block = blockRepository.save(block);

        // Update page modified date
        touchPage(pageId);

        // Index for search
        Page page = pageRepository.findById(pageId).orElse(null);
        if (page != null) {
            searchService.indexBlock(block, page.primaryTitle());
        }

        syncService.notifyChange("block", id);
        return block;
    }

    public Block updateBlock(String blockId, String body, Integer level, Boolean collapsed, String propertiesJson) {
        Block existing = blockRepository.findById(blockId)
                .orElseThrow(() -> new IllegalArgumentException("Block not found: " + blockId));

        String newBody = body != null ? body : existing.body();
        String newHtml = body != null ? parseBody(body) : existing.html();
        int newLevel = level != null ? level : existing.level();
        boolean newCollapsed = collapsed != null ? collapsed : existing.collapsed();
        String newProps = propertiesJson != null ? propertiesJson : existing.propertiesJson();
        List<String> types = body != null ? detectBlockTypes(body) : existing.types();

        Block updated = new Block(
                existing.id(),
                existing.pageId(),
                existing.createdDate(),
                Instant.now(),
                existing.idx(),
                newLevel,
                newBody,
                newHtml,
                newCollapsed,
                types,
                newProps
        );

        updated = blockRepository.save(updated);
        touchPage(existing.pageId());

        Page page = pageRepository.findById(existing.pageId()).orElse(null);
        if (page != null) {
            searchService.indexBlock(updated, page.primaryTitle());
        }

        syncService.notifyChange("block", blockId);
        return updated;
    }

    public List<Block> moveBlocks(List<String> blockIds, UUID targetPageId, String afterBlockId, Integer levelDelta) {
        List<Block> movedBlocks = new ArrayList<>();
        List<Block> targetBlocks = blockRepository.findByPageId(targetPageId);

        // Find insertion point
        String afterIdx = null;
        String nextIdx = null;
        if (afterBlockId != null && !afterBlockId.isEmpty()) {
            for (int i = 0; i < targetBlocks.size(); i++) {
                if (targetBlocks.get(i).id().equals(afterBlockId)) {
                    afterIdx = targetBlocks.get(i).idx();
                    if (i + 1 < targetBlocks.size()) {
                        nextIdx = targetBlocks.get(i + 1).idx();
                    }
                    break;
                }
            }
        } else if (!targetBlocks.isEmpty()) {
            nextIdx = targetBlocks.getFirst().idx();
        }

        int delta = levelDelta != null ? levelDelta : 0;
        Instant now = Instant.now();

        for (String blockId : blockIds) {
            Block block = blockRepository.findById(blockId)
                    .orElseThrow(() -> new IllegalArgumentException("Block not found: " + blockId));

            String newIdx = FractionalIndexer.between(afterIdx, nextIdx);
            int newLevel = Math.max(0, block.level() + delta);

            Block moved = new Block(
                    block.id(),
                    targetPageId,
                    block.createdDate(),
                    now,
                    newIdx,
                    newLevel,
                    block.body(),
                    block.html(),
                    block.collapsed(),
                    block.types(),
                    block.propertiesJson()
            );

            moved = blockRepository.save(moved);
            movedBlocks.add(moved);

            afterIdx = newIdx;
            nextIdx = null;
        }

        touchPage(targetPageId);
        return movedBlocks;
    }

    public void deleteBlock(String blockId) {
        Optional<Block> block = blockRepository.findById(blockId);
        blockRepository.delete(blockId);
        searchService.removeBlock(blockId);
        block.ifPresent(b -> {
            touchPage(b.pageId());
            syncService.notifyChange("block", blockId);
        });
    }

    private void touchPage(UUID pageId) {
        pageRepository.findById(pageId).ifPresent(page -> {
            Page updated = new Page(
                    page.id(), page.dirPath(), page.fileName(),
                    page.createdDate(), Instant.now(),
                    page.tags(), page.titles(), page.propertiesJson()
            );
            pageRepository.save(updated);
        });
    }

    private static final Pattern WIKI_LINK_PATTERN = Pattern.compile("\\[\\[([^\\]]+)\\]\\]");

    /**
     * Replace [[pagename]] in HTML with hyperlinks.
     * If the page exists, link to /pages/{id}. Otherwise, mark as new.
     */
    private String processWikiLinks(String html) {
        if (html == null || html.isEmpty()) return html;
        Matcher matcher = WIKI_LINK_PATTERN.matcher(html);
        StringBuilder sb = new StringBuilder();
        while (matcher.find()) {
            String pageName = matcher.group(1);
            List<Page> pages = pageRepository.findByTitle(pageName);
            Optional<Page> exactMatch = pages.stream()
                    .filter(p -> p.primaryTitle().equalsIgnoreCase(pageName))
                    .findFirst();
            String replacement;
            if (exactMatch.isPresent()) {
                replacement = "<a class=\"wiki-link\" data-page-id=\"" + exactMatch.get().id() + "\">" + pageName + "</a>";
            } else {
                replacement = "<a class=\"wiki-link wiki-link-new\" data-page-title=\"" + pageName + "\">" + pageName + "</a>";
            }
            matcher.appendReplacement(sb, Matcher.quoteReplacement(replacement));
        }
        matcher.appendTail(sb);
        return sb.toString();
    }

    private String parseBody(String body) {
        String html = markdownParser.parseToHtml(body);
        return processWikiLinks(html);
    }

    private List<String> detectBlockTypes(String body) {
        List<String> types = new ArrayList<>();
        if (body == null) return types;
        String trimmed = body.trim();
        if (trimmed.startsWith("- [ ]") || trimmed.startsWith("- [x]") || trimmed.startsWith("- [/]")) {
            types.add("task");
        }
        if (trimmed.startsWith("#")) {
            types.add("heading");
        }
        if (trimmed.startsWith("```")) {
            types.add("code");
        }
        return types;
    }
}
