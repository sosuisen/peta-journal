package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import com.sosuisha.hmtid.HMTID;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.LinkedHashSet;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.Set;

/**
 * Search over items and pages with filter terms: {@code #tag}, {@code key=value},
 * {@code in:path}, {@code created:}/{@code modified:} date filters and plain text,
 * combined with spaces (AND) and {@code OR}.
 */
@ApplicationScoped
public class CombinedSearchService {

    private static final Logger LOG = Logger.getLogger(CombinedSearchService.class);
    /** Filter conditions are read in full, then intersected; this caps each of them. */
    private static final int FILTER_LIMIT = 10000;
    private static final int SNIPPET_LENGTH = 200;

    private final PageRepository pageRepository;
    private final SearchService searchService;
    private final PropertyRepository propertyRepository;
    private final ItemRepository itemRepository;

    @Inject
    public CombinedSearchService(
            PageRepository pageRepository,
            SearchService searchService,
            PropertyRepository propertyRepository,
            ItemRepository itemRepository) {
        this.pageRepository = pageRepository;
        this.searchService = searchService;
        this.propertyRepository = propertyRepository;
        this.itemRepository = itemRepository;
    }

    public List<SearchResult> search(String query, int maxResults) {
        if (query == null || query.isBlank()) {
            return List.of();
        }

        // Split by " OR " (case-sensitive) into OR-groups
        String[] orGroups = query.split("\\s+OR\\s+");
        Map<String, SearchResult> merged = new LinkedHashMap<>();

        for (String group : orGroups) {
            List<SearchResult> groupResults = executeAndGroup(group.trim(), maxResults);
            for (SearchResult r : groupResults) {
                String key = r.itemId() != null && !r.itemId().isEmpty()
                        ? r.pageId() + ":" + r.itemId() : r.pageId().toString();
                merged.putIfAbsent(key, r);
            }
        }

        return merged.values().stream().limit(maxResults).toList();
    }

    /** The terms of one AND group, sorted by kind. Package-private for tests. */
    record QueryTerms(
            List<String> tags,
            List<String[]> propertyFilters,
            List<String> pagePrefixes,
            List<String[]> dateFilters,
            String text) {

        boolean hasFilters() {
            return !tags.isEmpty() || !propertyFilters.isEmpty() || !pagePrefixes.isEmpty() || !dateFilters.isEmpty();
        }

        static QueryTerms parse(String group) {
            List<String> tags = new ArrayList<>();
            List<String[]> propertyFilters = new ArrayList<>();
            List<String> pagePrefixes = new ArrayList<>();
            List<String[]> dateFilters = new ArrayList<>();
            List<String> textTokens = new ArrayList<>();

            for (String token : group.split("\\s+")) {
                if (token.startsWith("#") && token.length() > 1) {
                    tags.add(token.substring(1));
                } else if (token.startsWith("in:") && token.length() > 3) {
                    pagePrefixes.add(token.substring(3));
                } else if (token.startsWith("created:") && token.length() > 8) {
                    dateFilters.add(new String[]{"created_date", token.substring(8)});
                } else if (token.startsWith("modified:") && token.length() > 9) {
                    dateFilters.add(new String[]{"modified_date", token.substring(9)});
                } else if (token.contains("=") && !token.contains("==") && !token.startsWith("=")) {
                    int eqIdx = token.indexOf('=');
                    propertyFilters.add(new String[]{token.substring(0, eqIdx), token.substring(eqIdx + 1)});
                } else {
                    textTokens.add(token);
                }
            }
            return new QueryTerms(tags, propertyFilters, pagePrefixes, dateFilters,
                    String.join(" ", textTokens).trim());
        }
    }

    private List<SearchResult> executeAndGroup(String group, int maxResults) {
        if (group.isBlank()) return List.of();

        QueryTerms terms = QueryTerms.parse(group);
        boolean hasFilterConditions = terms.hasFilters();

        LOG.infof("Combined search: tags=%s, props=%d, prefixes=%s, dates=%d, text='%s'",
                terms.tags(), terms.propertyFilters().size(), terms.pagePrefixes(), terms.dateFilters().size(), terms.text());

        // Collect item ID sets for each condition (for AND intersection)
        List<Set<String>> conditions = new ArrayList<>();

        for (String tag : terms.tags()) {
            Set<String> ids = new LinkedHashSet<>(propertyRepository.findItemIdsByTagValue(tag, FILTER_LIMIT));
            LOG.infof("  tag '#%s' → %d items", tag, ids.size());
            conditions.add(ids);
        }

        for (String[] prop : terms.propertyFilters()) {
            Set<String> ids = new LinkedHashSet<>(
                    propertyRepository.findItemIdsByScalarProperty(prop[0], prop[1], FILTER_LIMIT));
            LOG.infof("  prop '%s=%s' → %d items", prop[0], prop[1], ids.size());
            conditions.add(ids);
        }

        for (String prefix : terms.pagePrefixes()) {
            Set<String> ids = new LinkedHashSet<>(propertyRepository.findItemIdsByPagePrefix(prefix, FILTER_LIMIT));
            LOG.infof("  prefix 'in:%s' → %d items", prefix, ids.size());
            conditions.add(ids);
        }

        for (String[] df : terms.dateFilters()) {
            Set<String> ids = new LinkedHashSet<>(propertyRepository.findItemIdsByDateRange(df[0], df[1], FILTER_LIMIT));
            LOG.infof("  date '%s:%s' → %d items", df[0], df[1], ids.size());
            conditions.add(ids);
        }

        // Text condition
        String textQuery = terms.text();
        List<SearchResult> textPageResults = List.of();
        List<SearchResult> textItemResults = List.of();
        if (!textQuery.isEmpty()) {
            textPageResults = searchPageNamesInternal(textQuery, maxResults);
            textItemResults = searchService.searchFullText(textQuery, hasFilterConditions ? FILTER_LIMIT : maxResults);
            if (hasFilterConditions) {
                Set<String> textItemIds = new LinkedHashSet<>();
                for (SearchResult r : textItemResults) {
                    if (r.itemId() != null && !r.itemId().isEmpty()) {
                        textItemIds.add(r.itemId());
                    }
                }
                conditions.add(textItemIds);
            }
        }

        // Pure text search: page results first, then the items of other pages
        if (!hasFilterConditions) {
            if (textQuery.isEmpty()) return List.of();
            List<SearchResult> combined = new ArrayList<>();
            Set<String> seenPageIds = new HashSet<>();
            for (SearchResult r : textPageResults) {
                combined.add(r);
                seenPageIds.add(r.pageId().toString());
            }
            for (SearchResult r : textItemResults) {
                if (!seenPageIds.contains(r.pageId().toString())) {
                    combined.add(r);
                }
            }
            return combined.stream().limit(maxResults).toList();
        }

        // Intersect all conditions
        Set<String> matchedItemIds = new LinkedHashSet<>(conditions.getFirst());
        for (int i = 1; i < conditions.size(); i++) {
            matchedItemIds.retainAll(conditions.get(i));
        }
        LOG.infof("  intersection → %d items (sets: %s)", matchedItemIds.size(),
                conditions.stream().map(s -> s.stream().limit(3).toList().toString()).toList());

        // Build results: items found by the text search keep their highlighted snippet
        Map<String, SearchResult> snippetMap = new HashMap<>();
        for (SearchResult r : textItemResults) {
            if (r.itemId() != null) snippetMap.put(r.itemId(), r);
        }
        List<SearchResult> results = new ArrayList<>();
        for (String itemId : matchedItemIds) {
            if (results.size() >= maxResults) break;
            SearchResult withSnippet = snippetMap.get(itemId);
            if (withSnippet != null) {
                results.add(withSnippet);
                continue;
            }
            resultForItem(itemId).ifPresent(results::add);
        }
        return results;
    }

    /**
     * Builds search results for the given items, with the start of the body as snippet.
     * Items or pages that no longer exist are skipped.
     *
     * @param itemIds the item IDs, in result order
     */
    public List<SearchResult> resultsForItems(List<String> itemIds) {
        List<SearchResult> results = new ArrayList<>();
        for (String itemId : itemIds) {
            resultForItem(itemId).ifPresent(results::add);
        }
        return results;
    }

    private Optional<SearchResult> resultForItem(String itemId) {
        Optional<Item> itemOpt = itemRepository.findById(HMTID.of(itemId));
        if (itemOpt.isEmpty()) return Optional.empty();
        Item item = itemOpt.get();
        Optional<Page> pageOpt = pageRepository.findById(item.pageId());
        if (pageOpt.isEmpty()) return Optional.empty();
        Page page = pageOpt.get();
        String pageTitle = page.dirPath() + page.name();
        String snippet = item.body() != null ? item.body() : "";
        if (snippet.length() > SNIPPET_LENGTH) snippet = snippet.substring(0, SNIPPET_LENGTH);
        return Optional.of(new SearchResult(page.id(), pageTitle, itemId, snippet, 1.0f));
    }

    private List<SearchResult> searchPageNamesInternal(String query, int maxResults) {
        Set<String> seenIds = new LinkedHashSet<>();
        List<SearchResult> all = new ArrayList<>();
        for (Page p : pageRepository.findIndexPagesByDirSegment(query, maxResults)) {
            if (seenIds.add(p.id().toString())) all.add(SearchResult.fromPage(p));
        }
        for (Page p : pageRepository.findByPathPrefix(query, maxResults)) {
            if (seenIds.add(p.id().toString())) all.add(SearchResult.fromPage(p));
        }
        if (!all.isEmpty()) return all.stream().limit(maxResults).toList();
        var lucene = searchService.searchPageNames(query, maxResults);
        if (!lucene.isEmpty()) return lucene;
        return pageRepository.findByPathContains(query, maxResults).stream()
                .map(SearchResult::fromPage).toList();
    }
}
