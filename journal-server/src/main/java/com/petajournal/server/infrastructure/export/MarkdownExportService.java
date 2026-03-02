package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.ConfigService;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import io.quarkus.scheduler.Scheduled;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
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

    @Inject
    ConfigService configService;

    @Inject
    ExportQueueRepository exportQueueRepository;

    @Inject
    PageRepository pageRepository;

    @Inject
    ItemRepository itemRepository;

    private volatile boolean isProcessing = false;

    public void queueExport(UUID pageId) {
        exportQueueRepository.enqueue(pageId);
    }

    @Scheduled(every = "${petajournal.export.interval}s", delay = 5, delayUnit = java.util.concurrent.TimeUnit.SECONDS)
    void processExportQueue() {
        // Skip if already processing to prevent concurrent execution
        if (isProcessing) {
            LOG.debug("Export queue processing already in progress, skipping this run");
            return;
        }

        isProcessing = true;
        try {
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
        } finally {
            isProcessing = false;
        }
    }

    void exportPage(UUID pageId) throws IOException {
        String markdownDir = configService.getMarkdownDir();
        if (markdownDir == null || markdownDir.trim().isEmpty()) {
            LOG.warn("Markdown directory not configured, skipping export");
            return;
        }

        var pageOpt = pageRepository.findById(pageId);
        if (pageOpt.isEmpty()) return;

        Page page = pageOpt.get();
        List<Item> items = itemRepository.findByPageId(pageId);

        String markdown = serializeToOutlineMarkdown(page, items);
        String typeDir = page.type() + "s";
        String subDir = page.dirPath();
        Path dir = subDir.isEmpty() ? Path.of(markdownDir, typeDir) : Path.of(markdownDir, typeDir, subDir);
        Files.createDirectories(dir);
        Path filePath = dir.resolve(page.name() + ".md");

        try {
            writeWithLock(filePath, markdown);
        } catch (IOException e) {
            // File locked, write to alternate name
            Path altPath = dir.resolve(page.name() + ".conflict.md");
            LOG.warnv("File locked, writing to alternate: {0}", altPath);
            Files.writeString(altPath, markdown, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING);
        }

        LOG.debugv("Exported page {0} to {1}", page.name(), filePath);
    }

    String serializeToOutlineMarkdown(Page page, List<Item> items) {
        StringBuilder sb = new StringBuilder();

        appendFrontmatter(sb, page);

        for (int i = 0; i < items.size(); i++) {
            appendItem(sb, items.get(i));

            // Add blank line separator between items (except after last item)
            if (i < items.size() - 1) {
                sb.append("\n");
            }
        }

        return sb.toString();
    }

    private void appendFrontmatter(StringBuilder sb, Page page) {
        sb.append("---\n");
        sb.append("id: ").append(page.id()).append("\n");
        sb.append("created: ").append(page.createdDate()).append("\n");
        sb.append("modified: ").append(page.modifiedDate()).append("\n");
        sb.append("---\n\n");
    }

    private void appendItem(StringBuilder sb, Item item) {
        String body = item.body();
        if (body == null) body = "";

        // Empty body → single space (blank line would be an item separator)
        if (body.isEmpty()) body = " ";

        String status = item.status() != null ? item.status() : "normal";

        // Prepend GFM task list prefix for todo/done
        if ("todo".equals(status)) {
            body = "- [ ] " + body;
        } else if ("done".equals(status)) {
            body = "- [x] " + body;
        }

        // Write body lines as-is (no indentation, no bullets)
        sb.append(body).append("\n");

        // Write metadata comments
        sb.append("<!-- ").append(item.id()).append(" -->\n");

        if (item.level() > 1) {
            sb.append("<!-- level=").append(item.level()).append(" -->\n");
        }

        if (item.collapsed()) {
            sb.append("<!-- collapsed=true -->\n");
        }

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
}
