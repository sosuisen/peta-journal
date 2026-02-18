package com.petajournal.server.presentation.mcp;

import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import io.quarkiverse.mcp.server.Tool;
import io.quarkiverse.mcp.server.ToolArg;
import jakarta.inject.Inject;

import java.util.List;
import java.util.Map;

public class McpSearchTools {

    @Inject
    SearchService searchService;

    @Inject
    PageRepository pageRepository;

    @Inject
    McpJsonHelper jsonHelper;

    @Tool(description = "Search pages by name. Uses prefix matching first, then fuzzy/token search, then substring matching as fallback.")
    String search_page_names(
            @ToolArg(description = "Search query string") String query,
            @ToolArg(description = "Maximum number of results (default 20)") int maxResults
    ) {
        int max = maxResults > 0 ? maxResults : 20;

        // 1st: SQLite prefix search on dir_path || file_name
        List<Page> prefixPages = pageRepository.findByPathPrefix(query, max);
        if (!prefixPages.isEmpty()) {
            return jsonHelper.toJson(Map.of("results",
                    prefixPages.stream().map(SearchResult::fromPage).toList()));
        }

        // 2nd: Lucene fallback (fuzzy/token-based)
        var luceneResults = searchService.searchPageNames(query, max);
        if (!luceneResults.isEmpty()) {
            return jsonHelper.toJson(Map.of("results", luceneResults));
        }

        // 3rd: SQLite contains search
        List<Page> containsPages = pageRepository.findByPathContains(query, max);
        return jsonHelper.toJson(Map.of("results",
                containsPages.stream().map(SearchResult::fromPage).toList()));
    }

    @Tool(description = "Full-text search across all block content. Returns matching blocks with page info and text snippets.")
    String search_full_text(
            @ToolArg(description = "Search query string") String query,
            @ToolArg(description = "Maximum number of results") int maxResults
    ) {
        var results = searchService.searchFullText(query, maxResults);
        return jsonHelper.toJson(Map.of("results", results, "totalHits", results.size()));
    }

}
