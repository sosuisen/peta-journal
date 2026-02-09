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
        Path dir = Path.of(markdownDir, page.dirPath());
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

        // YAML frontmatter
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

        // Blocks separated by empty lines
        // Level 0 (depth 1): no prefix (or " [ ] " / " [x] " for todo/done)
        // Level 1 (depth 2): "- " (or "- [ ] " / "- [x] " for todo/done)
        // Level 2 (depth 3): "  - " (2 spaces + "- ")
        // Level n (depth n+1): repeat("  ", n-1) + "- "

        for (int i = 0; i < blocks.size(); i++) {
            Block block = blocks.get(i);
            String body = block.body();
            if (body == null) body = "";

            // Add prefix based on level and status
            String prefix = "";
            String status = block.status() != null ? block.status() : "normal";

            if (block.level() == 0) {
                // Level 0: add checkbox only for todo/done
                if ("todo".equals(status)) {
                    prefix = "[ ] ";
                } else if ("done".equals(status)) {
                    prefix = "[x] ";
                }
            } else {
                // Level 1+: always add bullet, with checkbox for todo/done
                String indent = "  ".repeat(block.level() - 1);
                if ("todo".equals(status)) {
                    prefix = indent + "- [ ] ";
                } else if ("done".equals(status)) {
                    prefix = indent + "- [x] ";
                } else {
                    prefix = indent + "- ";
                }
            }

            // First line gets the prefix, subsequent lines don't need indenting
            String[] lines = body.split("\n", -1);
            for (int j = 0; j < lines.length; j++) {
                if (j == 0) {
                    sb.append(prefix).append(lines[j]);
                } else {
                    sb.append(lines[j]);
                }

                // Add block ID at the end of the last line
                if (j == lines.length - 1) {
                    sb.append(" {#").append(block.id()).append("}");
                }
                sb.append("\n");
            }

            // Add blank line separator between blocks (except after last block)
            if (i < blocks.size() - 1) {
                sb.append("\n");
            }
        }

        return sb.toString();
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
