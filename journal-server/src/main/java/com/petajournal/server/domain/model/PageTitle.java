package com.petajournal.server.domain.model;

import java.util.UUID;

public record PageTitle(
    UUID pageId,
    int orderIndex,
    String title
) {}
