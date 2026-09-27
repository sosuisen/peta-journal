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
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.TreeMap;

@ApplicationScoped
public class MarkdownExportService {

    private static final Logger LOG = Logger.getLogger(MarkdownExportService.class);

    @Inject
    ConfigService configService;

    @Inject
    ExportQueueRepository exportQueueRepository;

    @Inject
    FileSyncStateRepository fileSyncStateRepository;

    @Inject
    PageRepository pageRepository;

    @Inject
    ItemRepository itemRepository;

    @Inject
    PropertyRepository propertyRepository;

    private volatile boolean isProcessing = false;
    private volatile boolean warnedDirUnavailable = false;

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

    /**
     * Returns the markdown root if it is configured and currently reachable.
     * Empty when Dropbox (or the drive) is not mounted — callers must not create it.
     */
    public Optional<Path> markdownRoot() {
        String markdownDir = configService.getMarkdownDir();
        if (markdownDir == null || markdownDir.trim().isEmpty()) return Optional.empty();
        Path root = Path.of(markdownDir);
        if (!Files.isDirectory(root)) {
            if (!warnedDirUnavailable) {
                LOG.warnv("Markdown directory is not available: {0}", root);
                warnedDirUnavailable = true;
            }
            return Optional.empty();
        }
        if (warnedDirUnavailable) {
            LOG.infov("Markdown directory is available again: {0}", root);
            warnedDirUnavailable = false;
        }
        return Optional.of(root);
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
        Optional<Path> rootOpt = markdownRoot();
        if (rootOpt.isEmpty()) {
            throw new IOException("Markdown directory not available");
        }
        Path root = rootOpt.get();

        var pageOpt = pageRepository.findById(pageId);
        if (pageOpt.isEmpty()) {
            fileSyncStateRepository.deleteByPageId(pageId);
            return;
        }

        Page page = pageOpt.get();
        List<Item> items = itemRepository.findByPageId(pageId);

        String markdown = serializeToOutlineMarkdown(page, items);
        String relPath = MarkdownPathMapper.toRelPath(page);
        String hash = MarkdownPathMapper.sha256(markdown);

        // The page was renamed or moved: remove the file at its old location
        for (FileSyncState stale : fileSyncStateRepository.findByPageId(pageId)) {
            if (!stale.relPath().equals(relPath)) {
                deleteQuietly(root.resolve(stale.relPath()));
                fileSyncStateRepository.deleteByPath(stale.relPath());
            }
        }

        Path filePath = root.resolve(relPath);
        Optional<FileSyncState> state = fileSyncStateRepository.findByPath(relPath);
        if (state.isPresent() && !state.get().isMissing()
                && hash.equals(state.get().contentHash()) && Files.exists(filePath)) {
            LOG.debugv("Page {0} unchanged on disk, skipping export", page.name());
            return;
        }

        Files.createDirectories(filePath.getParent());
        try {
            writeAtomic(filePath, markdown);
        } catch (IOException e) {
            // File locked by another process, write to alternate name
            Path altPath = filePath.resolveSibling(page.name() + ".conflict.md");
            LOG.warnv("Cannot write {0} ({1}), writing to alternate: {2}", filePath, e.getMessage(), altPath);
            Files.writeString(altPath, markdown, StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING);
            return;
        }

        BasicFileAttributes attrs = Files.readAttributes(filePath, BasicFileAttributes.class);
        fileSyncStateRepository.upsertSynced(relPath, pageId, hash, attrs.size(), attrs.lastModifiedTime().toMillis());
        LOG.debugv("Exported page {0} to {1}", page.name(), filePath);
    }

    /**
     * Removes the markdown file of a page that was deleted from the DB.
     * Safe to call when the file is already gone.
     */
    public void deletePageFile(Page page) {
        Optional<Path> root = markdownRoot();
        if (root.isPresent()) {
            deleteQuietly(root.get().resolve(MarkdownPathMapper.toRelPath(page)));
            for (FileSyncState state : fileSyncStateRepository.findByPageId(page.id())) {
                deleteQuietly(root.get().resolve(state.relPath()));
            }
        }
        fileSyncStateRepository.deleteByPageId(page.id());
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
        sb.append("<!-- idx=").append(item.idx()).append(" -->\n");

        if (item.level() > 1) {
            sb.append("<!-- level=").append(item.level()).append(" -->\n");
        }

        if (item.collapsed()) {
            sb.append("<!-- collapsed=true -->\n");
        }

        if (item.transcludeItemId() != null) {
            sb.append("<!-- transclude=").append(item.transcludeItemId()).append(" -->\n");
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

    /**
     * Writes to a temp file in the same directory and renames it over the target,
     * so Dropbox and the folder watcher never see a half-written file.
     */
    private void writeAtomic(Path path, String content) throws IOException {
        Path tmp = path.resolveSibling("." + path.getFileName() + ".tmp");
        Files.writeString(tmp, content, StandardCharsets.UTF_8,
                StandardOpenOption.CREATE, StandardOpenOption.TRUNCATE_EXISTING, StandardOpenOption.WRITE);
        try {
            Files.move(tmp, path, StandardCopyOption.REPLACE_EXISTING, StandardCopyOption.ATOMIC_MOVE);
        } catch (AtomicMoveNotSupportedException e) {
            Files.move(tmp, path, StandardCopyOption.REPLACE_EXISTING);
        } catch (IOException e) {
            deleteQuietly(tmp);
            throw e;
        }
    }

    private void deleteQuietly(Path path) {
        try {
            Files.deleteIfExists(path);
        } catch (IOException e) {
            LOG.debugv("Could not delete {0}: {1}", path, e.getMessage());
        }
    }
}
