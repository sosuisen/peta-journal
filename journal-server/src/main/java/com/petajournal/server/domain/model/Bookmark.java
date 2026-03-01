package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.UUID;

public record Bookmark(
    UUID id,
    UUID groupId,
    String targetType,    // "page" or "item"
    String targetId,
    String sortOrder,     // fractional index within group
    Instant createdAt,
    Page targetPage       // resolved page data (nullable)
) {}
