package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Item;

import java.util.List;
import java.util.Optional;
import java.util.UUID;

/** Repository for item persistence. */
public interface ItemRepository {

    Optional<Item> findById(UUID id);

    List<Item> findByPageId(UUID pageId);

    Item save(Item item);

    void delete(UUID id);

    void deleteByPageId(UUID pageId);

    List<Item> findByPageIdContainingLink(String pageTitle);

    List<Item> findByTypes(List<String> types);

    void updateIndex(UUID itemId, String newIdx);

    /** Finds all items that transclude the given source item. */
    List<Item> findTranscluders(UUID sourceItemId);

    /** Finds an item and all its descendants using a recursive query on parent_id. */
    List<Item> findItemWithDescendants(UUID itemId);

    /** Recalculates parent_id for all items on a page based on idx ordering and level. */
    void recalculateParentIds(UUID pageId);
}
