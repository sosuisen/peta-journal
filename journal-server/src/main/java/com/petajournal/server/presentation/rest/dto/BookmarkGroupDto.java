package com.petajournal.server.presentation.rest.dto;

public record BookmarkGroupDto(
        String id,
        String name,
        String sortOrder,
        long createdAt) {
}
