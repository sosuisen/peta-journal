package com.petajournal.server.domain.model;

public record BacklinkEntry(
    Page sourcePage,
    Block sourceBlock
) {}
