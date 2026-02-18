package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.JournalService;
import com.petajournal.server.application.PageService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.time.LocalDate;
import java.util.LinkedHashMap;
import java.util.Map;

public class McpJournalTools {

    @Inject
    JournalService journalService;

    @Inject
    PageService pageService;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Get or create a journal page by file name. Supports daily (YYYY-MM-DD), weekly (YYYY-WNN), monthly (YYYY-MM), and yearly (YYYY) formats.")
    String get_or_create_journal(
            @ToolArg(description = "Journal file name (e.g. '2026-02-18', '2026-W08', '2026-02', '2026')") String fileName
    ) {
        var result = journalService.getOrCreateJournalByName(fileName);
        return jsonHelper.toJson(Map.of(
                "page", result.page(),
                "blocks", result.blocks(),
                "created", result.created()
        ));
    }

    @Tool(description = "Get the most recent daily journal before the specified date")
    String get_most_recent_journal_before(
            @ToolArg(description = "Date in YYYY-MM-DD format (exclusive upper bound)") String beforeDate
    ) {
        var result = journalService.getMostRecentJournalBefore(LocalDate.parse(beforeDate));
        Map<String, Object> response = new LinkedHashMap<>();
        response.put("found", result.found());
        if (result.found()) {
            response.put("page", result.page());
            response.put("blocks", result.blocks());
        }
        return jsonHelper.toJson(response);
    }

    @Tool(description = "Get recently modified pages, ordered by modification date (newest first)")
    String get_recent_pages(
            @ToolArg(description = "Maximum number of results (default 10)") int maxResults,
            @ToolArg(description = "Filter by page type: 'journal', 'doc', or empty for all") String type
    ) {
        int max = maxResults > 0 ? maxResults : 10;
        String t = (type == null || type.isEmpty()) ? null : type;
        var pages = pageService.getRecentPages(max, t);
        return jsonHelper.toJson(Map.of("pages", pages));
    }
}
