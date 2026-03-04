package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.List;
import java.util.UUID;

public record Item(
    UUID id,
    UUID pageId,
    UUID parentItemId,   // Parent item ID (null for top-level items)
    String dirPath,
    String name,
    String type,         // "journal" or "doc" (from parent page)
    Instant createdDate,
    Instant modifiedDate,
    String idx,          // Fractional index string for ordering
    int level,           // Indentation level (1-based, 1=top level)
    String body,         // Markdown source text
    String ast,          // Parsed AST JSON
    boolean collapsed,
    List<String> types   // Item types (e.g. "heading", "code")
) {}
