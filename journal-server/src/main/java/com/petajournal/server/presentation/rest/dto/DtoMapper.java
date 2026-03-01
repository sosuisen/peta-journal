package com.petajournal.server.presentation.rest.dto;

import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Bookmark;
import com.petajournal.server.domain.model.BookmarkGroup;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;

import java.util.List;

public final class DtoMapper {

    private DtoMapper() {}

    public static PageDto toDto(Page page) {
        return new PageDto(
                page.id().toString(),
                page.parentId() != null ? page.parentId().toString() : "",
                page.dirPath(),
                page.name(),
                page.type() != null ? page.type() : "doc",
                page.createdDate().toEpochMilli(),
                page.modifiedDate().toEpochMilli(),
                page.tags() != null ? page.tags() : List.of());
    }

    public static BlockDto toDto(Block block) {
        return new BlockDto(
                block.id().toString(),
                block.pageId().toString(),
                block.parentBlockId() != null ? block.parentBlockId().toString() : null,
                block.dirPath(),
                block.name(),
                block.type() != null ? block.type() : "doc",
                block.createdDate().toEpochMilli(),
                block.modifiedDate().toEpochMilli(),
                block.idx(),
                block.level(),
                block.body(),
                block.html(),
                block.collapsed(),
                block.status() != null ? block.status() : "normal",
                block.types() != null ? block.types() : List.of());
    }

    public static SearchResultDto toDto(SearchResult result) {
        return new SearchResultDto(
                result.pageId().toString(),
                result.pageTitle(),
                result.blockId() != null ? result.blockId() : "",
                result.snippet() != null ? result.snippet() : "",
                result.score());
    }

    public static BookmarkGroupDto toDto(BookmarkGroup group) {
        return new BookmarkGroupDto(
                group.id().toString(),
                group.name(),
                group.sortOrder(),
                group.createdAt().toEpochMilli());
    }

    public static BookmarkDto toDto(Bookmark bookmark) {
        return new BookmarkDto(
                bookmark.id().toString(),
                bookmark.groupId().toString(),
                bookmark.targetType(),
                bookmark.targetId(),
                bookmark.sortOrder(),
                bookmark.createdAt().toEpochMilli(),
                bookmark.targetPage() != null ? toDto(bookmark.targetPage()) : null);
    }

    public static BacklinkEntryDto toDto(BacklinkEntry entry) {
        return new BacklinkEntryDto(
                toDto(entry.sourcePage()),
                entry.sourceBlock() != null ? toDto(entry.sourceBlock()) : null);
    }
}
