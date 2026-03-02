package com.petajournal.server.presentation.rest.dto;

public record PageDto(
        String id,
        String parentId,
        String dirPath,
        String name,
        String type,
        long createdDate,
        long modifiedDate) {
}
