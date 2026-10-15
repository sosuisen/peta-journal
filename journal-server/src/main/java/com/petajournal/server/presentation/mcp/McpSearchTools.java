package com.petajournal.server.presentation.mcp;

import com.petajournal.server.application.CombinedSearchService;
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

    private final SearchService searchService;
    private final PageRepository pageRepository;
    private final McpJsonHelper jsonHelper;
    private final CombinedSearchService combinedSearchService;

    @Inject
    public McpSearchTools(
            SearchService searchService,
            PageRepository pageRepository,
            McpJsonHelper jsonHelper,
            CombinedSearchService combinedSearchService) {
        this.searchService = searchService;
        this.pageRepository = pageRepository;
        this.jsonHelper = jsonHelper;
        this.combinedSearchService = combinedSearchService;
    }

    @Tool(description = "Search pages by name. Uses prefix matching first, then fuzzy/token search, then substring matching as fallback.")
    String search_page_names(
            @ToolArg(description = "Search query string") String query,
            @ToolArg(description = "Maximum number of results (default 20)") int maxResults
    ) {
        int max = maxResults > 0 ? maxResults : 20;

        // 1st: SQLite prefix search on dir_path || name
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

    @Tool(description = "Combined search supporting #tag, key=value properties, in:path page prefix, created:/modified: date filters, plain text, and OR/AND operators. Examples: '#report in:DailyReport/2026-03', 'status=done', 'created:2026-03', '#urgent OR #important'")
    String search_combined(
            @ToolArg(description = "Search query string") String query,
            @ToolArg(description = "Maximum number of results") int maxResults
    ) {
        int max = maxResults > 0 ? maxResults : 20;
        var results = combinedSearchService.search(query, max);
        return jsonHelper.toJson(Map.of("results", results));
    }

}
