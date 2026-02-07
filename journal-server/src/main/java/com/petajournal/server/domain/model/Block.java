package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.List;
import java.util.UUID;

public record Block(
    String id,           // 7-char hex
    UUID pageId,
    Instant createdDate,
    Instant modifiedDate,
    String idx,          // Fractional index string for ordering
    int level,           // Indentation level (0-based)
    String body,         // Markdown source text
    String html,         // Parsed HTML
    boolean collapsed,
    List<String> types,  // Block types (e.g. "task", "heading")
    String propertiesJson
) {}
