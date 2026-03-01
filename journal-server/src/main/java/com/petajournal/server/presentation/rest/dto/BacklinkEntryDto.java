package com.petajournal.server.presentation.rest.dto;

public record BacklinkEntryDto(
        PageDto sourcePage,
        BlockDto sourceBlock) {
}
