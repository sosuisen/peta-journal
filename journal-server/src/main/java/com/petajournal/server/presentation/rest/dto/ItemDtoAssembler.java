package com.petajournal.server.presentation.rest.dto;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.sosuisha.hmtid.HMTID;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

/**
 * Builds item DTOs with their properties loaded, and the transcluded trees a list of items refers to.
 * The properties of a list of items are loaded in two queries, not one per item.
 */
@ApplicationScoped
public class ItemDtoAssembler {

    private static final String ITEM_TARGET = "item";
    /** How deep a transclusion inside a transclusion is followed. */
    private static final int MAX_TRANSCLUSION_DEPTH = 5;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    ItemRepository itemRepository;

    /** Converts one item, loading its properties. */
    public ItemDto toDto(Item item) {
        String id = item.id().toString();
        var scalars = propertyRepository.findScalarByTargets(ITEM_TARGET, List.of(id)).getOrDefault(id, Map.of());
        var lists = propertyRepository.findListByTargets(ITEM_TARGET, List.of(id)).getOrDefault(id, Map.of());
        return DtoMapper.toDto(item, scalars, lists);
    }

    /** Converts items in order, loading all their properties at once. */
    public List<ItemDto> toDtos(List<Item> items) {
        List<String> ids = items.stream().map(i -> i.id().toString()).toList();
        var scalars = propertyRepository.findScalarByTargets(ITEM_TARGET, ids);
        var lists = propertyRepository.findListByTargets(ITEM_TARGET, ids);
        return DtoMapper.toDtos(items, scalars, lists);
    }

    /**
     * Loads the source trees of every transclusion among the items, and of transclusions
     * found inside those trees, up to {@link #MAX_TRANSCLUSION_DEPTH} levels deep.
     * A source that no longer exists maps to an empty list.
     *
     * @param items the items of a page
     * @return source item ID to the source item followed by its descendants, in outline order
     */
    public Map<String, List<ItemDto>> transcludedTrees(List<Item> items) {
        Map<String, List<ItemDto>> trees = new LinkedHashMap<>();
        Set<String> visited = new HashSet<>();
        loadTranscludedTrees(items, trees, visited, 0);
        return trees;
    }

    private void loadTranscludedTrees(List<Item> items, Map<String, List<ItemDto>> trees, Set<String> visited, int depth) {
        for (Item item : items) {
            if (item.transcludeItemId() == null) continue;
            String sourceId = item.transcludeItemId().toString();
            if (!trees.containsKey(sourceId)) {
                loadTranscludedTree(sourceId, trees, visited, depth);
            }
        }
    }

    private void loadTranscludedTree(String sourceId, Map<String, List<ItemDto>> trees, Set<String> visited, int depth) {
        if (depth >= MAX_TRANSCLUSION_DEPTH || visited.contains(sourceId)) return;
        visited.add(sourceId);

        List<Item> descendants = itemRepository.findItemWithDescendants(HMTID.of(sourceId));
        if (descendants.isEmpty()) {
            trees.put(sourceId, List.of());
            return;
        }

        trees.put(sourceId, toDtos(descendants));

        // Recursively load transcluded trees for nested transclusions
        loadTranscludedTrees(descendants, trees, visited, depth + 1);
    }
}
