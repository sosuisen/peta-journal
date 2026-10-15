package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.ItemService;
import com.petajournal.server.domain.model.ItemPosition;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.petajournal.server.presentation.rest.dto.FieldSelector;
import com.petajournal.server.presentation.rest.dto.ItemDtoAssembler;
import com.sosuisha.hmtid.HMTID;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.util.List;
import java.util.Map;

public class McpItemTools {

    @Inject
    ItemService itemService;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    McpJsonHelper jsonHelper;

    @Inject
    ItemDtoAssembler dtos;

    @Tool(description = "Get a single item by its ID. Returns the item with its properties. Use 'fields' to return only specific fields (comma-separated, e.g. 'body,level,types').")
    String get_item(
            @ToolArg(description = "ID of the item") String itemId,
            @ToolArg(description = "Comma-separated field names to return (e.g. 'body,level'). Empty string for all fields.") String fields
    ) {
        var item = itemService.getItemById(HMTID.of(itemId));
        var dto = dtos.toDto(item);
        if (fields != null && !fields.isEmpty()) {
            return jsonHelper.toJson(FieldSelector.selectFields(dto, fields));
        }
        return jsonHelper.toJson(Map.of("item", dto));
    }

    @Tool(description = "Create a new item in a page. Use position to specify where: 'first' for page start, 'last' for page end, 'after:<id>' to insert after a specific item.")
    String create_item(
            @ToolArg(description = "ID of the page") String pageId,
            @ToolArg(description = "Position: 'first', 'last', or 'after:<id>'") String position,
            @ToolArg(description = "Indentation level (1-based)") int level,
            @ToolArg(description = "Markdown body text") String body
    ) {
        var item = itemService.createItem(
                HMTID.of(pageId),
                ItemPosition.parse(position),
                level,
                body,
                null
        );
        return jsonHelper.toJson(Map.of("item", item));
    }

    @Tool(description = "Update an existing item's body, level, or collapsed state. Only non-null fields are updated.")
    String update_item(
            @ToolArg(description = "ID of the item") String itemId,
            @ToolArg(description = "New markdown body (null to keep current)") String body,
            @ToolArg(description = "New indentation level, or -1 to keep current") int level,
            @ToolArg(description = "New collapsed state: 'true', 'false', or 'null' to keep current") String collapsed
    ) {
        Integer lvl = (level < 1) ? null : level;
        Boolean col = (collapsed == null || "null".equals(collapsed)) ? null : Boolean.parseBoolean(collapsed);
        var item = itemService.updateItem(HMTID.of(itemId), body, lvl, col);
        return jsonHelper.toJson(Map.of("item", item));
    }

    @Tool(description = "Delete an item by its ID")
    String delete_item(
            @ToolArg(description = "ID of the item") String itemId
    ) {
        itemService.deleteItem(HMTID.of(itemId));
        return jsonHelper.toJson(Map.of("deleted", true));
    }

    @Tool(description = "Move items to a target page and position. Returns items with updated indices.")
    String move_items(
            @ToolArg(description = "Comma-separated item IDs to move") String itemIds,
            @ToolArg(description = "ID of the target page") String targetPageId,
            @ToolArg(description = "Position: 'first', 'last', or 'after:<id>'") String position,
            @ToolArg(description = "Level delta adjustment, or 0 for no change") int levelDelta
    ) {
        List<HMTID> ids = List.of(itemIds.split(",")).stream().map(HMTID::of).toList();
        Integer delta = (levelDelta == 0) ? null : levelDelta;
        var items = itemService.moveItems(ids, HMTID.of(targetPageId), ItemPosition.parse(position), delta);
        return jsonHelper.toJson(Map.of("items", items));
    }

    @Tool(description = "Get all items with the specified types (e.g. all 'task' items)")
    String get_items_by_type(
            @ToolArg(description = "Comma-separated types to filter by (e.g. 'task')") String types
    ) {
        List<String> typeList = List.of(types.split(","));
        var items = itemService.getItemsByTypes(typeList);
        return jsonHelper.toJson(Map.of("items", items));
    }

    @Tool(description = "Export items as hierarchical tree markdown. Returns tree markdown, flat markdown, and HTML representations.")
    String export_tree_markdown(
            @ToolArg(description = "Comma-separated item IDs to export") String itemIds
    ) {
        List<HMTID> ids = List.of(itemIds.split(",")).stream().map(HMTID::of).toList();
        var result = itemService.exportTreeMarkdown(ids);
        return jsonHelper.toJson(Map.of(
                "treeMarkdown", result.treeMarkdown(),
                "ast", result.ast(),
                "flatMarkdown", result.flatMarkdown()
        ));
    }

    @Tool(description = "Export an item and all its descendants (subtree) as tree markdown, flat markdown, and AST. Use this to get a complete subtree rooted at a specific item.")
    String export_subtree_markdown(
            @ToolArg(description = "ID of the root item") String itemId
    ) {
        var result = itemService.exportSubtreeMarkdown(HMTID.of(itemId));
        return jsonHelper.toJson(Map.of(
                "treeMarkdown", result.treeMarkdown(),
                "ast", result.ast(),
                "flatMarkdown", result.flatMarkdown()
        ));
    }

    @Tool(description = "Cycle the task state of an item: non-task → task, task → started (begin), started → completed (end), completed → non-task.")
    String cycle_task(
            @ToolArg(description = "ID of the item") String itemId
    ) {
        var item = itemService.cycleTask(HMTID.of(itemId));
        return jsonHelper.toJson(Map.of("item", dtos.toDto(item)));
    }

    @Tool(description = "Set a transclusion on an item, making it mirror a source item's content.")
    String set_transclusion(
            @ToolArg(description = "ID of the item to set as a transclusion") String itemId,
            @ToolArg(description = "ID of the source item to transclude") String sourceItemId
    ) {
        var item = itemService.setTransclusion(HMTID.of(itemId), HMTID.of(sourceItemId));
        return jsonHelper.toJson(Map.of("item", dtos.toDto(item)));
    }

    @Tool(description = "Remove a transclusion from an item, clearing its body.")
    String remove_transclusion(
            @ToolArg(description = "ID of the transclusion item") String itemId
    ) {
        var item = itemService.removeTransclusion(HMTID.of(itemId));
        return jsonHelper.toJson(Map.of("item", dtos.toDto(item)));
    }

    @Tool(description = "Upsert a scalar property on an item (e.g. begin, end, due, plan, cancelled).")
    String upsert_property(
            @ToolArg(description = "ID of the item") String itemId,
            @ToolArg(description = "Property key (e.g. 'begin', 'end', 'due', 'plan')") String key,
            @ToolArg(description = "Value type: 'text', 'number', 'checkbox', 'date', or 'datetime'") String valueType,
            @ToolArg(description = "Property value as string") String value
    ) {
        var item = itemService.getItemById(HMTID.of(itemId));
        propertyRepository.upsertScalar("item", itemId, key, valueType, value);
        return jsonHelper.toJson(Map.of("item", dtos.toDto(item)));
    }

    @Tool(description = "Convert tree markdown (indented list format) to flat markdown.")
    String import_tree_markdown(
            @ToolArg(description = "Tree markdown string to convert") String treeMarkdown
    ) {
        String flatMd = itemService.importTreeMarkdown(treeMarkdown);
        return jsonHelper.toJson(Map.of("flatMarkdown", flatMd));
    }
}
