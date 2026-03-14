package com.petajournal.server.presentation.rest.dto;

import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Set;

public final class FieldSelector {

    private FieldSelector() {}

    /**
     * Extracts only the specified fields from an ItemDto.
     *
     * @param dto        the ItemDto to extract fields from
     * @param fieldsCsv  comma-separated field names (e.g. "body,level,types")
     * @return a map containing only the requested fields and their values
     */
    public static Map<String, Object> selectFields(ItemDto dto, String fieldsCsv) {
        Set<String> requested = Set.of(fieldsCsv.split(","));
        Map<String, Object> all = toMap(dto);
        Map<String, Object> result = new LinkedHashMap<>();
        for (String field : requested) {
            if (all.containsKey(field)) {
                result.put(field, all.get(field));
            }
        }
        return result;
    }

    private static Map<String, Object> toMap(ItemDto dto) {
        Map<String, Object> map = new LinkedHashMap<>();
        map.put("id", dto.id());
        map.put("pageId", dto.pageId());
        map.put("parentItemId", dto.parentItemId());
        map.put("dirPath", dto.dirPath());
        map.put("name", dto.name());
        map.put("type", dto.type());
        map.put("createdDate", dto.createdDate());
        map.put("modifiedDate", dto.modifiedDate());
        map.put("idx", dto.idx());
        map.put("level", dto.level());
        map.put("body", dto.body());
        map.put("ast", dto.ast());
        map.put("collapsed", dto.collapsed());
        map.put("types", dto.types());
        map.put("properties", dto.properties());
        map.put("transcludeItemId", dto.transcludeItemId());
        return map;
    }
}
