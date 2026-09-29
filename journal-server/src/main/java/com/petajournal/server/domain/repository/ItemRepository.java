package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.ItemPatch;
import com.petajournal.server.domain.model.ItemPlacement;
import com.sosuisha.hmtid.HMTID;

import java.time.Instant;
import java.util.List;
import java.util.Optional;

/** Repository for item persistence. */
public interface ItemRepository {

    Optional<Item> findById(HMTID id);

    List<Item> findByPageId(HMTID pageId);

    Item save(Item item);

    /**
     * Writes only the fields set in the patch, in one transaction.
     * Columns that the patch leaves out are not read and not written.
     *
     * @param id    the item to update
     * @param patch the fields to write
     * @return true if the item exists
     * @throws NullPointerException if id or patch is null
     */
    boolean patch(HMTID id, ItemPatch patch);

    /**
     * Adds delta to the level of every given item, in one transaction.
     * The new level is computed in the database from the current level,
     * so the result does not depend on the order in which requests arrive.
     * A level never goes below 1.
     *
     * @param ids          the items to shift
     * @param delta        the amount to add to each level; may be negative
     * @param modifiedDate new modified date of the items
     * @throws NullPointerException if ids or modifiedDate is null
     */
    void shiftLevels(List<HMTID> ids, int delta, Instant modifiedDate);

    /**
     * Moves items to their new places on a page, in one transaction.
     * Either every item moves or none does. Only the page, the index, the level
     * and the modified date are written; the body and the other fields are not touched.
     * The parent of each moved item is cleared; call {@link #recalculateParentIds} afterwards.
     *
     * @param targetPageId the page the items move to
     * @param placements   the items and their new indices
     * @param levelDelta   the amount to add to each level; a level never goes below 1
     * @param modifiedDate new modified date of the items
     * @throws NullPointerException     if targetPageId, placements or modifiedDate is null
     * @throws IllegalArgumentException if an item does not exist
     */
    void moveItems(HMTID targetPageId, List<ItemPlacement> placements, int levelDelta, Instant modifiedDate);

    void delete(HMTID id);

    void deleteByPageId(HMTID pageId);

    List<Item> findByPageIdContainingLink(String pageTitle);

    List<Item> findByTypes(List<String> types);

    void updateIndex(HMTID itemId, String newIdx);

    /** Finds all items that transclude the given source item. */
    List<Item> findTranscluders(HMTID sourceItemId);

    /** Finds an item and all its descendants using a recursive query on parent_id. */
    List<Item> findItemWithDescendants(HMTID itemId);

    /** Recalculates parent_id for all items on a page based on idx ordering and level. */
    void recalculateParentIds(HMTID pageId);
}
