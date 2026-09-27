package com.petajournal.server.infrastructure.export;

import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.PageRepository;
import com.sosuisha.hmtid.HMTID;
import io.quarkus.runtime.ShutdownEvent;
import io.quarkus.scheduler.Scheduled;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.attribute.BasicFileAttributes;
import java.util.*;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.stream.Stream;

/**
 * Full scan of the markdown folder against {@code file_sync_state}.
 * Runs shortly after startup and then periodically, as a safety net for file
 * events the watcher never sees (network drives, Dropbox catching up while the
 * server was down, macOS polling).
 */
@ApplicationScoped
public class MarkdownReconcileService {

    private static final Logger LOG = Logger.getLogger(MarkdownReconcileService.class);
    private static final List<String> TOP_DIRS = List.of("docs", "journals");

    @ConfigProperty(name = "petajournal.import.enabled", defaultValue = "true")
    boolean importEnabled;

    @Inject MarkdownExportService exportService;
    @Inject MarkdownImportService importService;
    @Inject FileSyncStateRepository fileSyncStateRepository;
    @Inject PageRepository pageRepository;
    @Inject PendingDeletePrompt pendingDeletePrompt;
    @Inject com.petajournal.server.domain.repository.SearchService searchService;

    private final AtomicBoolean scanning = new AtomicBoolean(false);
    private volatile boolean scanRequested = false;

    // A full scan can take minutes on a big folder; it must never run on a Vert.x worker thread
    private final ExecutorService scanExecutor = Executors.newSingleThreadExecutor(r -> {
        Thread t = new Thread(r, "markdown-scan");
        t.setDaemon(true);
        return t;
    });

    void onStop(@Observes ShutdownEvent event) {
        scanExecutor.shutdownNow();
    }

    /** Ask for a scan on the next scheduler tick (used by the watcher for directory-level events). */
    public void requestScan() {
        scanRequested = true;
    }

    @Scheduled(every = "${petajournal.import.scan-interval}s", delay = 10, delayUnit = TimeUnit.SECONDS)
    void scheduledScan() {
        scanAsync();
    }

    @Scheduled(every = "2s")
    void requestedScan() {
        if (scanRequested) {
            scanRequested = false;
            scanAsync();
        }
    }

    /** Runs a scan on the scan thread unless one is already running or queued. */
    public void scanAsync() {
        if (!importEnabled) return;
        if (!scanning.compareAndSet(false, true)) return;
        scanExecutor.submit(() -> {
            try {
                doScan();
            } catch (Throwable e) {
                // submit() would otherwise swallow this into the Future; Errors included
                LOG.error("Folder scan failed", e);
            } finally {
                scanning.set(false);
            }
        });
    }

    public record ScanSummary(int imported, int conflicts, int newMissing, int exported) {}

    /** Runs a scan on the calling thread. Prefer {@link #scanAsync()} from scheduled or request threads. */
    public ScanSummary scan() {
        if (!importEnabled) return new ScanSummary(0, 0, 0, 0);
        if (!scanning.compareAndSet(false, true)) return new ScanSummary(0, 0, 0, 0);
        try {
            return doScan();
        } catch (Throwable e) {
            LOG.error("Folder scan failed", e);
            return new ScanSummary(0, 0, 0, 0);
        } finally {
            scanning.set(false);
        }
    }

    private ScanSummary doScan() {
        long startedAt = System.currentTimeMillis();
        Optional<Path> rootOpt = exportService.markdownRoot();
        if (rootOpt.isEmpty()) return new ScanSummary(0, 0, 0, 0);
        Path root = rootOpt.get();

        // If neither top folder is there, the folder is not really available (unmounted, selective sync off)
        boolean anyTop = TOP_DIRS.stream().anyMatch(d -> Files.isDirectory(root.resolve(d)));
        if (!anyTop) {
            LOG.warnv("Neither docs/ nor journals/ found under {0}; skipping scan", root);
            return new ScanSummary(0, 0, 0, 0);
        }

        Map<String, FileSyncState> states = new HashMap<>();
        Set<HMTID> pagesWithState = new HashSet<>();
        for (FileSyncState s : fileSyncStateRepository.findAll()) {
            states.put(s.relPath(), s);
            pagesWithState.add(s.pageId());
        }

        int imported = 0, conflicts = 0, newMissing = 0, exported = 0;

        for (String top : TOP_DIRS) {
            Path dir = root.resolve(top);
            if (!Files.isDirectory(dir)) continue;
            List<Path> files;
            try (Stream<Path> walk = Files.walk(dir)) {
                files = walk.filter(Files::isRegularFile).toList();
            } catch (IOException e) {
                LOG.warnv("Cannot walk {0}: {1}", dir, e.getMessage());
                continue;
            }
            for (Path file : files) {
                String rel = MarkdownPathMapper.relativize(root, file);
                if (MarkdownPathMapper.isIgnored(rel)) continue;
                FileSyncState state = states.remove(rel);
                if (state != null && !state.isMissing() && unchangedOnDisk(file, state)) continue;
                long t0 = System.currentTimeMillis();
                MarkdownImportService.Outcome outcome = importService.syncFile(root, rel);
                long ms = System.currentTimeMillis() - t0;
                if (ms > 5000) LOG.warnv("Syncing {0} took {1} ms", rel, ms);
                switch (outcome) {
                    case IMPORTED -> imported++;
                    case CONFLICT -> conflicts++;
                    default -> { }
                }
            }
        }
        if (imported + conflicts > 0) searchService.flush();

        // Files we knew about that are no longer there
        for (FileSyncState state : states.values()) {
            if (state.isMissing()) continue;
            if (Files.exists(root.resolve(state.relPath()))) continue; // exists but ignored (e.g. renamed to .conflict)
            fileSyncStateRepository.markMissing(state.relPath());
            newMissing++;
        }

        // Pages that never got a file (fresh install, or created while the folder was unavailable)
        for (Page page : pageRepository.findAll()) {
            if (pagesWithState.contains(page.id())) continue;
            if (Files.exists(root.resolve(MarkdownPathMapper.toRelPath(page)))) continue;
            exportService.queueExport(page.id());
            exported++;
        }

        // Keep asking while anything is left undecided; the prompt's cooldown limits how often
        int pendingMissing = fileSyncStateRepository.findMissing().size();
        if (pendingMissing > 0) {
            LOG.infov("{0} markdown file(s) missing ({1} new); asking the user what to do", pendingMissing, newMissing);
            pendingDeletePrompt.notifyNewMissing();
        }
        LOG.infov("Folder scan finished in {0} ms: {1} imported, {2} conflicts, {3} missing, {4} queued for export",
                System.currentTimeMillis() - startedAt, imported, conflicts, newMissing, exported);
        return new ScanSummary(imported, conflicts, newMissing, exported);
    }

    private static boolean unchangedOnDisk(Path file, FileSyncState state) {
        try {
            BasicFileAttributes attrs = Files.readAttributes(file, BasicFileAttributes.class);
            return attrs.size() == state.fileSize() && attrs.lastModifiedTime().toMillis() == state.fileMtime();
        } catch (IOException e) {
            return false;
        }
    }
}
