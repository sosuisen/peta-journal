package com.petajournal.server.infrastructure.sync;

import com.petajournal.server.domain.repository.IndexService;
import jakarta.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class NoOpIndexService implements IndexService {

    @Override
    public boolean isEnabled() {
        return false;
    }

    @Override
    public void rebuildIndex() {
        // No-op for MVP
    }
}
