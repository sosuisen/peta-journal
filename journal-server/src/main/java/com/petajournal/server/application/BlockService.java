package com.petajournal.server.application;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.domain.BlockIdGenerator;
import com.petajournal.server.domain.FractionalIndexer;
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
    com.petajournal.server.infrastructure.export.MarkdownExportService exportService;

    @Inject
    TreeMarkdownConverter treeMarkdownConverter;

    public Block createBlock(UUID pageId, String afterBlockId, int level, String body) {
        String id = BlockIdGenerator.generate();
        Instant now = Instant.now();
        String html = parseBody(body);

        List<Block> existingBlocks = blockRepository.findByPageId(pageId);
        String idx = calculateInsertionIndex(existingBlocks, afterBlockId);

        List<String> types = detectBlockTypes(body);
        String status = "normal";  // Default status

        // Get page to get dir_path and file_name
        Page page = pageRepository.findById(pageId).orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        Block block = new Block(id, pageId, page.dirPath(), page.fileName(), now, now, idx, level, body, html, false, status, types, "{}");
        block = blockRepository.save(block);

        // Update page modified date
        touchPage(pageId);

        // Index for search
        searchService.indexBlock(block, page.fileName());

        // Queue for export
        exportService.queueExport(pageId);

        return block;
    }

    public Block updateBlock(String blockId, String body, Integer level, Boolean collapsed, String status, String propertiesJson) {
        Block existing = blockRepository.findById(blockId)
                .orElseThrow(() -> new IllegalArgumentException("Block not found: " + blockId));

        String newBody = body != null ? body : existing.body();
        String newHtml = body != null ? parseBody(body) : existing.html();
        int newLevel = level != null ? level : existing.level();
        boolean newCollapsed = collapsed != null ? collapsed : existing.collapsed();
        String newStatus = status != null ? status : existing.status();
        String newProps = propertiesJson != null ? propertiesJson : existing.propertiesJson();
        List<String> types = body != null ? detectBlockTypes(body) : existing.types();

        Block updated = new Block(
                existing.id(),
                existing.pageId(),
                existing.dirPath(),
                existing.fileName(),
                existing.createdDate(),
                Instant.now(),
                existing.idx(),
                newLevel,
                newBody,
                newHtml,
                newCollapsed,
                newStatus,
                types,
                newProps
        );

        updated = blockRepository.save(updated);
        touchPage(existing.pageId());

        Page page = pageRepository.findById(existing.pageId()).orElse(null);
        if (page != null) {
            searchService.indexBlock(updated, page.fileName());
        }

        // Queue for export
        exportService.queueExport(existing.pageId());

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

        // Get target page to get dir_path and file_name
        Page targetPage = pageRepository.findById(targetPageId)
                .orElseThrow(() -> new IllegalArgumentException("Target page not found: " + targetPageId));

        for (String blockId : blockIds) {
            Block block = blockRepository.findById(blockId)
                    .orElseThrow(() -> new IllegalArgumentException("Block not found: " + blockId));

            String newIdx = FractionalIndexer.between(afterIdx, nextIdx);
            int newLevel = Math.max(0, block.level() + delta);

            Block moved = new Block(
                    block.id(),
                    targetPageId,
                    targetPage.dirPath(),
                    targetPage.fileName(),
                    block.createdDate(),
                    now,
                    newIdx,
                    newLevel,
                    block.body(),
                    block.html(),
                    block.collapsed(),
                    block.status(),
                    block.types(),
                    block.propertiesJson()
            );

            moved = blockRepository.save(moved);
            movedBlocks.add(moved);

            afterIdx = newIdx;
            nextIdx = null;
        }

        touchPage(targetPageId);

        // Queue for export
        exportService.queueExport(targetPageId);

        return movedBlocks;
    }

    public void deleteBlock(String blockId) {
        Optional<Block> block = blockRepository.findById(blockId);
        blockRepository.delete(blockId);
        searchService.removeBlock(blockId);
        block.ifPresent(b -> {
            touchPage(b.pageId());
            // Queue for export
            exportService.queueExport(b.pageId());
        });
    }

    /**
     * Update block status with validation.
     * Valid statuses: normal, todo, running, done
     */
    public List<Block> getBlocksByStatuses(List<String> statuses) {
        return blockRepository.findByStatuses(statuses);
    }

    public Block updateBlockStatus(String blockId, String status) {
        if (!List.of("normal", "todo", "running", "done").contains(status)) {
            throw new IllegalArgumentException("Invalid status: " + status);
        }
        return updateBlock(blockId, null, null, null, status, null);
    }

    public record TreeMarkdownExport(String treeMarkdown, String html, String flatMarkdown) {}

    public TreeMarkdownExport exportTreeMarkdown(List<String> blockIds) {
        List<Block> blocks = blockIds.stream()
            .map(id -> blockRepository.findById(id))
            .filter(Optional::isPresent)
            .map(Optional::get)
            .toList();
        String treeMd = treeMarkdownConverter.blocksToTreeMarkdown(blocks);
        String html = markdownParser.parseToHtml(treeMd);
        String flatMd = treeMarkdownConverter.blocksToFlatMarkdown(blocks);
        return new TreeMarkdownExport(treeMd, html, flatMd);
    }

    public String importTreeMarkdown(String treeMarkdown) {
        return treeMarkdownConverter.treeMarkdownToFlatMarkdown(treeMarkdown);
    }

    /**
     * Calculate the fractional index for inserting a block after the specified block.
     * If afterBlockId is null/empty, returns an index for the beginning of the list.
     */
    private String calculateInsertionIndex(List<Block> existingBlocks, String afterBlockId) {
        if (afterBlockId == null || afterBlockId.isEmpty()) {
            if (existingBlocks.isEmpty()) {
                return FractionalIndexer.initial();
            }
            return FractionalIndexer.between(null, existingBlocks.getFirst().idx());
        }

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
        return FractionalIndexer.between(afterIdx, nextIdx);
    }

    private void touchPage(UUID pageId) {
        pageRepository.findById(pageId).ifPresent(page -> {
            Page updated = new Page(
                    page.id(), page.dirPath(), page.fileName(),
                    page.createdDate(), Instant.now(),
                    page.tags(), page.propertiesJson()
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
                    .filter(p -> p.fileName().equalsIgnoreCase(pageName))
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
        if (trimmed.startsWith("#")) {
            types.add("heading");
        }
        if (trimmed.startsWith("```")) {
            types.add("code");
        }
        if (trimmed.startsWith(">")) {
            types.add("blockquote");
        }
        return types;
    }
}
