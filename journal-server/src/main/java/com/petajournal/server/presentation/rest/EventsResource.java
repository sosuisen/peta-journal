package com.petajournal.server.presentation.rest;

import jakarta.inject.Inject;
import jakarta.ws.rs.GET;
import jakarta.ws.rs.Path;
import jakarta.ws.rs.Produces;
import jakarta.ws.rs.core.Context;
import jakarta.ws.rs.core.MediaType;
import jakarta.ws.rs.sse.Sse;
import jakarta.ws.rs.sse.SseEventSink;

@Path("/api/events")
public class EventsResource {

    @Inject
    SseBroadcaster sseBroadcaster;

    @GET
    @Produces(MediaType.SERVER_SENT_EVENTS)
    public void stream(@Context SseEventSink sink, @Context Sse sse) {
        // Client may have already disconnected before this method runs (e.g. rapid reconnect)
        if (sink.isClosed()) return;
        try {
            // Send an initial comment to flush proxy buffers (Vite dev server buffers until first data)
            sink.send(sse.newEventBuilder().comment("connected").build());
        } catch (IllegalStateException e) {
            // Client disconnected before we could send — ignore
            return;
        }
        sseBroadcaster.register(sink, sse);
    }
}
