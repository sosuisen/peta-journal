package com.petajournal.server.presentation.rest.dto;

import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;

import java.util.List;
import java.util.Map;
import java.util.TreeMap;

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
                page.modifiedDate().toEpochMilli());
    }

    public static ItemDto toDto(Item item) {
        return new ItemDto(
                item.id().toString(),
                item.pageId().toString(),
                item.parentItemId() != null ? item.parentItemId().toString() : null,
                item.dirPath(),
                item.name(),
                item.type() != null ? item.type() : "doc",
                item.createdDate().toEpochMilli(),
                item.modifiedDate().toEpochMilli(),
                item.idx(),
                item.level(),
                item.body(),
                item.ast(),
                item.collapsed(),
                item.types() != null ? item.types() : List.of(),
                Map.of(),
                item.transcludeItemId() != null ? item.transcludeItemId().toString() : null);
    }

    public static ItemDto toDto(Item item, Map<String, String> scalars, Map<String, List<String>> lists) {
        Map<String, Object> properties = new TreeMap<>();
        if (scalars != null) properties.putAll(scalars);
        if (lists != null) properties.putAll(lists);
        return new ItemDto(
                item.id().toString(),
                item.pageId().toString(),
                item.parentItemId() != null ? item.parentItemId().toString() : null,
                item.dirPath(),
                item.name(),
                item.type() != null ? item.type() : "doc",
                item.createdDate().toEpochMilli(),
                item.modifiedDate().toEpochMilli(),
                item.idx(),
                item.level(),
                item.body(),
                item.ast(),
                item.collapsed(),
                item.types() != null ? item.types() : List.of(),
                properties,
                item.transcludeItemId() != null ? item.transcludeItemId().toString() : null);
    }

    public static List<ItemDto> toDtos(List<Item> items,
                                        Map<String, Map<String, String>> scalarProps,
                                        Map<String, Map<String, List<String>>> listProps) {
        return items.stream().map(item -> {
            String id = item.id().toString();
            return toDto(item,
                    scalarProps.getOrDefault(id, Map.of()),
                    listProps.getOrDefault(id, Map.of()));
        }).toList();
    }

    public static SearchResultDto toDto(SearchResult result) {
        return new SearchResultDto(
                result.pageId().toString(),
                result.pageTitle(),
                result.itemId() != null ? result.itemId() : "",
                result.snippet() != null ? result.snippet() : "",
                result.score());
    }

    public static BacklinkEntryDto toDto(BacklinkEntry entry) {
        return new BacklinkEntryDto(
                toDto(entry.sourcePage()),
                entry.sourceItem() != null ? toDto(entry.sourceItem()) : null);
    }
}
