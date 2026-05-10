package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.PageService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import com.sosuisha.hmtid.HMTID;
import java.util.Map;

public class McpNavigationTools {

    @Inject
    PageService pageService;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Get pages and items that link to a given page (backlinks). Each entry contains the source page and the item containing the link.")
    String get_backlinks(
            @ToolArg(description = "HMTID of the target page") String pageId) {
        var entries = pageService.getBacklinks(HMTID.of(pageId));
        return jsonHelper.toJson(Map.of("entries", entries));
    }
}
