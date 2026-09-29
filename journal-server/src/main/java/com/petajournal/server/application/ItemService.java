package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.ItemPatch;
import com.petajournal.server.domain.model.ItemPlacement;
import com.petajournal.server.domain.model.ItemPosition;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.FractionalIndexer;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import com.petajournal.server.domain.HashtagExtractor;
import java.time.Instant;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;

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

    @Inject
    IdGenerator idGenerator;

    private final Object moveLock = new Object();

    /**
     * Creates a new item on the specified page.
     *
     * @param pageId       the page to add the item to
     * @param position     the position to insert at (FIRST, LAST, or after a specific item)
     * @param level        the indentation level (1-based, 1=top level)
     * @param body         the markdown body text
     * @return the newly created item
     * @throws IllegalArgumentException if the page is not found
     */
    public Item createItem(HMTID pageId, ItemPosition position, int level, String body, HMTID clientId) {
        HMTID id = clientId != null ? clientId : idGenerator.generateItemId();
        Instant now = Instant.now();
        boolean isTask = body != null && (body.startsWith("- [ ] ") || body.startsWith("- [x] "));
        if (isTask) body = body.substring(6);
        String ast = parseBody(body);

        List<Item> existingItems = itemRepository.findByPageId(pageId);

        // Validate level constraint
        Item aboveItem = findItemAbove(existingItems, position);
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
            idx = calculateInsertionIndex(existingItems, position);
        } catch (IllegalStateException e) {
            // Fractional index exhausted — reindex and retry
            reindexPage(pageId, existingItems);
            existingItems = itemRepository.findByPageId(pageId);
            idx = calculateInsertionIndex(existingItems, position);
        }

        List<String> types = new ArrayList<>(detectItemTypes(body));
        if (isTask) types.add("task");

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
     * Updates an existing item. Only non-null parameters are written.
     * The other fields keep their current database value, so a body save and
     * a level change that run at the same time do not overwrite each other.
     *
     * @param itemId        the item to update
     * @param body           new markdown body, or {@code null} to keep current
     * @param level          new indentation level, or {@code null} to keep current
     * @param collapsed      new collapsed state, or {@code null} to keep current
     * @return the updated item
     * @throws IllegalArgumentException if the item is not found, or if level is less than 1
     */
    public Item updateItem(HMTID itemId, String body, Integer level, Boolean collapsed) {
        Item existing = itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));

        boolean isTask = body != null && (body.startsWith("- [ ] ") || body.startsWith("- [x] "));
        String newBody = isTask ? body.substring(6) : body;
        String newAst = body != null ? parseBody(newBody) : null;
        List<String> bodyTypes = body != null ? detectItemTypes(newBody) : null;

        // A task stays a task when its body changes; only the prefix can add the type here
        itemRepository.patch(itemId, new ItemPatch(
                newBody, newAst, bodyTypes, isTask ? Boolean.TRUE : null, level, collapsed, Instant.now()));

        // Sync hashtags when body changes
        if (body != null) {
            List<String> hashtags = HashtagExtractor.extractHashtags(newAst);
            propertyRepository.syncTags("item", existing.id().toString(), hashtags);
        }

        // Recalculate parent_id when level changes
        if (level != null) {
            itemRepository.recalculateParentIds(existing.pageId());
        }
        Item updated = itemRepository.findById(itemId)
                .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));

        touchPage(existing.pageId());

        // The search index holds the body only; level and collapsed do not change it
        if (body != null) {
            pageRepository.findById(existing.pageId())
                    .ifPresent(page -> searchService.indexItem(updated, page.dirPath() + page.name()));
        }

        // Queue for export
        exportService.queueExport(existing.pageId());

        return updated;
    }

    /**
     * Adds delta to the level of every given item in one step.
     * The items keep their order; only their depth changes.
     *
     * @param itemIds the items to shift, usually an item and its descendants
     * @param delta   the amount to add to each level; may be negative. A level never goes below 1
     * @return the items after the change, in the order of itemIds
     * @throws NullPointerException     if itemIds is null
     * @throws IllegalArgumentException if itemIds is empty or an item is not found
     */
    public List<Item> shiftLevels(List<HMTID> itemIds, int delta) {
        Objects.requireNonNull(itemIds, "itemIds must not be null");
        if (itemIds.isEmpty()) {
            throw new IllegalArgumentException("itemIds must not be empty");
        }
        Set<HMTID> pageIds = new LinkedHashSet<>();
        for (HMTID itemId : itemIds) {
            Item item = itemRepository.findById(itemId)
                    .orElseThrow(() -> new IllegalArgumentException("Item not found: " + itemId));
            pageIds.add(item.pageId());
        }

        itemRepository.shiftLevels(itemIds, delta, Instant.now());

        for (HMTID pageId : pageIds) {
            itemRepository.recalculateParentIds(pageId);
            touchPage(pageId);
            exportService.queueExport(pageId);
        }

        return itemIds.stream()
                .map(id -> itemRepository.findById(id)
                        .orElseThrow(() -> new IllegalArgumentException("Item not found: " + id)))
                .toList();
    }

    /**
     * Moves items to a target page at the specified position.
     * The move is all or nothing: if it fails, no item has moved.
     * Only the place and the level of the items change; their body is not written.
     *
     * @param itemIds     the IDs of items to move (order is preserved)
     * @param targetPageId the destination page
     * @param position     the position to insert at (FIRST, LAST, or after a specific item)
     * @param levelDelta   level adjustment to apply, or {@code null} for no change
     * @return the moved items with updated indices
     * @throws NullPointerException     if itemIds, targetPageId or position is null
     * @throws IllegalArgumentException if an item or the target page is not found, if the position
     *                                  names an item that is not on the target page or is being moved,
     *                                  or if a level does not fit the item above
     */
    public List<Item> moveItems(List<HMTID> itemIds, HMTID targetPageId, ItemPosition position, Integer levelDelta) {
        Objects.requireNonNull(itemIds, "itemIds must not be null");
        Objects.requireNonNull(targetPageId, "targetPageId must not be null");
        Objects.requireNonNull(position, "position must not be null");
        int delta = levelDelta != null ? levelDelta : 0;

        Set<HMTID> touchedPageIds = new LinkedHashSet<>();
        touchedPageIds.add(targetPageId);

        // Two moves must not plan their indices from the same snapshot of the page
        synchronized (moveLock) {
            pageRepository.findById(targetPageId)
                    .orElseThrow(() -> new IllegalArgumentException("Target page not found: " + targetPageId));
            List<Item> moving = itemIds.stream()
                    .map(id -> itemRepository.findById(id)
                            .orElseThrow(() -> new IllegalArgumentException("Item not found: " + id)))
                    .toList();
            moving.forEach(item -> touchedPageIds.add(item.pageId()));

            List<Item> staying = itemsStayingOn(targetPageId, itemIds);
            validateLevels(moving, delta, findAnchor(staying, position));

            List<String> newIdxs;
            try {
                newIdxs = planIndices(staying, position, moving.size());
            } catch (IllegalStateException e) {
                // No room left in the gap: reindex the page and plan again
                reindexPage(targetPageId, itemRepository.findByPageId(targetPageId));
                staying = itemsStayingOn(targetPageId, itemIds);
                newIdxs = planIndices(staying, position, moving.size());
            }

            List<ItemPlacement> placements = new ArrayList<>(moving.size());
            for (int i = 0; i < moving.size(); i++) {
                placements.add(new ItemPlacement(moving.get(i).id(), newIdxs.get(i)));
            }
            itemRepository.moveItems(targetPageId, placements, delta, Instant.now());
        }

        // The pages the items left need new parents and a new export too
        for (HMTID pageId : touchedPageIds) {
            itemRepository.recalculateParentIds(pageId);
            touchPage(pageId);
            exportService.queueExport(pageId);
        }

        return itemIds.stream()
                .map(id -> itemRepository.findById(id)
                        .orElseThrow(() -> new IllegalArgumentException("Item not found: " + id)))
                .toList();
    }

    private List<Item> itemsStayingOn(HMTID pageId, List<HMTID> movingIds) {
        Set<HMTID> moving = Set.copyOf(movingIds);
        return itemRepository.findByPageId(pageId).stream()
                .filter(item -> !moving.contains(item.id()))
                .toList();
    }

    /** Returns the item the group is placed under, or null when the group goes to the top of the page. */
    private Item findAnchor(List<Item> staying, ItemPosition position) {
        return switch (position) {
            case ItemPosition.First() -> null;
            case ItemPosition.Last() -> staying.isEmpty() ? null : staying.getLast();
            case ItemPosition.After(HMTID anchorId) -> staying.stream()
                    .filter(item -> item.id().equals(anchorId))
                    .findFirst()
                    .orElseThrow(() -> new IllegalArgumentException(
                            "Position item is not on the target page or is being moved: " + anchorId));
        };
    }

    private void validateLevels(List<Item> moving, int delta, Item anchor) {
        Integer aboveLevel = anchor != null ? anchor.level() : null;
        for (Item item : moving) {
            int newLevel = Math.max(1, item.level() + delta);
            if (aboveLevel == null) {
                if (newLevel != 1) {
                    throw new IllegalArgumentException("Level must be 1 when there is no item above the insertion position");
                }
            } else if (newLevel > aboveLevel + 1) {
                throw new IllegalArgumentException("Level must be at most " + (aboveLevel + 1)
                        + " (one more than the item above), but was " + newLevel);
            }
            // For subsequent items, the item just placed is the one above
            aboveLevel = newLevel;
        }
    }

    /** Plans indices for the whole group inside the gap at the position, so none reaches the next item. */
    private List<String> planIndices(List<Item> staying, ItemPosition position, int count) {
        Item anchor = findAnchor(staying, position);
        int nextPos = anchor == null ? 0 : staying.indexOf(anchor) + 1;
        String afterIdx = anchor != null ? anchor.idx() : null;
        String nextIdx = nextPos < staying.size() ? staying.get(nextPos).idx() : null;
        if (afterIdx == null && nextIdx == null) {
            return FractionalIndexer.generateIndices(count);
        }
        return FractionalIndexer.between(afterIdx, nextIdx, count);
    }

    /**
     * Deletes an item and removes it from the search index.
     *
     * @param itemId the item to delete
     */
    public void deleteItem(HMTID itemId) {
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
    public Item setTransclusion(HMTID itemId, HMTID sourceItemId) {
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
    public Item removeTransclusion(HMTID itemId) {
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
    public Item getItemById(HMTID itemId) {
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
     * @return the page HMTID as a string, or {@code null} if the item is not found
     */
    public String getItemPageId(HMTID itemId) {
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
    public Item cycleTask(HMTID itemId) {
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
            Item updated = patchTask(existing, true);
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
            Item updated = patchTask(existing, false);
            propertyRepository.deleteProperty("item", id, "begin");
            propertyRepository.deleteProperty("item", id, "end");
            touchPage(existing.pageId());
            exportService.queueExport(existing.pageId());
            return updated;
        }
    }

    /** Adds or removes the task type without writing any other field of the item. */
    private Item patchTask(Item existing, boolean task) {
        itemRepository.patch(existing.id(), new ItemPatch(null, null, null, task, null, null, Instant.now()));
        return itemRepository.findById(existing.id()).orElse(existing);
    }

    /** Result of exporting items as tree markdown. */
    public record TreeMarkdownExport(String treeMarkdown, String ast, String flatMarkdown) {}

    /**
     * Exports all items on a page as tree markdown, flat markdown, and AST.
     *
     * @param pageId the page ID
     * @return the exported representations
     */
    public TreeMarkdownExport exportPageMarkdown(HMTID pageId) {
        List<Item> items = itemRepository.findByPageId(pageId);
        List<String> itemIds = items.stream().map(i -> i.id().toString()).toList();
        var scalarProps = propertyRepository.findScalarByTargets("item", itemIds);
        var listProps = propertyRepository.findListByTargets("item", itemIds);
        String treeMd = treeMarkdownConverter.itemsToTreeMarkdown(items);
        String ast = markdownParser.parseToAst(treeMd);
        String flatMd = treeMarkdownConverter.itemsToFlatMarkdown(items, scalarProps, listProps);
        return new TreeMarkdownExport(treeMd, ast, flatMd);
    }

    /**
     * Exports an item and all its descendants (subtree) as tree markdown, flat markdown, and AST.
     *
     * @param itemId the root item ID
     * @return the exported representations of the subtree
     */
    public TreeMarkdownExport exportSubtreeMarkdown(HMTID itemId) {
        List<Item> items = itemRepository.findItemWithDescendants(itemId);
        List<String> itemIds = items.stream().map(i -> i.id().toString()).toList();
        var scalarProps = propertyRepository.findScalarByTargets("item", itemIds);
        var listProps = propertyRepository.findListByTargets("item", itemIds);
        String treeMd = treeMarkdownConverter.itemsToTreeMarkdown(items);
        String ast = markdownParser.parseToAst(treeMd);
        String flatMd = treeMarkdownConverter.itemsToFlatMarkdown(items, scalarProps, listProps);
        return new TreeMarkdownExport(treeMd, ast, flatMd);
    }

    /**
     * Exports items as hierarchical tree markdown, flat markdown, and HTML.
     *
     * @param itemIds the item IDs to export
     * @return the exported representations
     */
    public TreeMarkdownExport exportTreeMarkdown(List<HMTID> itemIds) {
        List<Item> items = itemIds.stream()
            .map(id -> itemRepository.findById(id))
            .filter(Optional::isPresent)
            .map(Optional::get)
            .toList();
        List<String> idStrings = items.stream().map(i -> i.id().toString()).toList();
        var scalarProps = propertyRepository.findScalarByTargets("item", idStrings);
        var listProps = propertyRepository.findListByTargets("item", idStrings);
        String treeMd = treeMarkdownConverter.itemsToTreeMarkdown(items);
        String ast = markdownParser.parseToAst(treeMd);
        String flatMd = treeMarkdownConverter.itemsToFlatMarkdown(items, scalarProps, listProps);
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
    private void reindexPage(HMTID pageId, List<Item> items) {
        LOG.warnv("REINDEX: Reassigning fractional indices for page {0} ({1} items). " +
                "This may cause conflicts with CRDT-based sync.", pageId, items.size());
        List<String> newIndices = FractionalIndexer.generateIndices(items.size());
        for (int i = 0; i < items.size(); i++) {
            itemRepository.updateIndex(items.get(i).id(), newIndices.get(i));
        }
    }

    /**
     * Calculate the fractional index for inserting an item at the specified position.
     */
    private String calculateInsertionIndex(List<Item> existingItems, ItemPosition position) {
        if (existingItems.isEmpty()) {
            return FractionalIndexer.initial();
        }
        return switch (position) {
            case ItemPosition.First() -> FractionalIndexer.between(null, existingItems.getFirst().idx());
            case ItemPosition.Last() -> FractionalIndexer.between(existingItems.getLast().idx(), null);
            case ItemPosition.After(HMTID itemId) -> {
                String afterIdx = null;
                String nextIdx = null;
                for (int i = 0; i < existingItems.size(); i++) {
                    if (existingItems.get(i).id().equals(itemId)) {
                        afterIdx = existingItems.get(i).idx();
                        if (i + 1 < existingItems.size()) {
                            nextIdx = existingItems.get(i + 1).idx();
                        }
                        break;
                    }
                }
                yield FractionalIndexer.between(afterIdx, nextIdx);
            }
        };
    }

    /**
     * Finds the item directly above the insertion position.
     * Returns null if the list is empty or position is FIRST.
     */
    private Item findItemAbove(List<Item> existingItems, ItemPosition position) {
        if (existingItems.isEmpty()) {
            return null;
        }
        return switch (position) {
            case ItemPosition.First() -> null;
            case ItemPosition.Last() -> existingItems.getLast();
            case ItemPosition.After(HMTID itemId) -> existingItems.stream()
                .filter(i -> i.id().equals(itemId)).findFirst().orElse(null);
        };
    }

    private void touchPage(HMTID pageId) {
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
