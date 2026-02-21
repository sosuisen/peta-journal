package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.BlockService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.util.List;
import java.util.Map;
import java.util.UUID;

public class McpBlockTools {

    @Inject
    BlockService blockService;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Create a new block in a page. The block is inserted after the specified block, or at the end if afterBlockId is empty.")
    String create_block(
            @ToolArg(description = "UUID of the page") String pageId,
            @ToolArg(description = "Block ID to insert after (empty string to append at end)") String afterBlockId,
            @ToolArg(description = "Indentation level (1-based)") int level,
            @ToolArg(description = "Markdown body text") String body
    ) {
        var block = blockService.createBlock(
                UUID.fromString(pageId),
                afterBlockId.isEmpty() ? null : UUID.fromString(afterBlockId),
                level,
                body
        );
        return jsonHelper.toJson(Map.of("block", block));
    }

    @Tool(description = "Update an existing block's body, level, collapsed state, or status. Only non-null fields are updated.")
    String update_block(
            @ToolArg(description = "UUID of the block") String blockId,
            @ToolArg(description = "New markdown body (null to keep current)") String body,
            @ToolArg(description = "New indentation level, or -1 to keep current") int level,
            @ToolArg(description = "New collapsed state: 'true', 'false', or 'null' to keep current") String collapsed,
            @ToolArg(description = "New status: 'normal', 'todo', 'running', 'done', or 'null' to keep current") String status
    ) {
        Integer lvl = (level < 1) ? null : level;
        Boolean col = "null".equals(collapsed) ? null : Boolean.parseBoolean(collapsed);
        String st = "null".equals(status) ? null : status;
        var block = blockService.updateBlock(UUID.fromString(blockId), body, lvl, col, st, null);
        return jsonHelper.toJson(Map.of("block", block));
    }

    @Tool(description = "Delete a block by its ID")
    String delete_block(
            @ToolArg(description = "UUID of the block") String blockId
    ) {
        blockService.deleteBlock(UUID.fromString(blockId));
        return jsonHelper.toJson(Map.of("deleted", true));
    }

    @Tool(description = "Move blocks to a target page and position. Returns blocks with updated indices.")
    String move_blocks(
            @ToolArg(description = "Comma-separated block IDs to move") String blockIds,
            @ToolArg(description = "UUID of the target page") String targetPageId,
            @ToolArg(description = "Block ID to insert after (empty string to insert at beginning)") String afterBlockId,
            @ToolArg(description = "Level delta adjustment, or 0 for no change") int levelDelta
    ) {
        List<UUID> ids = List.of(blockIds.split(",")).stream().map(UUID::fromString).toList();
        Integer delta = (levelDelta == 0) ? null : levelDelta;
        var blocks = blockService.moveBlocks(ids, UUID.fromString(targetPageId), afterBlockId.isEmpty() ? null : UUID.fromString(afterBlockId), delta);
        return jsonHelper.toJson(Map.of("blocks", blocks));
    }

    @Tool(description = "Update a block's status (normal, todo, running, done)")
    String update_block_status(
            @ToolArg(description = "UUID of the block") String blockId,
            @ToolArg(description = "New status: 'normal', 'todo', 'running', or 'done'") String status
    ) {
        var block = blockService.updateBlockStatus(UUID.fromString(blockId), status);
        return jsonHelper.toJson(Map.of("block", block));
    }

    @Tool(description = "Get all blocks with the specified statuses (e.g. all 'todo' and 'running' blocks)")
    String get_blocks_by_status(
            @ToolArg(description = "Comma-separated statuses to filter by (e.g. 'todo,running')") String statuses
    ) {
        List<String> statusList = List.of(statuses.split(","));
        var blocks = blockService.getBlocksByStatuses(statusList);
        return jsonHelper.toJson(Map.of("blocks", blocks));
    }

    @Tool(description = "Export blocks as hierarchical tree markdown. Returns tree markdown, flat markdown, and HTML representations.")
    String export_tree_markdown(
            @ToolArg(description = "Comma-separated block IDs to export") String blockIds
    ) {
        List<UUID> ids = List.of(blockIds.split(",")).stream().map(UUID::fromString).toList();
        var result = blockService.exportTreeMarkdown(ids);
        return jsonHelper.toJson(Map.of(
                "treeMarkdown", result.treeMarkdown(),
                "html", result.html(),
                "flatMarkdown", result.flatMarkdown()
        ));
    }
}
