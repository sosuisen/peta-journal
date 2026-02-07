package com.petajournal.server.infrastructure.sync;

import com.petajournal.server.domain.repository.PluginService;
import jakarta.enterprise.context.ApplicationScoped;

@ApplicationScoped
public class NoOpPluginService implements PluginService {

    @Override
    public boolean isEnabled() {
        return false;
    }
}
