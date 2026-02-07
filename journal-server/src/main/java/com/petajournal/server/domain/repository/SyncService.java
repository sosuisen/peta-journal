package com.petajournal.server.domain.repository;

public interface SyncService {

    void notifyChange(String entityType, String entityId);

    boolean isEnabled();
}
