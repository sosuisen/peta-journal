package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Block;

import java.util.List;
import java.util.Optional;
import java.util.UUID;

public interface BlockRepository {

    Optional<Block> findById(String id);

    List<Block> findByPageId(UUID pageId);

    Block save(Block block);

    void delete(String id);

    void deleteByPageId(UUID pageId);

    List<Block> findByPageIdContainingLink(String pageTitle);

    List<Block> findByStatuses(List<String> statuses);
}
