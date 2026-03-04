package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
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
    ItemRepository itemRepository;

    @Inject
    MarkdownParser markdownParser;

    @Inject
    SearchService searchService;

    public Page importFile(Path filePath) throws IOException {
        String content = Files.readString(filePath);
        String fileName = filePath.getFileName().toString().replaceAll("\\.md$", "");
        String parentDir = filePath.getParent() != null ? filePath.getParent().toString().replace('\\', '/') : "";
        String type = parentDir.contains("journal") ? "journal" : "doc";

        OutlineMarkdownParser parser = new OutlineMarkdownParser();
        OutlineMarkdownParser.ParseResult result = parser.parse(content);

        Page page = parser.toPage(result, "", fileName, type);
        page = pageRepository.save(page);
        searchService.indexPage(page);

        List<Item> items = parser.toItems(result, page.id(), page.dirPath(), page.name(), page.type());
        for (Item item : items) {
            String ast = markdownParser.parseToAst(item.body());
            Item withAst = new Item(
                    item.id(), item.pageId(), item.parentItemId(), item.dirPath(), item.name(), item.type(),
                    item.createdDate(), item.modifiedDate(),
                    item.idx(), item.level(), item.body(), ast,
                    item.collapsed(), item.types()
            );
            itemRepository.save(withAst);
            searchService.indexItem(withAst, page.dirPath() + page.name());
        }

        LOG.infov("Imported page from {0}: {1}", filePath, page.name());
        return page;
    }
}
