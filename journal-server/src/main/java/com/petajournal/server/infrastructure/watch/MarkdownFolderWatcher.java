package com.petajournal.server.infrastructure.watch;

import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.export.FileSyncStateRepository;
import com.petajournal.server.infrastructure.export.MarkdownExportService;
import com.petajournal.server.infrastructure.export.MarkdownImportService;
import com.petajournal.server.infrastructure.export.MarkdownPathMapper;
import com.petajournal.server.infrastructure.export.MarkdownReconcileService;
import com.petajournal.server.infrastructure.export.PendingDeletePrompt;
import io.quarkus.runtime.ShutdownEvent;
import io.quarkus.runtime.StartupEvent;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.enterprise.event.Observes;
import jakarta.inject.Inject;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

import java.io.IOException;
import java.nio.file.*;
import java.util.HashMap;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.TimeUnit;
import java.util.stream.Stream;

import static java.nio.file.StandardWatchEventKinds.*;

/**
 * Watches the markdown folder and feeds changed files to {@link MarkdownImportService}.
 * Events are debounced per file so a burst (Dropbox catching up, an editor saving in
 * several steps) turns into one import once the file has been quiet for a while.
 */
@ApplicationScoped
public class MarkdownFolderWatcher {

    private static final Logger LOG = Logger.getLogger(MarkdownFolderWatcher.class);

    private final boolean importEnabled;
    private final long debounceMs;
    private final MarkdownExportService exportService;
    private final MarkdownImportService importService;
    private final MarkdownReconcileService reconcileService;
    private final FileSyncStateRepository fileSyncStateRepository;
    private final PendingDeletePrompt pendingDeletePrompt;
    private final SearchService searchService;

    private final ScheduledExecutorService scheduler = Executors.newSingleThreadScheduledExecutor(r -> {
        Thread t = new Thread(r, "markdown-watcher-debounce");
        t.setDaemon(true);
        return t;
    });

    /** relPath → time of the last event (millis) */
    private final Map<String, Long> pending = new ConcurrentHashMap<>();
    /** relPath → failed attempts so far; we give up after MAX_RETRIES and leave it to the scan */
    private final Map<String, Integer> retries = new ConcurrentHashMap<>();
    private static final int MAX_RETRIES = 3;

    private volatile WatchService watchService;
    private volatile Path root;
    private volatile Thread watchThread;
    private final Map<WatchKey, Path> keys = new HashMap<>();

    @Inject
    public MarkdownFolderWatcher(
            @ConfigProperty(name = "petajournal.import.enabled", defaultValue = "true") boolean importEnabled,
            @ConfigProperty(name = "petajournal.import.debounce-ms", defaultValue = "2000") long debounceMs,
            MarkdownExportService exportService,
            MarkdownImportService importService,
            MarkdownReconcileService reconcileService,
            FileSyncStateRepository fileSyncStateRepository,
            PendingDeletePrompt pendingDeletePrompt,
            SearchService searchService) {
        this.importEnabled = importEnabled;
        this.debounceMs = debounceMs;
        this.exportService = exportService;
        this.importService = importService;
        this.reconcileService = reconcileService;
        this.fileSyncStateRepository = fileSyncStateRepository;
        this.pendingDeletePrompt = pendingDeletePrompt;
        this.searchService = searchService;
    }

    void onStart(@Observes StartupEvent event) {
        if (!importEnabled) {
            LOG.info("Folder → DB import is disabled (petajournal.import.enabled=false)");
            return;
        }
        scheduler.scheduleWithFixedDelay(this::flushPending, 500, 500, TimeUnit.MILLISECONDS);
        // Retry until the folder is configured and mounted (also covers the first-run folder chooser)
        scheduler.scheduleWithFixedDelay(this::ensureWatching, 3, 15, TimeUnit.SECONDS);
    }

    void onStop(@Observes ShutdownEvent event) {
        scheduler.shutdownNow();
        stopWatching();
    }

    /** Called when the markdown directory setting changes. */
    public synchronized void restart() {
        stopWatching();
        ensureWatching();
    }

    private synchronized void ensureWatching() {
        var rootOpt = exportService.markdownRoot();
        if (rootOpt.isEmpty()) {
            if (watchService != null) stopWatching();
            return;
        }
        Path newRoot = rootOpt.get();
        if (watchService != null && newRoot.equals(root)) return;
        if (watchService != null) stopWatching();

        try {
            watchService = newRoot.getFileSystem().newWatchService();
            root = newRoot;
            for (String top : new String[]{"docs", "journals"}) {
                Path dir = newRoot.resolve(top);
                if (Files.isDirectory(dir)) registerTree(dir);
            }
            watchThread = new Thread(this::watchLoop, "markdown-watcher");
            watchThread.setDaemon(true);
            watchThread.start();
            LOG.infov("Watching markdown folder: {0}", newRoot);
            reconcileService.requestScan();
        } catch (IOException e) {
            LOG.warnv("Cannot watch {0}: {1}", newRoot, e.getMessage());
            stopWatching();
        }
    }

    private synchronized void stopWatching() {
        WatchService ws = watchService;
        watchService = null;
        keys.clear();
        if (ws != null) {
            try {
                ws.close();
            } catch (IOException ignored) {
                // closing anyway
            }
        }
        Thread t = watchThread;
        watchThread = null;
        if (t != null) t.interrupt();
    }

    private void registerTree(Path dir) throws IOException {
        try (Stream<Path> walk = Files.walk(dir)) {
            for (Path d : walk.filter(Files::isDirectory).toList()) {
                if (MarkdownPathMapper.hasIgnoredSegment(MarkdownPathMapper.relativize(root, d))) continue;
                register(d);
            }
        }
    }

    private void register(Path dir) throws IOException {
        WatchService ws = watchService;
        if (ws == null) return;
        WatchKey key = dir.register(ws, ENTRY_CREATE, ENTRY_MODIFY, ENTRY_DELETE);
        synchronized (this) {
            keys.put(key, dir);
        }
    }

    private void watchLoop() {
        WatchService ws = watchService;
        while (ws != null && ws == watchService) {
            WatchKey key;
            try {
                key = ws.take();
            } catch (InterruptedException | ClosedWatchServiceException e) {
                return;
            }
            Path dir;
            synchronized (this) {
                dir = keys.get(key);
            }
            if (dir == null) {
                key.reset();
                continue;
            }
            for (WatchEvent<?> event : key.pollEvents()) {
                if (event.kind() == OVERFLOW) {
                    LOG.debug("Watch overflow, requesting full scan");
                    reconcileService.requestScan();
                    continue;
                }
                Path changed = dir.resolve((Path) event.context());
                handle(event.kind(), changed);
            }
            if (!key.reset()) {
                // Directory gone (deleted, or selective sync turned off): let the scan sort it out
                synchronized (this) {
                    keys.remove(key);
                }
                reconcileService.requestScan();
            }
        }
    }

    private void handle(WatchEvent.Kind<?> kind, Path path) {
        if (kind == ENTRY_CREATE && Files.isDirectory(path)) {
            try {
                registerTree(path);
            } catch (IOException e) {
                LOG.debugv("Cannot register {0}: {1}", path, e.getMessage());
            }
            reconcileService.requestScan(); // pick up files that arrived with the directory
            return;
        }
        String rel = MarkdownPathMapper.relativize(root, path);
        if (kind == ENTRY_DELETE && !rel.endsWith(MarkdownPathMapper.MD_EXT)) {
            // Could be a directory we were watching
            reconcileService.requestScan();
            return;
        }
        if (MarkdownPathMapper.isIgnored(rel)) return;
        pending.put(rel, System.currentTimeMillis());
    }

    /** Imports files that have been quiet for the debounce period. */
    private void flushPending() {
        Path currentRoot = root;
        if (currentRoot == null || pending.isEmpty()) return;
        long cutoff = System.currentTimeMillis() - debounceMs;
        int newMissing = 0;
        boolean indexChanged = false;
        for (var entry : pending.entrySet()) {
            if (entry.getValue() > cutoff) continue;
            String rel = entry.getKey();
            pending.remove(rel, entry.getValue());
            try {
                MarkdownImportService.Outcome outcome = importService.syncFile(currentRoot, rel);
                if (outcome == MarkdownImportService.Outcome.IMPORTED || outcome == MarkdownImportService.Outcome.CONFLICT) {
                    indexChanged = true;
                }
                if (outcome == MarkdownImportService.Outcome.MISSING) {
                    var state = fileSyncStateRepository.findByPath(rel);
                    if (state.isPresent() && !state.get().isMissing()) {
                        fileSyncStateRepository.markMissing(rel);
                        newMissing++;
                    }
                } else if (outcome == MarkdownImportService.Outcome.ERROR) {
                    int n = retries.merge(rel, 1, Integer::sum);
                    if (n <= MAX_RETRIES) {
                        pending.putIfAbsent(rel, System.currentTimeMillis()); // try again after the next quiet period
                    } else {
                        retries.remove(rel);
                        LOG.warnv("Giving up on {0} for now; the periodic scan will retry", rel);
                    }
                } else {
                    retries.remove(rel);
                }
            } catch (RuntimeException e) {
                LOG.warnv("Failed to sync {0}: {1}", rel, e.getMessage());
            }
        }
        if (indexChanged) searchService.flush();
        if (newMissing > 0) {
            LOG.infov("{0} markdown file(s) deleted; asking the user what to do", newMissing);
            pendingDeletePrompt.notifyNewMissing();
        }
    }
}
