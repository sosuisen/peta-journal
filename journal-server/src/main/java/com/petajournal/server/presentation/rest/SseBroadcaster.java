package com.petajournal.server.presentation.rest;

import io.smallrye.mutiny.Multi;
import io.smallrye.mutiny.subscription.MultiEmitter;
import jakarta.enterprise.context.ApplicationScoped;
import org.jboss.logging.Logger;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

@ApplicationScoped
public class SseBroadcaster {

    private static final Logger LOG = Logger.getLogger(SseBroadcaster.class);

    private final List<MultiEmitter<? super String>> emitters = new CopyOnWriteArrayList<>();

    /**
     * Returns a reactive Multi for SSE subscribers.
     * Sends an initial "connected" event, then broadcasts.
     * Non-blocking — does not hold a worker thread.
     */
    public Multi<String> getEventStream() {
        Multi<String> events = Multi.createFrom().<String>emitter(emitter -> {
            emitters.add(emitter);
            emitter.onTermination(() -> emitters.remove(emitter));
        });
        return Multi.createBy().concatenating().streams(
                Multi.createFrom().item("{\"type\":\"connected\"}"),
                events
        );
    }

    public void broadcastPagesUpdated(String pageId) {
        broadcastPagesUpdated(pageId, "app");
    }

    /**
     * @param source {@code "app"} for changes made through the API,
     *               {@code "file"} for changes picked up from the markdown folder
     */
    public void broadcastPagesUpdated(String pageId, String source) {
        String json = "{\"type\":\"pagesUpdated\",\"pageId\":\"" + pageId + "\",\"source\":\"" + source + "\"}";
        for (var emitter : emitters) {
            try {
                emitter.emit(json);
            } catch (Exception e) {
                LOG.debugv("Failed to broadcast SSE event: {0}", e.getMessage());
            }
        }
    }
}
