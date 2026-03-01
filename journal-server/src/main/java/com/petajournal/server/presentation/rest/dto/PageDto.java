package com.petajournal.server.presentation.rest.dto;

import java.util.List;

public record PageDto(
        String id,
        String parentId,
        String dirPath,
        String name,
        String type,
        long createdDate,
        long modifiedDate,
        List<String> tags) {
}
