package com.petajournal.server.infrastructure.export;

import com.petajournal.server.infrastructure.launcher.AppLauncher;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.eclipse.microprofile.config.inject.ConfigProperty;
import org.jboss.logging.Logger;

/**
 * Opens the "files disappeared" confirmation page in the user's browser.
 * Nothing is deleted from the DB until the user ticks a page there and submits.
 */
@ApplicationScoped
public class PendingDeletePrompt {

    private static final Logger LOG = Logger.getLogger(PendingDeletePrompt.class);

    private final int httpPort;
    private final long cooldownSeconds;
    private final AppLauncher appLauncher;

    private volatile long lastOpenedAt = 0;

    @Inject
    public PendingDeletePrompt(
            @ConfigProperty(name = "quarkus.http.port") int httpPort,
            @ConfigProperty(name = "petajournal.import.delete-prompt-cooldown", defaultValue = "300") long cooldownSeconds,
            AppLauncher appLauncher) {
        this.httpPort = httpPort;
        this.cooldownSeconds = cooldownSeconds;
        this.appLauncher = appLauncher;
    }

    public String url() {
        return "http://localhost:" + httpPort + "/api/sync/pending-deletes";
    }

    /**
     * Called whenever missing files are pending (new ones from the watcher, or any left undecided
     * on each periodic scan). Opens the browser unless it was opened within the cooldown.
     */
    public synchronized void notifyNewMissing() {
        long now = System.currentTimeMillis();
        if (now - lastOpenedAt < cooldownSeconds * 1000) {
            LOG.debug("Pending-deletes page opened recently; not opening again yet");
            return;
        }
        lastOpenedAt = now;
        if (!appLauncher.launch(url())) {
            LOG.warnv("Could not open a browser. Review missing files at {0}", url());
        }
    }
}
