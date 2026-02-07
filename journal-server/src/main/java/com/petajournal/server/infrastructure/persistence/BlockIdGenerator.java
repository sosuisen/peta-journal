package com.petajournal.server.infrastructure.persistence;

import java.util.UUID;

public final class BlockIdGenerator {

    private BlockIdGenerator() {}

    /**
     * Generates a 7-character hex block ID from a random UUID.
     */
    public static String generate() {
        return UUID.randomUUID().toString().replace("-", "").substring(0, 7);
    }
}
