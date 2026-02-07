package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;

@ApplicationScoped
public class ImportService {

    private static final Logger LOG = Logger.getLogger(ImportService.class);

    @Inject
    PageRepository pageRepository;

    @Inject
    BlockRepository blockRepository;

    @Inject
    MarkdownParser markdownParser;

    @Inject
    SearchService searchService;

    public Page importFile(Path filePath) throws IOException {
        String content = Files.readString(filePath);
        String fileName = filePath.getFileName().toString().replaceAll("\\.md$", "");
        String dirPath = filePath.getParent() != null ? filePath.getParent().toString() : "pages";

        OutlineMarkdownParser parser = new OutlineMarkdownParser();
        OutlineMarkdownParser.ParseResult result = parser.parse(content);

        Page page = parser.toPage(result, dirPath, fileName);
        page = pageRepository.save(page);
        searchService.indexPage(page);

        List<Block> blocks = parser.toBlocks(result, page.id());
        for (Block block : blocks) {
            String html = markdownParser.parseToHtml(block.body());
            Block withHtml = new Block(
                    block.id(), block.pageId(), block.createdDate(), block.modifiedDate(),
                    block.idx(), block.level(), block.body(), html,
                    block.collapsed(), block.types(), block.propertiesJson()
            );
            blockRepository.save(withHtml);
            searchService.indexBlock(withHtml, page.primaryTitle());
        }

        LOG.infov("Imported page from {0}: {1}", filePath, page.primaryTitle());
        return page;
    }
}
