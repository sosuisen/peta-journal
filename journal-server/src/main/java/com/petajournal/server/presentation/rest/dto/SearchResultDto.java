package com.petajournal.server.presentation.rest.dto;

public record SearchResultDto(
        String pageId,
        String pageTitle,
        String blockId,
        String snippet,
        float score) {
}
