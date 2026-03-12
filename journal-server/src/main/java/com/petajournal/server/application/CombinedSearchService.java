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

import java.util.*;

@ApplicationScoped
public class CombinedSearchService {

    private static final Logger LOG = Logger.getLogger(CombinedSearchService.class);

    @Inject
    PageRepository pageRepository;

    @Inject
    SearchService searchService;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    ItemRepository itemRepository;

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

    private List<SearchResult> executeAndGroup(String group, int maxResults) {
        if (group.isBlank()) return List.of();

        String[] tokens = group.split("\\s+");
        List<String> tagTerms = new ArrayList<>();
        List<String> textTokens = new ArrayList<>();
        List<String[]> propFilters = new ArrayList<>();
        List<String> pagePrefixes = new ArrayList<>();
        List<String[]> dateFilters = new ArrayList<>();

        for (String token : tokens) {
            if (token.startsWith("#") && token.length() > 1) {
                tagTerms.add(token.substring(1));
            } else if (token.startsWith("in:") && token.length() > 3) {
                pagePrefixes.add(token.substring(3));
            } else if (token.startsWith("created:") && token.length() > 8) {
                dateFilters.add(new String[]{"created_date", token.substring(8)});
            } else if (token.startsWith("modified:") && token.length() > 9) {
                dateFilters.add(new String[]{"modified_date", token.substring(9)});
            } else if (token.contains("=") && !token.contains("==") && !token.startsWith("=")) {
                int eqIdx = token.indexOf('=');
                propFilters.add(new String[]{token.substring(0, eqIdx), token.substring(eqIdx + 1)});
            } else {
                textTokens.add(token);
            }
        }
        String textQuery = String.join(" ", textTokens).trim();
        boolean hasFilterConditions = !tagTerms.isEmpty() || !propFilters.isEmpty()
                || !pagePrefixes.isEmpty() || !dateFilters.isEmpty();

        // Collect item ID sets for each condition (for AND intersection)
        List<Set<String>> conditions = new ArrayList<>();

        LOG.infof("Combined search: tags=%s, props=%d, prefixes=%s, dates=%d, text='%s'",
                tagTerms, propFilters.size(), pagePrefixes, dateFilters.size(), textQuery);

        int filterLimit = 10000;

        for (String tag : tagTerms) {
            Set<String> ids = new LinkedHashSet<>(
                    propertyRepository.findItemIdsByTagValue(tag, filterLimit));
            LOG.infof("  tag '#%s' → %d items", tag, ids.size());
            conditions.add(ids);
        }

        for (String[] prop : propFilters) {
            Set<String> ids = new LinkedHashSet<>(
                    propertyRepository.findItemIdsByScalarProperty(prop[0], prop[1], filterLimit));
            LOG.infof("  prop '%s=%s' → %d items", prop[0], prop[1], ids.size());
            conditions.add(ids);
        }

        for (String prefix : pagePrefixes) {
            Set<String> ids = new LinkedHashSet<>(
                    propertyRepository.findItemIdsByPagePrefix(prefix, filterLimit));
            LOG.infof("  prefix 'in:%s' → %d items", prefix, ids.size());
            conditions.add(ids);
        }

        for (String[] df : dateFilters) {
            Set<String> ids = new LinkedHashSet<>(
                    propertyRepository.findItemIdsByDateRange(df[0], df[1], filterLimit));
            LOG.infof("  date '%s:%s' → %d items", df[0], df[1], ids.size());
            conditions.add(ids);
        }

        // Text condition
        Set<String> textItemIds = null;
        List<SearchResult> textPageResults = List.of();
        List<SearchResult> textItemResults = List.of();
        if (!textQuery.isEmpty()) {
            textPageResults = searchPageNamesInternal(textQuery, maxResults);
            textItemResults = searchService.searchFullText(textQuery, hasFilterConditions ? filterLimit : maxResults);
            textItemIds = new LinkedHashSet<>();
            for (SearchResult r : textItemResults) {
                if (r.itemId() != null && !r.itemId().isEmpty()) {
                    textItemIds.add(r.itemId());
                }
            }
            if (hasFilterConditions) {
                conditions.add(textItemIds);
            }
        }

        // Intersect all conditions
        Set<String> matchedItemIds = null;
        if (!conditions.isEmpty()) {
            matchedItemIds = new LinkedHashSet<>(conditions.getFirst());
            for (int i = 1; i < conditions.size(); i++) {
                matchedItemIds.retainAll(conditions.get(i));
            }
            LOG.infof("  intersection → %d items (sets: %s)", matchedItemIds.size(),
                    conditions.stream().map(s -> s.stream().limit(3).toList().toString()).toList());
        }

        // Build results
        List<SearchResult> results = new ArrayList<>();

        if (matchedItemIds != null) {
            Map<String, SearchResult> snippetMap = new HashMap<>();
            if (textItemResults != null) {
                for (SearchResult r : textItemResults) {
                    if (r.itemId() != null) snippetMap.put(r.itemId(), r);
                }
            }

            for (String itemId : matchedItemIds) {
                if (results.size() >= maxResults) break;
                if (snippetMap.containsKey(itemId)) {
                    results.add(snippetMap.get(itemId));
                    continue;
                }
                Optional<Item> itemOpt = itemRepository.findById(UUID.fromString(itemId));
                if (itemOpt.isEmpty()) continue;
                Item item = itemOpt.get();
                Optional<Page> pageOpt = pageRepository.findById(item.pageId());
                if (pageOpt.isEmpty()) continue;
                Page page = pageOpt.get();
                String pageTitle = page.dirPath() + page.name();
                String snippet = item.body() != null ? item.body() : "";
                if (snippet.length() > 200) snippet = snippet.substring(0, 200);
                results.add(new SearchResult(page.id(), pageTitle, itemId, snippet, 1.0f));
            }
        }

        // Pure text search: include page results
        if (!hasFilterConditions && !textQuery.isEmpty()) {
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

        return results;
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
