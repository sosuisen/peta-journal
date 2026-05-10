package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;

import java.util.List;

/** Full-text search service for pages and items. */
public interface SearchService {

    /**
     * Indexes a page for search.
     *
     * @param page the page to index
     */
    void indexPage(Page page);

    /**
     * Indexes an item for full-text search.
     *
     * @param item      the item to index
     * @param pageTitle the title of the page containing the item
     */
    void indexItem(Item item, String pageTitle);

    /**
     * Removes a page and all its items from the search index.
     *
     * @param pageId the page HMTID as a string
     */
    void removePage(String pageId);

    /**
     * Removes an item from the search index.
     *
     * @param itemId the item ID
     */
    void removeItem(String itemId);

    /**
     * Searches for pages by name using fuzzy/token matching.
     *
     * @param query      the search query
     * @param maxResults maximum number of results
     * @return the matching search results
     */
    List<SearchResult> searchPageNames(String query, int maxResults);

    /**
     * Performs a full-text search across all item content.
     *
     * @param query      the search query
     * @param maxResults maximum number of results
     * @return the matching search results with highlighted snippets
     */
    List<SearchResult> searchFullText(String query, int maxResults);

    /**
     * Deletes all entries from the search index.
     */
    void deleteAll();

    /**
     * Checks whether the search index is empty.
     *
     * @return {@code true} if the index contains no documents
     */
    boolean isEmpty();
}
