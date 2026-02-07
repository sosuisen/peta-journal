package com.petajournal.server.application;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.PageTitle;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.persistence.BlockIdGenerator;
import com.petajournal.server.infrastructure.persistence.FractionalIndexer;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.time.Instant;
import java.time.LocalDate;
import java.time.format.DateTimeFormatter;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

@ApplicationScoped
public class JournalService {

    @Inject
    PageRepository pageRepository;

    @Inject
    BlockRepository blockRepository;

    @Inject
    SearchService searchService;

    public record JournalResult(Page page, List<Block> blocks, boolean created) {}

    public JournalResult getOrCreateJournal(LocalDate date) {
        Optional<Page> existing = pageRepository.findJournal(date);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Block> blocks = blockRepository.findByPageId(page.id());
            return new JournalResult(page, blocks, false);
        }

        // Create new journal page
        UUID id = UUID.randomUUID();
        Instant now = Instant.now();
        String dirPath = "journals/" + date.getYear() + "/" + String.format("%02d", date.getMonthValue());
        String fileName = date.format(DateTimeFormatter.ISO_LOCAL_DATE);
        String title = date.format(DateTimeFormatter.ofPattern("yyyy年M月d日"));

        List<PageTitle> titles = List.of(new PageTitle(id, 0, title));
        List<String> tags = List.of("journal");

        Page page = new Page(id, dirPath, fileName, now, now, tags, titles, "{}");
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create initial empty block
        String blockId = BlockIdGenerator.generate();
        Block block = new Block(blockId, id, now, now, FractionalIndexer.initial(), 0, "", "", false, "normal", List.of(), "{}");
        block = blockRepository.save(block);

        return new JournalResult(page, List.of(block), true);
    }
}
