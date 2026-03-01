package com.petajournal.server.presentation.rest;

import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.SearchResultDto;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Set;

@Path("/api/search")
@Produces(MediaType.APPLICATION_JSON)
public class SearchResource {

    @Inject
    PageRepository pageRepository;

    @Inject
    SearchService searchService;

    // --- Response records ---

    record SearchPageNamesResponse(List<SearchResultDto> results) {}
    record SearchFullTextResponse(List<SearchResultDto> results, int totalHits) {}

    @GET
    @Path("/pages")
    public SearchPageNamesResponse searchPageNames(
            @QueryParam("q") String query,
            @QueryParam("maxResults") @DefaultValue("20") int maxResults) {

        Set<String> seenIds = new LinkedHashSet<>();
        List<SearchResultDto> allResults = new ArrayList<>();

        // 0th: Index pages whose last dir_path segment matches query
        for (Page p : pageRepository.findIndexPagesByDirSegment(query, maxResults)) {
            if (seenIds.add(p.id().toString())) {
                allResults.add(DtoMapper.toDto(SearchResult.fromPage(p)));
            }
        }

        // 1st: SQLite prefix search on dir_path || name
        for (Page p : pageRepository.findByPathPrefix(query, maxResults)) {
            if (seenIds.add(p.id().toString())) {
                allResults.add(DtoMapper.toDto(SearchResult.fromPage(p)));
            }
        }

        if (!allResults.isEmpty()) {
            return new SearchPageNamesResponse(allResults.stream().limit(maxResults).toList());
        }

        // 2nd: Lucene fallback (fuzzy/token-based)
        var luceneResults = searchService.searchPageNames(query, maxResults);
        if (!luceneResults.isEmpty()) {
            return new SearchPageNamesResponse(luceneResults.stream().map(DtoMapper::toDto).toList());
        }

        // 3rd: SQLite contains search
        List<Page> containsPages = pageRepository.findByPathContains(query, maxResults);
        return new SearchPageNamesResponse(
                containsPages.stream()
                        .map(SearchResult::fromPage)
                        .map(DtoMapper::toDto)
                        .toList());
    }

    @GET
    @Path("/fulltext")
    public SearchFullTextResponse searchFullText(
            @QueryParam("q") String query,
            @QueryParam("maxResults") @DefaultValue("20") int maxResults) {
        var results = searchService.searchFullText(query, maxResults);
        return new SearchFullTextResponse(
                results.stream().map(DtoMapper::toDto).toList(),
                results.size());
    }
}
