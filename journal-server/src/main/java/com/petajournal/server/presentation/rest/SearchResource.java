package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.CombinedSearchService;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.SearchResultDto;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import com.sosuisha.hmtid.HMTID;
import java.util.ArrayList;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Optional;
import java.util.Set;

@Path("/api/search")
@Produces(MediaType.APPLICATION_JSON)
public class SearchResource {

    @Inject
    PageRepository pageRepository;

    @Inject
    SearchService searchService;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    ItemRepository itemRepository;

    @Inject
    CombinedSearchService combinedSearchService;

    // --- Response records ---

    record SearchPageNamesResponse(List<SearchResultDto> results) {}

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
    @Path("/combined")
    public SearchPageNamesResponse searchCombined(
            @QueryParam("q") String query,
            @QueryParam("maxResults") @DefaultValue("20") int maxResults) {

        var results = combinedSearchService.search(query, maxResults);
        return new SearchPageNamesResponse(results.stream().map(DtoMapper::toDto).toList());
    }

    @GET
    @Path("/tags")
    public SearchPageNamesResponse searchByTag(
            @QueryParam("q") String tagValue,
            @QueryParam("maxResults") @DefaultValue("20") int maxResults) {

        List<String> itemIds = propertyRepository.findItemIdsByTagValue(tagValue, maxResults);
        List<SearchResultDto> results = new ArrayList<>();

        for (String itemId : itemIds) {
            Optional<Item> itemOpt = itemRepository.findById(HMTID.of(itemId));
            if (itemOpt.isEmpty()) continue;
            Item item = itemOpt.get();
            Optional<Page> pageOpt = pageRepository.findById(item.pageId());
            if (pageOpt.isEmpty()) continue;
            Page page = pageOpt.get();
            String pageTitle = page.dirPath() + page.name();
            String snippet = item.body() != null ? item.body() : "";
            if (snippet.length() > 200) snippet = snippet.substring(0, 200);
            SearchResult sr = new SearchResult(page.id(), pageTitle, itemId, snippet, 1.0f);
            results.add(DtoMapper.toDto(sr));
        }

        return new SearchPageNamesResponse(results);
    }

}
