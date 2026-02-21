package com.petajournal.server.domain.model;

import java.time.Instant;
import java.util.UUID;

public record BookmarkGroup(
    UUID id,
    String name,          // empty = default group
    String sortOrder,     // fractional index
    Instant createdAt
) {}
