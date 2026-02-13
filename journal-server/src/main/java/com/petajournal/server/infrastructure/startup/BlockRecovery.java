package com.petajournal.server.infrastructure.startup;

import com.petajournal.server.application.ConfigService;
import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.MarkdownParser;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.export.OutlineMarkdownParser;
import io.quarkus.runtime.StartupEvent;
import jakarta.annotation.Priority;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

@ApplicationScoped
public class BlockRecovery {

    private static final Logger LOG = Logger.getLogger(BlockRecovery.class);

    @Inject
    PageRepository pageRepository;

    @Inject
    BlockRepository blockRepository;

    @Inject
    MarkdownParser markdownParser;

    @Inject
    SearchService searchService;

    @Inject
    ConfigService configService;

    void onStartup(@Observes @Priority(50) StartupEvent event) {
        String markdownDir = configService.getMarkdownDir();
        if (markdownDir == null || markdownDir.trim().isEmpty()) return;

        List<Page> pages = pageRepository.findAll();
        int recovered = 0;

        for (Page page : pages) {
            List<Block> blocks = blockRepository.findByPageId(page.id());
            if (!blocks.isEmpty()) continue;

            // Page has 0 blocks — try to recover from markdown file
            String typeDir = page.type() + "s";
            String subDir = page.dirPath();
            Path filePath = subDir.isEmpty()
                    ? Path.of(markdownDir, typeDir, page.fileName() + ".md")
                    : Path.of(markdownDir, typeDir, subDir, page.fileName() + ".md");

            if (!Files.exists(filePath)) {
                LOG.warnv("No markdown file for empty page {0}: {1}", page.fileName(), filePath);
                continue;
            }

            try {
                String content = Files.readString(filePath);
                OutlineMarkdownParser parser = new OutlineMarkdownParser();
                OutlineMarkdownParser.ParseResult result = parser.parse(content);

                List<Block> parsed = parser.toBlocks(result, page.id(), page.dirPath(), page.fileName(), page.type());
                for (Block block : parsed) {
                    // Strip bullet prefix "- " for level > 0 blocks
                    String body = block.body();
                    if (body.startsWith("- ")) {
                        body = body.substring(2);
                    }

                    // Detect and strip checkbox prefix, set status
                    String status = "normal";
                    if (body.startsWith("[ ] ")) {
                        status = "todo";
                        body = body.substring(4);
                    } else if (body.startsWith("[x] ")) {
                        status = "done";
                        body = body.substring(4);
                    }

                    String html = markdownParser.parseToHtml(body);
                    List<String> types = detectBlockTypes(body);
                    Block withHtml = new Block(
                            block.id(), block.pageId(), block.dirPath(), block.fileName(), block.type(),
                            block.createdDate(), block.modifiedDate(),
                            block.idx(), block.level(), body, html,
                            block.collapsed(), status, types, block.propertiesJson()
                    );
                    blockRepository.save(withHtml);
                    if (block.body() != null && !block.body().isBlank()) {
                        searchService.indexBlock(withHtml, page.fileName());
                    }
                }
                recovered++;
                LOG.infov("Recovered {0} blocks for page: {1}", parsed.size(), page.fileName());
            } catch (Exception e) {
                LOG.errorv("Failed to recover blocks for page {0}: {1}", page.fileName(), e.getMessage());
            }
        }

        if (recovered > 0) {
            LOG.infov("Block recovery complete: {0} pages recovered", recovered);
        }
    }

    private List<String> detectBlockTypes(String body) {
        List<String> types = new ArrayList<>();
        if (body == null) return types;
        String trimmed = body.trim();
        if (trimmed.startsWith("#")) {
            types.add("heading");
        }
        if (trimmed.startsWith("```")) {
            types.add("code");
        }
        if (trimmed.startsWith(">")) {
            types.add("blockquote");
        }
        return types;
    }
}
