package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.ConfigService;
import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
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
    BlockRepository blockRepository;

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
        List<Block> blocks = blockRepository.findByPageId(pageId);

        String markdown = serializeToOutlineMarkdown(page, blocks);
        String typeDir = page.type() + "s";
        String subDir = page.dirPath();
        Path dir = subDir.isEmpty() ? Path.of(markdownDir, typeDir) : Path.of(markdownDir, typeDir, subDir);
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

        LOG.debugv("Exported page {0} to {1}", page.fileName(), filePath);
    }

    String serializeToOutlineMarkdown(Page page, List<Block> blocks) {
        StringBuilder sb = new StringBuilder();

        appendFrontmatter(sb, page);

        for (int i = 0; i < blocks.size(); i++) {
            appendBlock(sb, blocks.get(i));

            // Add blank line separator between blocks (except after last block)
            if (i < blocks.size() - 1) {
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
        if (page.tags() != null && !page.tags().isEmpty()) {
            sb.append("tags:\n");
            for (String tag : page.tags()) {
                sb.append("  - ").append(tag).append("\n");
            }
        }
        sb.append("---\n\n");
    }

    private void appendBlock(StringBuilder sb, Block block) {
        String body = block.body();
        if (body == null) body = "";

        // Empty body → single space (blank line would be a block separator)
        if (body.isEmpty()) body = " ";

        String status = block.status() != null ? block.status() : "normal";

        // Prepend GFM task list prefix for todo/done
        if ("todo".equals(status)) {
            body = "- [ ] " + body;
        } else if ("done".equals(status)) {
            body = "- [x] " + body;
        }

        // Write body lines as-is (no indentation, no bullets)
        sb.append(body).append("\n");

        // Write metadata comments
        sb.append("<!-- ").append(block.id()).append(" -->\n");

        if (block.level() > 1) {
            sb.append("<!-- level=").append(block.level()).append(" -->\n");
        }

        if (block.collapsed()) {
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
