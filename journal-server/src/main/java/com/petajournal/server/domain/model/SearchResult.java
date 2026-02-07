package com.petajournal.server.domain.model;

import java.util.UUID;

public record SearchResult(
    UUID pageId,
    String pageTitle,
    String blockId,
    String snippet,
    float score
) {}
