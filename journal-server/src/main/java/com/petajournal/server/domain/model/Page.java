package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.UUID;

public record Page(
    UUID id,
    UUID parentId,
    String dirPath,
    String name,
    String type,        // "journal" or "doc"
    Instant createdDate,
    Instant modifiedDate
) {
}
