package com.petajournal.server.domain;

import java.util.UUID;

public final class BlockIdGenerator {

    private BlockIdGenerator() {}

    /**
     * Generates a random UUID for a block.
     */
    public static UUID generate() {
        return UUID.randomUUID();
    }
}
