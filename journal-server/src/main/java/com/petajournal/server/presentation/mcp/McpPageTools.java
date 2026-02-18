package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.PageService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.util.List;
import java.util.Map;
import java.util.UUID;

public class McpPageTools {

    @Inject
    PageService pageService;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Get a page and its blocks by page ID")
    String get_page(
            @ToolArg(description = "UUID of the page") String pageId
    ) {
        var page = pageService.getPage(UUID.fromString(pageId))
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));
        var blocks = pageService.getPageBlocks(page.id());
        return jsonHelper.toJson(Map.of("page", page, "blocks", blocks));
    }

    @Tool(description = "Get or create a document page by title. Returns the page and its blocks. If the page does not exist, it is created with an empty block.")
    String get_doc_by_name(
            @ToolArg(description = "Title of the document (used as file name)") String title,
            @ToolArg(description = "Directory path (e.g. 'projects/'). Empty string for root.") String dirPath,
            @ToolArg(description = "Comma-separated tags (e.g. 'tag1,tag2'). Empty string for no tags.") String tags
    ) {
        List<String> tagList = (tags == null || tags.isEmpty()) ? List.of() : List.of(tags.split(","));
        var result = pageService.getOrCreateDoc(title, dirPath, tagList);
        return jsonHelper.toJson(Map.of(
                "page", result.page(),
                "blocks", result.blocks(),
                "created", result.created()
        ));
    }

    @Tool(description = "Update a page's title, tags, or directory path")
    String update_page(
            @ToolArg(description = "UUID of the page") String pageId,
            @ToolArg(description = "New title (null to keep current)") String title,
            @ToolArg(description = "Comma-separated tags (null to keep current)") String tags,
            @ToolArg(description = "New directory path (null to keep current)") String dirPath
    ) {
        List<String> tagList = (tags == null || tags.isEmpty()) ? null : List.of(tags.split(","));
        String t = (title == null || title.isEmpty()) ? null : title;
        String d = (dirPath == null) ? null : dirPath;
        var page = pageService.updatePage(UUID.fromString(pageId), t, tagList, null, d);
        return jsonHelper.toJson(Map.of("page", page));
    }

    @Tool(description = "Delete a page and all its blocks")
    String delete_page(
            @ToolArg(description = "UUID of the page to delete") String pageId
    ) {
        pageService.deletePage(UUID.fromString(pageId));
        return jsonHelper.toJson(Map.of("deleted", true));
    }
}
