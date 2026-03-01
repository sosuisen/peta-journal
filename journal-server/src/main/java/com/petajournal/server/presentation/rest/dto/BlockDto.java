package com.petajournal.server.presentation.rest.dto;

import java.util.List;

public record BlockDto(
        String id,
        String pageId,
        String dirPath,
        String name,
        String type,
        long createdDate,
        long modifiedDate,
        String idx,
        int level,
        String body,
        String html,
        boolean collapsed,
        String status,
        List<String> types) {
}
