package com.petajournal.server.presentation.rest.dto;

public record BookmarkDto(
        String id,
        String groupId,
        String targetType,
        String targetId,
        String sortOrder,
        long createdAt,
        PageDto targetPage) {
}
