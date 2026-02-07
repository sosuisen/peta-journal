package com.petajournal.server.infrastructure.sync;

import com.petajournal.server.domain.repository.SyncService;
import jakarta.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class NoOpSyncService implements SyncService {

    @Override
    public void notifyChange(String entityType, String entityId) {
        // No-op for MVP
    }

    @Override
    public boolean isEnabled() {
        return false;
    }
}
