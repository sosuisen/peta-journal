package com.petajournal.server.application;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import io.quarkus.runtime.StartupEvent;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.util.List;

@ApplicationScoped
public class SearchIndexRebuilder {

    private static final Logger LOG = Logger.getLogger(SearchIndexRebuilder.class);

    private final PageRepository pageRepository;
    private final ItemRepository itemRepository;
    private final SearchService searchService;

    @Inject
    public SearchIndexRebuilder(
            PageRepository pageRepository,
            ItemRepository itemRepository,
            SearchService searchService) {
        this.pageRepository = pageRepository;
        this.itemRepository = itemRepository;
        this.searchService = searchService;
    }

    void onStartup(@Observes @Priority(100) StartupEvent event) {
        if (!searchService.isEmpty()) {
            return;
        }
        LOG.info("Building search index...");

        List<Page> pages = pageRepository.findAll();
        for (Page page : pages) {
            searchService.indexPage(page);
            String pageTitle = page.dirPath() + page.name();
            List<Item> items = itemRepository.findByPageId(page.id());
            for (Item item : items) {
                if (item.body() != null && !item.body().isBlank()) {
                    searchService.indexItem(item, pageTitle);
                }
            }
        }
        LOG.infov("Search index built: {0} pages indexed", pages.size());
    }
}
