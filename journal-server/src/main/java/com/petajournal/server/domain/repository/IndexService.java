package com.petajournal.server.domain.repository;

public interface IndexService {

    boolean isEnabled();

    void rebuildIndex();
}
