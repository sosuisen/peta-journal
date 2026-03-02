package com.petajournal.server.presentation.rest.dto;

import java.util.List;
import java.util.Map;

public record ItemDto(
        String id,
        String pageId,
        String parentItemId,
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
        List<String> types,
        Map<String, Object> properties) {
}
