package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.domain.ItemIdGenerator;
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
public class ItemService {

    private static final Logger LOG = Logger.getLogger(ItemService.class);

    @Inject
    ItemRepository itemRepository;

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
     * Creates a new item on the specified page.
     *
     * @param pageId       the page to add the item to
     * @param afterItemId the item ID after which to insert, or empty/null to prepend
     * @param level        the indentation level (1-based, 1=top level)
     * @param body         the markdown body text
     * @return the newly created item
     * @throws IllegalArgumentException if the page is not found
     */
    public Item createItem(UUID pageId, UUID afterItemId, int level, String body, UUID clientId) {
        UUID id = clientId != null ? clientId : ItemIdGenerator.generate();
        Instant now = Instant.now();
        String html = parseBody(body);

        List<Item> existingItems = itemRepository.findByPageId(pageId);
        String idx;
        try {
            idx = calculateInsertionIndex(existingItems, afterItemId);
        } catch (IllegalStateException e) {
            // Fractional index exhausted — reindex and retry
            reindexPage(pageId, existingItems);
            existingItems = itemRepository.findByPageId(pageId);
            idx = calculateInsertionIndex(existingItems, afterItemId);
        }

        List<String> types = detectItemTypes(body);
        String status = "normal";  // Default status

        // Get page to get dir_path and name
        Page page = pageRepository.findById(pageId).orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        Item item = new Item(id, pageId, null, page.dirPath(), page.name(), page.type(), now, now, idx, level, body, html, false, status, types);
        item = itemRepository.save(item);

        // Recalculate parent_id for all items on this page
        itemRepository.recalculateParentIds(pageId);
        item = itemRepository.findById(id).orElse(item);

        // Update page modified date
        touchPage(pageId);

        // Index for search
        searchService.indexItem(item, page.dirPath() + page.name());

        // Queue for export
        exportService.queueExport(pageId);

        return item;
    }

    /**
     * Updates an existing item. Only non-null parameters are applied.
     *
     * @param itemId        the item to update
     * @param body           new markdown body, or {@code null} to keep current
     * @param level          new indentation level, or {@code null} to keep current
     * @param collapsed      new collapsed state, or {@code null} to keep current
     * @param status         new status, or {@code null} to keep current
     * @return the updated item
     * @throws IllegalArgumentException if the item is not found
     */
    public Item updateItem(UUID itemId, String body, Integer level, Boolean collapsed, String status) {
        Item existing = itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));

        String newBody = body != null ? body : existing.body();
        String newHtml = body != null ? parseBody(body) : existing.html();
        int newLevel = level != null ? level : existing.level();
        boolean newCollapsed = collapsed != null ? collapsed : existing.collapsed();
        String newStatus = status != null ? status : existing.status();
        List<String> types = body != null ? detectItemTypes(body) : existing.types();

        Item updated = new Item(
                existing.id(),
                existing.pageId(),
                existing.parentItemId(),
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

        updated = itemRepository.save(updated);

        // Recalculate parent_id when level changes
        if (level != null) {
            itemRepository.recalculateParentIds(existing.pageId());
            updated = itemRepository.findById(existing.id()).orElse(updated);
        }

        touchPage(existing.pageId());

        Page page = pageRepository.findById(existing.pageId()).orElse(null);
        if (page != null) {
            searchService.indexItem(updated, page.dirPath() + page.name());
        }

        // Queue for export
        exportService.queueExport(existing.pageId());

        return updated;
    }

    /**
     * Moves items to a target page at the specified position.
     *
     * @param itemIds     the IDs of items to move (order is preserved)
     * @param targetPageId the destination page
     * @param afterItemId the item ID to insert after, or empty/null to prepend
     * @param levelDelta   level adjustment to apply, or {@code null} for no change
     * @return the moved items with updated indices
     * @throws IllegalArgumentException if an item or the target page is not found
     */
    public List<Item> moveItems(List<UUID> itemIds, UUID targetPageId, UUID afterItemId, Integer levelDelta) {
        List<Item> movedItems = new ArrayList<>();
        List<Item> targetItems = itemRepository.findByPageId(targetPageId);

        // Find insertion point
        String afterIdx = null;
        String nextIdx = null;
        if (afterItemId != null) {
            for (int i = 0; i < targetItems.size(); i++) {
                if (targetItems.get(i).id().equals(afterItemId)) {
                    afterIdx = targetItems.get(i).idx();
                    if (i + 1 < targetItems.size()) {
                        nextIdx = targetItems.get(i + 1).idx();
                    }
                    break;
                }
            }
        } else if (!targetItems.isEmpty()) {
            nextIdx = targetItems.getFirst().idx();
        }

        int delta = levelDelta != null ? levelDelta : 0;
        Instant now = Instant.now();

        // Get target page to get dir_path and name
        Page targetPage = pageRepository.findById(targetPageId)
                .orElseThrow(() -> new IllegalArgumentException("Target page not found: " + targetPageId));

        for (UUID itemId : itemIds) {
            Item item = itemRepository.findById(itemId)
                    .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));

            String newIdx = FractionalIndexer.between(afterIdx, nextIdx);
            int newLevel = Math.max(1, item.level() + delta);

            Item moved = new Item(
                    item.id(),
                    targetPageId,
                    null,
                    targetPage.dirPath(),
                    targetPage.name(),
                    targetPage.type(),
                    item.createdDate(),
                    now,
                    newIdx,
                    newLevel,
                    item.body(),
                    item.html(),
                    item.collapsed(),
                    item.status(),
                    item.types()
            );

            moved = itemRepository.save(moved);
            movedItems.add(moved);

            afterIdx = newIdx;
            nextIdx = null;
        }

        // Recalculate parent_id for all items on the target page
        itemRepository.recalculateParentIds(targetPageId);

        // Re-read moved items to get updated parent_id
        movedItems = movedItems.stream()
                .map(b -> itemRepository.findById(b.id()).orElse(b))
                .toList();

        touchPage(targetPageId);

        // Queue for export
        exportService.queueExport(targetPageId);

        return movedItems;
    }

    /**
     * Deletes an item and removes it from the search index.
     *
     * @param itemId the item to delete
     */
    public void deleteItem(UUID itemId) {
        Optional<Item> item = itemRepository.findById(itemId);
        itemRepository.delete(itemId);
        searchService.removeItem(itemId.toString());
        item.ifPresent(b -> {
            itemRepository.recalculateParentIds(b.pageId());
            touchPage(b.pageId());
            // Queue for export
            exportService.queueExport(b.pageId());
        });
    }

    /** Returns all items matching the given statuses. */
    public List<Item> getItemsByStatuses(List<String> statuses) {
        return itemRepository.findByStatuses(statuses);
    }

    /**
     * Returns the page ID that contains the given item.
     *
     * @param itemId the item to look up
     * @return the page UUID as a string, or {@code null} if the item is not found
     */
    public String getItemPageId(UUID itemId) {
        return itemRepository.findById(itemId)
                .map(b -> b.pageId().toString())
                .orElse(null);
    }

    /**
     * Updates the status of an item.
     *
     * @param itemId the item to update
     * @param status  the new status ({@code normal}, {@code todo}, {@code running}, or {@code done})
     * @return the updated item
     * @throws IllegalArgumentException if the status is invalid or the item is not found
     */
    public Item updateItemStatus(UUID itemId, String status) {
        if (!List.of("normal", "todo", "running", "done").contains(status)) {
            throw new IllegalArgumentException("Invalid status: " + status);
        }
        return updateItem(itemId, null, null, null, status);
    }

    /** Result of exporting items as tree markdown. */
    public record TreeMarkdownExport(String treeMarkdown, String html, String flatMarkdown) {}

    /**
     * Exports items as hierarchical tree markdown, flat markdown, and HTML.
     *
     * @param itemIds the item IDs to export
     * @return the exported representations
     */
    public TreeMarkdownExport exportTreeMarkdown(List<UUID> itemIds) {
        List<Item> items = itemIds.stream()
            .map(id -> itemRepository.findById(id))
            .filter(Optional::isPresent)
            .map(Optional::get)
            .toList();
        String treeMd = treeMarkdownConverter.itemsToTreeMarkdown(items);
        String html = markdownParser.parseToHtml(treeMd);
        String flatMd = treeMarkdownConverter.itemsToFlatMarkdown(items);
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
     * Re-assign fresh fractional indices to all items on a page.
     * WARNING: This operation reassigns all idx values on a page. In a future CRDT-based
     * sync system, this will conflict with remote changes and may cause ordering issues.
     */
    private void reindexPage(UUID pageId, List<Item> items) {
        LOG.warnv("REINDEX: Reassigning fractional indices for page {0} ({1} items). " +
                "This may cause conflicts with CRDT-based sync.", pageId, items.size());
        List<String> newIndices = FractionalIndexer.generateIndices(items.size());
        for (int i = 0; i < items.size(); i++) {
            itemRepository.updateIndex(items.get(i).id(), newIndices.get(i));
        }
    }

    /**
     * Calculate the fractional index for inserting an item after the specified item.
     * If afterItemId is null/empty, returns an index for the beginning of the list.
     */
    private String calculateInsertionIndex(List<Item> existingItems, UUID afterItemId) {
        if (afterItemId == null) {
            if (existingItems.isEmpty()) {
                return FractionalIndexer.initial();
            }
            return FractionalIndexer.between(null, existingItems.getFirst().idx());
        }

        String afterIdx = null;
        String nextIdx = null;
        for (int i = 0; i < existingItems.size(); i++) {
            if (existingItems.get(i).id().equals(afterItemId)) {
                afterIdx = existingItems.get(i).idx();
                if (i + 1 < existingItems.size()) {
                    nextIdx = existingItems.get(i + 1).idx();
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

    private List<String> detectItemTypes(String body) {
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
