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

import com.petajournal.server.domain.HashtagExtractor;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;

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

    @Inject
    com.petajournal.server.infrastructure.persistence.PropertyRepository propertyRepository;

    /**
     * Creates a new item on the specified page.
     *
     * @param pageId       the page to add the item to
     * @param afterItemId the item ID after which to insert, or empty/null to append
     * @param level        the indentation level (1-based, 1=top level)
     * @param body         the markdown body text
     * @return the newly created item
     * @throws IllegalArgumentException if the page is not found
     */
    public Item createItem(UUID pageId, UUID afterItemId, int level, String body, UUID clientId) {
        UUID id = clientId != null ? clientId : ItemIdGenerator.generate();
        Instant now = Instant.now();
        String ast = parseBody(body);

        List<Item> existingItems = itemRepository.findByPageId(pageId);

        // Validate level constraint
        Item aboveItem = findItemAbove(existingItems, afterItemId);
        if (aboveItem == null) {
            if (level != 1) {
                throw new IllegalArgumentException("Level must be 1 when there is no item above the insertion position");
            }
        } else {
            if (level > aboveItem.level() + 1) {
                throw new IllegalArgumentException("Level must be at most " + (aboveItem.level() + 1)
                        + " (one more than the item above), but was " + level);
            }
        }

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

        // Get page to get dir_path and name
        Page page = pageRepository.findById(pageId).orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        Item item = new Item(id, pageId, null, page.dirPath(), page.name(), page.type(), now, now, idx, level, body, ast, false, types, null);
        item = itemRepository.save(item);

        // Recalculate parent_id for all items on this page
        itemRepository.recalculateParentIds(pageId);
        item = itemRepository.findById(id).orElse(item);

        // Update page modified date
        touchPage(pageId);

        // Sync hashtags as tag properties
        List<String> hashtags = HashtagExtractor.extractHashtags(ast);
        if (!hashtags.isEmpty()) {
            propertyRepository.syncTags("item", id.toString(), hashtags);
        }

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
     * @return the updated item
     * @throws IllegalArgumentException if the item is not found
     */
    public Item updateItem(UUID itemId, String body, Integer level, Boolean collapsed) {
        Item existing = itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));

        String newBody = body != null ? body : existing.body();
        String newAst = body != null ? parseBody(body) : existing.ast();
        int newLevel = level != null ? level : existing.level();
        boolean newCollapsed = collapsed != null ? collapsed : existing.collapsed();
        List<String> types = body != null ? detectItemTypes(body) : existing.types();
        if (body != null && existing.types().contains("task") && !types.contains("task")) {
            types = new ArrayList<>(types);
            types.add("task");
        }

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
                newAst,
                newCollapsed,
                types,
                existing.transcludeItemId()
        );

        updated = itemRepository.save(updated);

        // Sync hashtags when body changes
        if (body != null) {
            List<String> hashtags = HashtagExtractor.extractHashtags(newAst);
            propertyRepository.syncTags("item", existing.id().toString(), hashtags);
        }

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
     * @param afterItemId the item ID to insert after, or empty/null to append
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
            afterIdx = targetItems.getLast().idx();
        }

        int delta = levelDelta != null ? levelDelta : 0;
        Instant now = Instant.now();

        // Validate level constraints for all items before making changes
        Item aboveItem = findItemAbove(targetItems, afterItemId);
        for (UUID itemId : itemIds) {
            Item item = itemRepository.findById(itemId)
                    .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));
            int newLevel = Math.max(1, item.level() + delta);
            if (aboveItem == null) {
                if (newLevel != 1) {
                    throw new IllegalArgumentException("Level must be 1 when there is no item above the insertion position");
                }
            } else {
                if (newLevel > aboveItem.level() + 1) {
                    throw new IllegalArgumentException("Level must be at most " + (aboveItem.level() + 1)
                            + " (one more than the item above), but was " + newLevel);
                }
            }
            // For subsequent items, use the moved level as the "above" level
            int aboveLevel = newLevel;
            aboveItem = new Item(item.id(), item.pageId(), item.parentItemId(),
                    item.dirPath(), item.name(), item.type(),
                    item.createdDate(), item.modifiedDate(), item.idx(),
                    aboveLevel, item.body(), item.ast(),
                    item.collapsed(), item.types(), item.transcludeItemId());
        }

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
                    item.ast(),
                    item.collapsed(),
                    item.types(),
                    item.transcludeItemId()
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
        propertyRepository.syncTags("item", itemId.toString(), List.of());
        item.ifPresent(b -> {
            itemRepository.recalculateParentIds(b.pageId());
            touchPage(b.pageId());
            // Queue for export
            exportService.queueExport(b.pageId());
        });
    }

    /**
     * Sets a transclusion on an item, making it mirror a source item.
     *
     * @param itemId       the item to set as a transclusion
     * @param sourceItemId the source item to transclude
     * @return the updated item
     * @throws IllegalArgumentException if the item or source is not found, or source is itself a transclusion
     */
    public Item setTransclusion(UUID itemId, UUID sourceItemId) {
        Item item = itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));
        Item source = itemRepository.findById(sourceItemId)
                .orElseThrow(() -> new IllegalArgumentException("Source item not found: " + sourceItemId));

        // Prevent transclusion of a transclusion
        if (source.transcludeItemId() != null) {
            throw new IllegalArgumentException("Cannot transclude a transclusion item. Please select the original source item.");
        }

        // Copy source body as fallback label and set transclude_item_id
        Item updated = new Item(
                item.id(), item.pageId(), item.parentItemId(),
                item.dirPath(), item.name(), item.type(),
                item.createdDate(), Instant.now(), item.idx(),
                item.level(), source.body(), parseBody(source.body()),
                item.collapsed(), item.types(), sourceItemId);
        updated = itemRepository.save(updated);
        touchPage(item.pageId());
        exportService.queueExport(item.pageId());
        return updated;
    }

    /**
     * Removes a transclusion from an item, clearing its body.
     *
     * @param itemId the transclusion item to revert
     * @return the updated item
     * @throws IllegalArgumentException if the item is not found
     */
    public Item removeTransclusion(UUID itemId) {
        Item item = itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));

        Item updated = new Item(
                item.id(), item.pageId(), item.parentItemId(),
                item.dirPath(), item.name(), item.type(),
                item.createdDate(), Instant.now(), item.idx(),
                item.level(), "", parseBody(""),
                item.collapsed(), item.types(), null);
        updated = itemRepository.save(updated);
        touchPage(item.pageId());
        exportService.queueExport(item.pageId());
        return updated;
    }

    /** Returns an item by its ID. */
    public Item getItemById(UUID itemId) {
        return itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));
    }

    /** Returns all items matching the given types. */
    public List<Item> getItemsByTypes(List<String> types) {
        return itemRepository.findByTypes(types);
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
     * Cycles the task state of an item:
     * 1. Not a task → add "task" to types, remove begin/end
     * 2. Task, no begin, no end → set begin = now
     * 3. Task, has begin, no end → set end = now
     * 4. Task, has begin AND end → remove task type, begin/end (back to normal)
     */
    public Item cycleTask(UUID itemId) {
        Item existing = itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));

        String id = itemId.toString();
        Map<String, String> scalars = propertyRepository.findScalarByTargets("item", List.of(id))
                .getOrDefault(id, Map.of());
        boolean isTask = existing.types().contains("task");
        boolean hasBegin = scalars.containsKey("begin");
        boolean hasEnd = scalars.containsKey("end");

        if (!isTask) {
            // State 1: Not a task → become a task
            List<String> newTypes = new ArrayList<>(existing.types());
            newTypes.add("task");
            Item updated = new Item(
                    existing.id(), existing.pageId(), existing.parentItemId(),
                    existing.dirPath(), existing.name(), existing.type(),
                    existing.createdDate(), Instant.now(), existing.idx(),
                    existing.level(), existing.body(), existing.ast(),
                    existing.collapsed(), newTypes, existing.transcludeItemId());
            itemRepository.save(updated);
            propertyRepository.deleteProperty("item", id, "begin");
            propertyRepository.deleteProperty("item", id, "end");
            touchPage(existing.pageId());
            exportService.queueExport(existing.pageId());
            return updated;
        } else if (!hasBegin && !hasEnd) {
            // State 2: Task, no begin → set begin
            String now = Instant.now().toString();
            propertyRepository.upsertScalar("item", id, "begin", "datetime", now);
            touchPage(existing.pageId());
            exportService.queueExport(existing.pageId());
            return existing;
        } else if (hasBegin && !hasEnd) {
            // State 3: Task, has begin, no end → set end
            String now = Instant.now().toString();
            propertyRepository.upsertScalar("item", id, "end", "datetime", now);
            touchPage(existing.pageId());
            exportService.queueExport(existing.pageId());
            return existing;
        } else {
            // State 4: Task, has begin AND end → back to normal (remove task type, begin/end)
            List<String> newTypes = new ArrayList<>(existing.types());
            newTypes.remove("task");
            Item updated = new Item(
                    existing.id(), existing.pageId(), existing.parentItemId(),
                    existing.dirPath(), existing.name(), existing.type(),
                    existing.createdDate(), Instant.now(), existing.idx(),
                    existing.level(), existing.body(), existing.ast(),
                    existing.collapsed(), newTypes, existing.transcludeItemId());
            itemRepository.save(updated);
            propertyRepository.deleteProperty("item", id, "begin");
            propertyRepository.deleteProperty("item", id, "end");
            touchPage(existing.pageId());
            exportService.queueExport(existing.pageId());
            return updated;
        }
    }

    /** Result of exporting items as tree markdown. */
    public record TreeMarkdownExport(String treeMarkdown, String ast, String flatMarkdown) {}

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
        String ast = markdownParser.parseToAst(treeMd);
        String flatMd = treeMarkdownConverter.itemsToFlatMarkdown(items);
        return new TreeMarkdownExport(treeMd, ast, flatMd);
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
     * If afterItemId is null/empty, returns an index for the end of the list.
     */
    private String calculateInsertionIndex(List<Item> existingItems, UUID afterItemId) {
        if (afterItemId == null) {
            if (existingItems.isEmpty()) {
                return FractionalIndexer.initial();
            }
            return FractionalIndexer.between(existingItems.getLast().idx(), null);
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

    /**
     * Finds the item directly above the insertion position.
     * If afterItemId is specified, that item is the one above.
     * If afterItemId is null (append), the last item in the list is above.
     * Returns null if the list is empty.
     */
    private Item findItemAbove(List<Item> existingItems, UUID afterItemId) {
        if (existingItems.isEmpty()) {
            return null;
        }
        if (afterItemId == null) {
            return existingItems.getLast();
        }
        for (Item item : existingItems) {
            if (item.id().equals(afterItemId)) {
                return item;
            }
        }
        return null;
    }

    private void touchPage(UUID pageId) {
        pageRepository.findById(pageId).ifPresent(page -> {
            Page updated = new Page(
                    page.id(), page.parentId(), page.dirPath(), page.name(), page.type(),
                    page.createdDate(), Instant.now()
            );
            pageRepository.save(updated);
        });
    }

    private String parseBody(String body) {
        return markdownParser.parseToAst(body);
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
