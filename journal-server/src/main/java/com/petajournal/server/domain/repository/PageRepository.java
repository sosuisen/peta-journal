package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Page;

import java.time.LocalDate;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

public interface PageRepository {

    Optional<Page> findById(UUID id);

    Optional<Page> findJournal(LocalDate date);

    Optional<Page> findMostRecentJournalBefore(LocalDate beforeDate);

    List<Page> findByTag(String tag);

    List<Page> findRecent(int maxResults);

    List<Page> findAll();

    List<Page> findByTitle(String titleQuery);

    Optional<Page> findByExactTitle(String title);

    Optional<Page> findByDirPathAndFileName(String dirPath, String fileName);

    Page save(Page page);

    void delete(UUID id);

    List<Page> findBacklinkSources(UUID targetPageId);
}
