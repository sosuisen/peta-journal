package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;

import java.util.List;

public interface SearchService {

    void indexPage(Page page);

    void indexBlock(Block block, String pageTitle);

    void removePage(String pageId);

    void removeBlock(String blockId);

    List<SearchResult> searchPageNames(String query, int maxResults);

    List<SearchResult> searchFullText(String query, int maxResults);

    void deleteAll();
}
