package com.petajournal.server.infrastructure.export;

import com.petajournal.server.application.ConfigService;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import io.quarkus.scheduler.Scheduled;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import com.sosuisha.hmtid.HMTID;
import java.io.IOException;
import java.nio.channels.FileChannel;
import java.nio.channels.FileLock;
import java.nio.file.*;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

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

    @Inject
    PropertyRepository propertyRepository;

    private volatile boolean isProcessing = false;

    public void queueExport(HMTID pageId) {
        exportQueueRepository.enqueue(pageId);
    }

    public void exportAllIfEmpty(String dir) {
        if (dir == null || dir.trim().isEmpty()) return;
        Path path = Path.of(dir);
        if (!Files.isDirectory(path)) return;
        try (var stream = Files.walk(path)) {
            boolean hasFiles = stream.anyMatch(p -> Files.isRegularFile(p) && p.toString().endsWith(".md"));
            if (hasFiles) return;
        } catch (IOException e) {
            LOG.warnv("Failed to scan directory {0}: {1}", dir, e.getMessage());
            return;
        }
        List<Page> allPages = pageRepository.findAll();
        for (Page page : allPages) {
            exportQueueRepository.enqueue(page.id());
        }
        LOG.infov("Directory empty, queued {0} pages for export", allPages.size());
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
            List<HMTID> pageIds = exportQueueRepository.dequeueAll();
            for (HMTID pageId : pageIds) {
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

    void exportPage(HMTID pageId) throws IOException {
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

        // Batch-load all properties for items on this page (avoids N+1)
        List<String> itemIds = items.stream().map(i -> i.id().toString()).toList();
        Map<String, Map<String, String>> scalarProps = propertyRepository.findScalarByTargets("item", itemIds);
        Map<String, Map<String, List<String>>> listProps = propertyRepository.findListByTargets("item", itemIds);

        for (int i = 0; i < items.size(); i++) {
            String id = items.get(i).id().toString();
            Map<String, String> scalars = scalarProps.getOrDefault(id, Map.of());
            Map<String, List<String>> lists = listProps.getOrDefault(id, Map.of());
            appendItem(sb, items.get(i), scalars, lists);

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

    private void appendItem(StringBuilder sb, Item item, Map<String, String> scalars, Map<String, List<String>> lists) {
        String body = item.body();
        if (body == null) body = "";

        // Empty body → single space (blank line would be an item separator)
        if (body.isEmpty()) body = " ";

        // Prepend GFM task list prefix for task type
        if (item.types() != null && item.types().contains("task")) {
            if (scalars.containsKey("end")) {
                body = "- [x] " + body;
            } else {
                body = "- [ ] " + body;
            }
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

        // Write property comments (alphabetically sorted by key)
        TreeMap<String, String> allProps = new TreeMap<>();
        allProps.putAll(scalars);
        for (var entry : lists.entrySet()) {
            allProps.put(entry.getKey(), String.join(",", entry.getValue()));
        }
        for (var entry : allProps.entrySet()) {
            sb.append("<!-- ").append(entry.getKey()).append("=").append(entry.getValue()).append(" -->\n");
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
