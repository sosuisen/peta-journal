package com.petajournal.server.domain.model;

import com.sosuisha.hmtid.HMTID;

public record SearchResult(
    HMTID pageId,
    String pageTitle,
    String itemId,
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
        String title = page.dirPath() + page.name();
        return new SearchResult(page.id(), title, "", title, 1.0f);
    }
}
