package com.petajournal.server.presentation.rest.dto;

public record SearchResultDto(
        String pageId,
        String pageTitle,
        String itemId,
        String snippet,
        float score) {
}
