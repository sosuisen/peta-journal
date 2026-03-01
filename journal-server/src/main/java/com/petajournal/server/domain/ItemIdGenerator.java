package com.petajournal.server.domain;

import java.util.UUID;

public final class ItemIdGenerator {

    private ItemIdGenerator() {}

    /**
     * Generates a random UUID for an item.
     */
    public static UUID generate() {
        return UUID.randomUUID();
    }
}
