package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.ItemService;
import com.petajournal.server.application.PageService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import com.sosuisha.hmtid.HMTID;
import java.util.Map;

public class McpPageTools {

    @Inject
    PageService pageService;

    @Inject
    ItemService itemService;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Get a page and its items by page ID")
    String get_page_by_id(
            @ToolArg(description = "UUID of the page") String pageId
    ) {
        var page = pageService.getPage(HMTID.of(pageId))
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));
        var items = pageService.getPageItems(page.id());
        return jsonHelper.toJson(Map.of("page", page, "items", items));
    }

    @Tool(description = "Get or create a document page by title. Returns the page and its items. If the page does not exist, it is created with an empty item.")
    String get_or_create_doc(
            @ToolArg(description = "Title of the document (used as file name)") String title,
            @ToolArg(description = "Directory path (e.g. 'projects/'). Empty string for root.") String dirPath
    ) {
        var result = pageService.getOrCreateDoc(title, dirPath);
        return jsonHelper.toJson(Map.of(
                "page", result.page(),
                "items", result.items(),
                "created", result.created()
        ));
    }

    @Tool(description = "Export a page's items as flat markdown, tree markdown, and AST. Includes item properties (begin, end, tags, etc.) in HTML comments.")
    String export_page_markdown(
            @ToolArg(description = "UUID of the page") String pageId
    ) {
        var result = itemService.exportPageMarkdown(HMTID.of(pageId));
        return jsonHelper.toJson(Map.of(
                "treeMarkdown", result.treeMarkdown(),
                "ast", result.ast(),
                "flatMarkdown", result.flatMarkdown()
        ));
    }

    @Tool(description = "Update a page's title or directory path")
    String update_page(
            @ToolArg(description = "UUID of the page") String pageId,
            @ToolArg(description = "New title (null to keep current)") String title,
            @ToolArg(description = "New directory path (null to keep current)") String dirPath
    ) {
        String t = (title == null || title.isEmpty()) ? null : title;
        String d = (dirPath == null) ? null : dirPath;
        var page = pageService.updatePage(HMTID.of(pageId), t, d);
        return jsonHelper.toJson(Map.of("page", page));
    }

    @Tool(description = "Delete a page and all its items")
    String delete_page(
            @ToolArg(description = "UUID of the page to delete") String pageId
    ) {
        pageService.deletePage(HMTID.of(pageId));
        return jsonHelper.toJson(Map.of("deleted", true));
    }
}
