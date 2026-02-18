package com.petajournal.server.domain.model;

import java.util.UUID;

public record SearchResult(
    UUID pageId,
    String pageTitle,
    String blockId,
    String snippet,
    float score
) {
    /**
     * Creates a search result from a page (used when converting SQLite page results
     * to the search result format).
     *
     * @param page the source page
     * @return a search result with the page's full path as the title
     */
    public static SearchResult fromPage(Page page) {
        String title = page.dirPath() + page.fileName();
        return new SearchResult(page.id(), title, "", title, 1.0f);
    }
}
