package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.ItemService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.util.List;
import java.util.Map;
import java.util.UUID;

public class McpItemTools {

    @Inject
    ItemService itemService;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Create a new item in a page. The item is inserted after the specified item, or at the end if afterItemId is empty.")
    String create_item(
            @ToolArg(description = "UUID of the page") String pageId,
            @ToolArg(description = "Item ID to insert after (empty string to append at end)") String afterItemId,
            @ToolArg(description = "Indentation level (1-based)") int level,
            @ToolArg(description = "Markdown body text") String body
    ) {
        var item = itemService.createItem(
                UUID.fromString(pageId),
                afterItemId.isEmpty() ? null : UUID.fromString(afterItemId),
                level,
                body,
                null
        );
        return jsonHelper.toJson(Map.of("item", item));
    }

    @Tool(description = "Update an existing item's body, level, or collapsed state. Only non-null fields are updated.")
    String update_item(
            @ToolArg(description = "UUID of the item") String itemId,
            @ToolArg(description = "New markdown body (null to keep current)") String body,
            @ToolArg(description = "New indentation level, or -1 to keep current") int level,
            @ToolArg(description = "New collapsed state: 'true', 'false', or 'null' to keep current") String collapsed
    ) {
        Integer lvl = (level < 1) ? null : level;
        Boolean col = "null".equals(collapsed) ? null : Boolean.parseBoolean(collapsed);
        var item = itemService.updateItem(UUID.fromString(itemId), body, lvl, col);
        return jsonHelper.toJson(Map.of("item", item));
    }

    @Tool(description = "Delete an item by its ID")
    String delete_item(
            @ToolArg(description = "UUID of the item") String itemId
    ) {
        itemService.deleteItem(UUID.fromString(itemId));
        return jsonHelper.toJson(Map.of("deleted", true));
    }

    @Tool(description = "Move items to a target page and position. Returns items with updated indices.")
    String move_items(
            @ToolArg(description = "Comma-separated item IDs to move") String itemIds,
            @ToolArg(description = "UUID of the target page") String targetPageId,
            @ToolArg(description = "Item ID to insert after (empty string to insert at beginning)") String afterItemId,
            @ToolArg(description = "Level delta adjustment, or 0 for no change") int levelDelta
    ) {
        List<UUID> ids = List.of(itemIds.split(",")).stream().map(UUID::fromString).toList();
        Integer delta = (levelDelta == 0) ? null : levelDelta;
        var items = itemService.moveItems(ids, UUID.fromString(targetPageId), afterItemId.isEmpty() ? null : UUID.fromString(afterItemId), delta);
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
        List<UUID> ids = List.of(itemIds.split(",")).stream().map(UUID::fromString).toList();
        var result = itemService.exportTreeMarkdown(ids);
        return jsonHelper.toJson(Map.of(
                "treeMarkdown", result.treeMarkdown(),
                "html", result.html(),
                "flatMarkdown", result.flatMarkdown()
        ));
    }
}
