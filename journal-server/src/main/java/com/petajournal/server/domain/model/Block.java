package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.List;
import java.util.UUID;

public record Block(
    UUID id,
    UUID pageId,
    UUID parentBlockId,  // Parent block ID (null for top-level blocks)
    String dirPath,
    String name,
    String type,         // "journal" or "doc" (from parent page)
    Instant createdDate,
    Instant modifiedDate,
    String idx,          // Fractional index string for ordering
    int level,           // Indentation level (1-based, 1=top level)
    String body,         // Markdown source text
    String html,         // Parsed HTML
    boolean collapsed,
    String status,       // Block status: normal, todo, running, done
    List<String> types   // Block types (e.g. "heading", "code")
) {}
