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
import org.jboss.logging.Logger;

import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.UUID;
import java.util.regex.Matcher;
import java.util.regex.Pattern;

@ApplicationScoped
public class BlockService {

    private static final Logger LOG = Logger.getLogger(BlockService.class);

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

    /**
     * Creates a new block on the specified page.
     *
     * @param pageId       the page to add the block to
     * @param afterBlockId the block ID after which to insert, or empty/null to prepend
     * @param level        the indentation level (1-based, 1=top level)
     * @param body         the markdown body text
     * @return the newly created block
     * @throws IllegalArgumentException if the page is not found
     */
    public Block createBlock(UUID pageId, UUID afterBlockId, int level, String body, UUID clientId) {
        UUID id = clientId != null ? clientId : BlockIdGenerator.generate();
        Instant now = Instant.now();
        String html = parseBody(body);

        List<Block> existingBlocks = blockRepository.findByPageId(pageId);
        String idx;
        try {
            idx = calculateInsertionIndex(existingBlocks, afterBlockId);
        } catch (IllegalStateException e) {
            // Fractional index exhausted — reindex and retry
            reindexPage(pageId, existingBlocks);
            existingBlocks = blockRepository.findByPageId(pageId);
            idx = calculateInsertionIndex(existingBlocks, afterBlockId);
        }

        List<String> types = detectBlockTypes(body);
        String status = "normal";  // Default status

        // Get page to get dir_path and name
        Page page = pageRepository.findById(pageId).orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        Block block = new Block(id, pageId, page.dirPath(), page.name(), page.type(), now, now, idx, level, body, html, false, status, types);
        block = blockRepository.save(block);

        // Update page modified date
        touchPage(pageId);

        // Index for search
        searchService.indexBlock(block, page.name());

        // Queue for export
        exportService.queueExport(pageId);

        return block;
    }

    /**
     * Updates an existing block. Only non-null parameters are applied.
     *
     * @param blockId        the block to update
     * @param body           new markdown body, or {@code null} to keep current
     * @param level          new indentation level, or {@code null} to keep current
     * @param collapsed      new collapsed state, or {@code null} to keep current
     * @param status         new status, or {@code null} to keep current
     * @return the updated block
     * @throws IllegalArgumentException if the block is not found
     */
    public Block updateBlock(UUID blockId, String body, Integer level, Boolean collapsed, String status) {
        Block existing = blockRepository.findById(blockId)
                .orElseThrow(() -> new IllegalArgumentException("Block not found: " + blockId));

        String newBody = body != null ? body : existing.body();
        String newHtml = body != null ? parseBody(body) : existing.html();
        int newLevel = level != null ? level : existing.level();
        boolean newCollapsed = collapsed != null ? collapsed : existing.collapsed();
        String newStatus = status != null ? status : existing.status();
        List<String> types = body != null ? detectBlockTypes(body) : existing.types();

        Block updated = new Block(
                existing.id(),
                existing.pageId(),
                existing.dirPath(),
                existing.name(),
                existing.type(),
                existing.createdDate(),
                Instant.now(),
                existing.idx(),
                newLevel,
                newBody,
                newHtml,
                newCollapsed,
                newStatus,
                types
        );

        updated = blockRepository.save(updated);
        touchPage(existing.pageId());

        Page page = pageRepository.findById(existing.pageId()).orElse(null);
        if (page != null) {
            searchService.indexBlock(updated, page.name());
        }

        // Queue for export
        exportService.queueExport(existing.pageId());

        return updated;
    }

    /**
     * Moves blocks to a target page at the specified position.
     *
     * @param blockIds     the IDs of blocks to move (order is preserved)
     * @param targetPageId the destination page
     * @param afterBlockId the block ID to insert after, or empty/null to prepend
     * @param levelDelta   level adjustment to apply, or {@code null} for no change
     * @return the moved blocks with updated indices
     * @throws IllegalArgumentException if a block or the target page is not found
     */
    public List<Block> moveBlocks(List<UUID> blockIds, UUID targetPageId, UUID afterBlockId, Integer levelDelta) {
        List<Block> movedBlocks = new ArrayList<>();
        List<Block> targetBlocks = blockRepository.findByPageId(targetPageId);

        // Find insertion point
        String afterIdx = null;
        String nextIdx = null;
        if (afterBlockId != null) {
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

        // Get target page to get dir_path and name
        Page targetPage = pageRepository.findById(targetPageId)
                .orElseThrow(() -> new IllegalArgumentException("Target page not found: " + targetPageId));

        for (UUID blockId : blockIds) {
            Block block = blockRepository.findById(blockId)
                    .orElseThrow(() -> new IllegalArgumentException("Block not found: " + blockId));

            String newIdx = FractionalIndexer.between(afterIdx, nextIdx);
            int newLevel = Math.max(1, block.level() + delta);

            Block moved = new Block(
                    block.id(),
                    targetPageId,
                    targetPage.dirPath(),
                    targetPage.name(),
                    targetPage.type(),
                    block.createdDate(),
                    now,
                    newIdx,
                    newLevel,
                    block.body(),
                    block.html(),
                    block.collapsed(),
                    block.status(),
                    block.types()
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

    /**
     * Deletes a block and removes it from the search index.
     *
     * @param blockId the block to delete
     */
    public void deleteBlock(UUID blockId) {
        Optional<Block> block = blockRepository.findById(blockId);
        blockRepository.delete(blockId);
        searchService.removeBlock(blockId.toString());
        block.ifPresent(b -> {
            touchPage(b.pageId());
            // Queue for export
            exportService.queueExport(b.pageId());
        });
    }

    /** Returns all blocks matching the given statuses. */
    public List<Block> getBlocksByStatuses(List<String> statuses) {
        return blockRepository.findByStatuses(statuses);
    }

    /**
     * Returns the page ID that contains the given block.
     *
     * @param blockId the block to look up
     * @return the page UUID as a string, or {@code null} if the block is not found
     */
    public String getBlockPageId(UUID blockId) {
        return blockRepository.findById(blockId)
                .map(b -> b.pageId().toString())
                .orElse(null);
    }

    /**
     * Updates the status of a block.
     *
     * @param blockId the block to update
     * @param status  the new status ({@code normal}, {@code todo}, {@code running}, or {@code done})
     * @return the updated block
     * @throws IllegalArgumentException if the status is invalid or the block is not found
     */
    public Block updateBlockStatus(UUID blockId, String status) {
        if (!List.of("normal", "todo", "running", "done").contains(status)) {
            throw new IllegalArgumentException("Invalid status: " + status);
        }
        return updateBlock(blockId, null, null, null, status);
    }

    /** Result of exporting blocks as tree markdown. */
    public record TreeMarkdownExport(String treeMarkdown, String html, String flatMarkdown) {}

    /**
     * Exports blocks as hierarchical tree markdown, flat markdown, and HTML.
     *
     * @param blockIds the block IDs to export
     * @return the exported representations
     */
    public TreeMarkdownExport exportTreeMarkdown(List<UUID> blockIds) {
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

    /**
     * Converts tree markdown (indented with tabs) to flat markdown.
     *
     * @param treeMarkdown the hierarchical tree markdown input
     * @return the converted flat markdown
     */
    public String importTreeMarkdown(String treeMarkdown) {
        return treeMarkdownConverter.treeMarkdownToFlatMarkdown(treeMarkdown);
    }

    /**
     * Re-assign fresh fractional indices to all blocks on a page.
     * WARNING: This operation reassigns all idx values on a page. In a future CRDT-based
     * sync system, this will conflict with remote changes and may cause ordering issues.
     */
    private void reindexPage(UUID pageId, List<Block> blocks) {
        LOG.warnv("REINDEX: Reassigning fractional indices for page {0} ({1} blocks). " +
                "This may cause conflicts with CRDT-based sync.", pageId, blocks.size());
        List<String> newIndices = FractionalIndexer.generateIndices(blocks.size());
        for (int i = 0; i < blocks.size(); i++) {
            blockRepository.updateIndex(blocks.get(i).id(), newIndices.get(i));
        }
    }

    /**
     * Calculate the fractional index for inserting a block after the specified block.
     * If afterBlockId is null/empty, returns an index for the beginning of the list.
     */
    private String calculateInsertionIndex(List<Block> existingBlocks, UUID afterBlockId) {
        if (afterBlockId == null) {
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
                    page.id(), page.parentId(), page.dirPath(), page.name(), page.type(),
                    page.createdDate(), Instant.now(),
                    page.tags()
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
                    .filter(p -> p.name().equalsIgnoreCase(pageName))
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
