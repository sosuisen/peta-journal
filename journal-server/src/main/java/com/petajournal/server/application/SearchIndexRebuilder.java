package com.petajournal.server.application;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
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

    @Inject
    PageRepository pageRepository;

    @Inject
    BlockRepository blockRepository;

    @Inject
    SearchService searchService;

    void onStartup(@Observes @Priority(100) StartupEvent event) {
        if (!searchService.isEmpty()) {
            return;
        }
        LOG.info("Building search index...");

        List<Page> pages = pageRepository.findAll();
        for (Page page : pages) {
            searchService.indexPage(page);
            String pageTitle = page.dirPath() + page.fileName();
            List<Block> blocks = blockRepository.findByPageId(page.id());
            for (Block block : blocks) {
                if (block.body() != null && !block.body().isBlank()) {
                    searchService.indexBlock(block, pageTitle);
                }
            }
        }
        LOG.infov("Search index built: {0} pages indexed", pages.size());
    }
}
