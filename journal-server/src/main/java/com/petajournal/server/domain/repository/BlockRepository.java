package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Block;

import java.util.List;
import java.util.Optional;
import java.util.UUID;

/** Repository for block persistence. */
public interface BlockRepository {

    /**
     * Finds a block by its ID.
     *
     * @param id the 7-char hex block ID
     * @return the block, or empty if not found
     */
    Optional<Block> findById(String id);

    /**
     * Returns all blocks belonging to a page, ordered by index.
     *
     * @param pageId the page UUID
     * @return the ordered list of blocks
     */
    List<Block> findByPageId(UUID pageId);

    /**
     * Saves a block (insert or update).
     *
     * @param block the block to save
     * @return the saved block
     */
    Block save(Block block);

    /**
     * Deletes a block by its ID.
     *
     * @param id the block ID
     */
    void delete(String id);

    /**
     * Deletes all blocks belonging to a page.
     *
     * @param pageId the page UUID
     */
    void deleteByPageId(UUID pageId);

    /**
     * Finds blocks whose body contains a wiki link to the given page title.
     *
     * @param pageTitle the page title to search for in wiki links
     * @return the matching blocks
     */
    List<Block> findByPageIdContainingLink(String pageTitle);

    /**
     * Finds all blocks matching the given statuses.
     *
     * @param statuses the list of statuses to filter by (e.g. {@code ["todo", "running"]})
     * @return the matching blocks
     */
    List<Block> findByStatuses(List<String> statuses);

    /**
     * Updates the fractional index of a block.
     *
     * @param blockId the block ID
     * @param newIdx  the new fractional index value
     */
    void updateIndex(String blockId, String newIdx);
}
