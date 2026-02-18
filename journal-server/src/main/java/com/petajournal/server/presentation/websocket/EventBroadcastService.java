package com.petajournal.server.presentation.websocket;

import com.petajournal.proto.*;
import io.quarkus.websockets.next.OpenConnections;
import io.quarkus.websockets.next.WebSocketConnection;
import io.vertx.core.buffer.Buffer;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

@ApplicationScoped
public class EventBroadcastService {

    private static final Logger LOG = Logger.getLogger(EventBroadcastService.class);

    @Inject
    OpenConnections openConnections;

    /**
     * Sends a connected event to a newly opened WebSocket connection.
     *
     * @param connection the WebSocket connection
     */
    public void sendConnectedEvent(WebSocketConnection connection) {
        ServerEvent event = ServerEvent.newBuilder()
                .setConnected(ConnectedEvent.newBuilder()
                        .setSessionId(connection.id())
                        .build())
                .build();
        ApiResponse response = ApiResponse.newBuilder()
                .setSuccess(true)
                .setServerEvent(event)
                .build();
        connection.sendBinaryAndAwait(Buffer.buffer(response.toByteArray()));
    }

    /**
     * Broadcasts a pages-updated event to all connected clients.
     *
     * @param pageId                  the ID of the updated page
     * @param originatorConnectionId  the connection that triggered the update
     */
    public void broadcastPagesUpdated(String pageId, String originatorConnectionId) {
        ServerEvent event = ServerEvent.newBuilder()
                .setOriginatorSessionId(originatorConnectionId)
                .setPagesUpdated(PagesUpdatedEvent.newBuilder()
                        .setPageId(pageId)
                        .build())
                .build();
        broadcastToAll(event);
    }

    private void broadcastToAll(ServerEvent event) {
        ApiResponse response = ApiResponse.newBuilder()
                .setSuccess(true)
                .setServerEvent(event)
                .build();
        byte[] bytes = response.toByteArray();
        for (WebSocketConnection conn : openConnections) {
            try {
                conn.sendBinaryAndAwait(Buffer.buffer(bytes));
            } catch (Exception e) {
                LOG.warnv("Failed to send event to connection {0}: {1}", conn.id(), e.getMessage());
            }
        }
    }
}
