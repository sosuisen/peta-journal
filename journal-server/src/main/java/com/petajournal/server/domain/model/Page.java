package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.List;
import java.util.UUID;

public record Page(
    UUID id,
    String dirPath,
    String fileName,
    Instant createdDate,
    Instant modifiedDate,
    List<String> tags,
    String propertiesJson
) {
}
