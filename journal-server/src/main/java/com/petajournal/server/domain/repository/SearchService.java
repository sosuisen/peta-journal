package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;

import java.util.List;

/** Full-text search service for pages and blocks. */
public interface SearchService {

    /**
     * Indexes a page for search.
     *
     * @param page the page to index
     */
    void indexPage(Page page);

    /**
     * Indexes a block for full-text search.
     *
     * @param block     the block to index
     * @param pageTitle the title of the page containing the block
     */
    void indexBlock(Block block, String pageTitle);

    /**
     * Removes a page and all its blocks from the search index.
     *
     * @param pageId the page UUID as a string
     */
    void removePage(String pageId);

    /**
     * Removes a block from the search index.
     *
     * @param blockId the block ID
     */
    void removeBlock(String blockId);

    /**
     * Searches for pages by name using fuzzy/token matching.
     *
     * @param query      the search query
     * @param maxResults maximum number of results
     * @return the matching search results
     */
    List<SearchResult> searchPageNames(String query, int maxResults);

    /**
     * Performs a full-text search across all block content.
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
