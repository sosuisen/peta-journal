package com.petajournal.server.domain.model;

import com.sosuisha.hmtid.HMTID;

import java.util.Objects;

/**
 * The new place of one item in a move.
 *
 * @param itemId the item to move
 * @param idx    the index the item gets on the target page
 */
public record ItemPlacement(HMTID itemId, String idx) {

    /**
     * Checks that both values are present.
     *
     * @throws NullPointerException if itemId or idx is null
     */
    public ItemPlacement {
        Objects.requireNonNull(itemId, "itemId must not be null");
        Objects.requireNonNull(idx, "idx must not be null");
    }
}
