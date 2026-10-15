package com.petajournal.server.application;

import com.petajournal.server.domain.HashtagExtractor;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import io.quarkus.runtime.StartupEvent;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.util.List;

@ApplicationScoped
public class HashtagBackfiller {

    private static final Logger LOG = Logger.getLogger(HashtagBackfiller.class);

    private final PageRepository pageRepository;
    private final ItemRepository itemRepository;
    private final PropertyRepository propertyRepository;

    @Inject
    public HashtagBackfiller(
            PageRepository pageRepository,
            ItemRepository itemRepository,
            PropertyRepository propertyRepository) {
        this.pageRepository = pageRepository;
        this.itemRepository = itemRepository;
        this.propertyRepository = propertyRepository;
    }

    void onStartup(@Observes @Priority(50) StartupEvent event) {
        if (propertyRepository.findAnyTagExists()) {
            return;
        }

        LOG.info("Backfilling hashtag tags for existing items...");
        int count = 0;

        List<Page> pages = pageRepository.findAll();
        for (Page page : pages) {
            List<Item> items = itemRepository.findByPageId(page.id());
            for (Item item : items) {
                if (item.ast() == null || item.ast().isBlank()) continue;
                List<String> hashtags = HashtagExtractor.extractHashtags(item.ast());
                if (!hashtags.isEmpty()) {
                    propertyRepository.syncTags("item", item.id().toString(), hashtags);
                    count++;
                }
            }
        }

        LOG.infov("Hashtag backfill complete: {0} items tagged", count);
    }
}
