package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Page;

import java.time.LocalDate;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

/** Repository for page persistence. */
public interface PageRepository {

    /**
     * Finds a page by its UUID.
     *
     * @param id the page UUID
     * @return the page, or empty if not found
     */
    Optional<Page> findById(UUID id);

    /**
     * Finds a daily journal page for the given date.
     *
     * @param date the journal date
     * @return the journal page, or empty if not found
     */
    Optional<Page> findJournal(LocalDate date);

    /**
     * Finds the most recent daily journal page before the given date.
     *
     * @param beforeDate the exclusive upper bound date
     * @return the most recent journal page, or empty if none exist
     */
    Optional<Page> findMostRecentJournalBefore(LocalDate beforeDate);

    /**
     * Returns recently modified pages, ordered by modification date descending.
     *
     * @param maxResults maximum number of results
     * @param type       filter by page type, or {@code null} for all
     * @return the list of recent pages
     */
    List<Page> findRecent(int maxResults, String type);

    /**
     * Returns all pages.
     *
     * @return the list of all pages
     */
    List<Page> findAll();

    /**
     * Finds pages whose file name matches the given query.
     *
     * @param titleQuery the title query string
     * @return the matching pages
     */
    List<Page> findByTitle(String titleQuery);

    /**
     * Finds a page by its type, directory path, and file name.
     *
     * @param type     the page type ({@code "journal"} or {@code "doc"})
     * @param dirPath  the directory path
     * @param fileName the file name
     * @return the matching page, or empty if not found
     */
    Optional<Page> findByTypeDirPathFileName(String type, String dirPath, String fileName);

    /**
     * Saves a page (insert or update).
     *
     * @param page the page to save
     * @return the saved page
     */
    Page save(Page page);

    /**
     * Deletes a page by its UUID.
     *
     * @param id the page UUID
     */
    void delete(UUID id);

    /**
     * Finds pages that contain blocks linking to the target page.
     *
     * @param targetPageId the target page UUID
     * @return the source pages that link to the target
     */
    List<Page> findBacklinkSources(UUID targetPageId);

    /**
     * Finds pages whose full path (dir_path + file_name) starts with the query.
     *
     * @param query      the prefix to search for
     * @param maxResults maximum number of results
     * @return the matching pages
     */
    List<Page> findByPathPrefix(String query, int maxResults);

    /**
     * Finds pages whose full path (dir_path + file_name) contains the query.
     *
     * @param query      the substring to search for
     * @param maxResults maximum number of results
     * @return the matching pages
     */
    List<Page> findByPathContains(String query, int maxResults);
}
