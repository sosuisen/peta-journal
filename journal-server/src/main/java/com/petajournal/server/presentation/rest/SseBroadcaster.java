package com.petajournal.server.presentation.rest;

import jakarta.enterprise.context.ApplicationScoped;
import jakarta.ws.rs.sse.OutboundSseEvent;
import jakarta.ws.rs.sse.Sse;
import jakarta.ws.rs.sse.SseEventSink;
import org.jboss.logging.Logger;

import java.util.List;
import java.util.concurrent.CopyOnWriteArrayList;

@ApplicationScoped
public class SseBroadcaster {

    private static final Logger LOG = Logger.getLogger(SseBroadcaster.class);

    private final List<SseClient> clients = new CopyOnWriteArrayList<>();

    record SseClient(SseEventSink sink, Sse sse) {}

    public void register(SseEventSink sink, Sse sse) {
        clients.add(new SseClient(sink, sse));
        clients.removeIf(c -> c.sink().isClosed());
    }

    public void broadcast(String eventType, String data) {
        clients.removeIf(c -> c.sink().isClosed());
        for (var client : clients) {
            try {
                OutboundSseEvent event = client.sse().newEventBuilder()
                        .name(eventType)
                        .data(data)
                        .mediaType(jakarta.ws.rs.core.MediaType.APPLICATION_JSON_TYPE)
                        .build();
                client.sink().send(event);
            } catch (Exception e) {
                LOG.debugv("Failed to send SSE event: {0}", e.getMessage());
            }
        }
    }

    public void broadcastPagesUpdated(String pageId) {
        broadcast("pagesUpdated", "{\"pageId\":\"" + pageId + "\"}");
    }
}
