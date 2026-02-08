package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.PageTitle;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.PageRepository;
import io.quarkus.scheduler.Scheduled;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.channels.FileChannel;
import java.nio.channels.FileLock;
import java.nio.file.*;
import java.util.List;
import java.util.UUID;

@ApplicationScoped
public class MarkdownExportService {

    private static final Logger LOG = Logger.getLogger(MarkdownExportService.class);

    @ConfigProperty(name = "petajournal.export.dir")
    String exportDir;

    @Inject
    ExportQueueRepository exportQueueRepository;

    @Inject
    PageRepository pageRepository;

    @Inject
    BlockRepository blockRepository;

    public void queueExport(UUID pageId) {
        exportQueueRepository.enqueue(pageId);
    }

    @Scheduled(every = "${petajournal.export.interval}s", delay = 5, delayUnit = java.util.concurrent.TimeUnit.SECONDS)
    void processExportQueue() {
        List<UUID> pageIds = exportQueueRepository.dequeueAll();
        for (UUID pageId : pageIds) {
            try {
                exportPage(pageId);
            } catch (Exception e) {
                LOG.warnv("Failed to export page {0}: {1}", pageId, e.getMessage());
                // Re-queue for retry
                exportQueueRepository.enqueue(pageId);
            }
        }
    }

    void exportPage(UUID pageId) throws IOException {
        var pageOpt = pageRepository.findById(pageId);
        if (pageOpt.isEmpty()) return;

        Page page = pageOpt.get();
        List<Block> blocks = blockRepository.findByPageId(pageId);

        String markdown = serializeToOutlineMarkdown(page, blocks);
        Path dir = Path.of(resolveHome(exportDir), page.dirPath());
        Files.createDirectories(dir);
        Path filePath = dir.resolve(page.fileName() + ".md");

        try {
            writeWithLock(filePath, markdown);
        } catch (IOException e) {
            // File locked, write to alternate name
            Path altPath = dir.resolve(page.fileName() + ".conflict.md");
            LOG.warnv("File locked, writing to alternate: {0}", altPath);
            Files.writeString(altPath, markdown, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING);
        }

        LOG.debugv("Exported page {0} to {1}", page.primaryTitle(), filePath);
    }

    String serializeToOutlineMarkdown(Page page, List<Block> blocks) {
        StringBuilder sb = new StringBuilder();

        // YAML frontmatter
        sb.append("---\n");
        sb.append("id: ").append(page.id()).append("\n");
        if (page.titles() != null && !page.titles().isEmpty()) {
            if (page.titles().size() == 1) {
                sb.append("title: ").append(yamlEscape(page.titles().getFirst().title())).append("\n");
            } else {
                sb.append("titles:\n");
                for (PageTitle t : page.titles()) {
                    sb.append("  - ").append(yamlEscape(t.title())).append("\n");
                }
            }
        }
        if (page.tags() != null && !page.tags().isEmpty()) {
            sb.append("tags:\n");
            for (String tag : page.tags()) {
                sb.append("  - ").append(tag).append("\n");
            }
        }
        sb.append("created: ").append(page.createdDate()).append("\n");
        sb.append("modified: ").append(page.modifiedDate()).append("\n");
        sb.append("---\n\n");

        // Blocks separated by blank lines with pandoc attributes
        for (Block block : blocks) {
            String body = block.body();
            if (body == null) body = "";

            // Indent by level
            String indent = "  ".repeat(block.level());
            String indentedBody = body.lines()
                    .map(line -> indent + line)
                    .reduce((a, b2) -> a + "\n" + b2)
                    .orElse(indent);

            sb.append(indentedBody);

            // Pandoc-style attributes
            StringBuilder attrs = new StringBuilder();
            attrs.append("{#").append(block.id());
            if (block.collapsed()) {
                attrs.append(" .collapsed");
            }
            if (block.types() != null) {
                for (String type : block.types()) {
                    attrs.append(" .").append(type);
                }
            }
            attrs.append("}");
            sb.append(" ").append(attrs).append("\n\n");
        }

        return sb.toString();
    }

    private String yamlEscape(String value) {
        if (value.contains(":") || value.contains("\"") || value.contains("'") || value.contains("\n")) {
            return "\"" + value.replace("\"", "\\\"") + "\"";
        }
        return value;
    }

    private void writeWithLock(Path path, String content) throws IOException {
        try (FileChannel channel = FileChannel.open(path,
                StandardOpenOption.WRITE, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING)) {
            FileLock lock = channel.tryLock();
            if (lock == null) {
                throw new IOException("Cannot acquire file lock: " + path);
            }
            try {
                channel.write(java.nio.ByteBuffer.wrap(content.getBytes(java.nio.charset.StandardCharsets.UTF_8)));
            } finally {
                lock.release();
            }
        }
    }

    private String resolveHome(String path) {
        if (path.startsWith("~")) {
            return System.getProperty("user.home") + path.substring(1);
        }
        return path;
    }
}
