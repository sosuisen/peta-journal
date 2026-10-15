package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Page;
import com.sosuisha.hmtid.HMTID;

import java.time.LocalDate;
import java.util.List;
import java.util.Optional;

/** Repository for page persistence. */
public interface PageRepository {

    /**
     * Finds a page by its HMTID.
     *
     * @param id the page HMTID
     * @return the page, or empty if not found
     */
    Optional<Page> findById(HMTID id);

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
     * Finds the most recent weekly journal page before the given week.
     *
     * @param beforeWeek the exclusive upper bound week name (e.g., "2026-W09")
     * @return the most recent weekly journal page, or empty if none exist
     */
    Optional<Page> findMostRecentWeeklyJournalBefore(String beforeWeek);

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
     * Changes a page's id, moving its items and related rows along.
     * Used when a markdown file for the same path carries a different (older) id
     * from another device, so both devices end up with the same page id.
     *
     * @param oldId the current page HMTID
     * @param newId the HMTID to use from now on
     */
    void rekey(HMTID oldId, HMTID newId);

    /**
     * Deletes a page by its HMTID.
     *
     * @param id the page HMTID
     */
    void delete(HMTID id);

    /**
     * Finds pages that contain items linking to the target page.
     *
     * @param targetPageId the target page HMTID
     * @return the source pages that link to the target
     */
    List<Page> findBacklinkSources(HMTID targetPageId);

    /**
     * Finds pages whose full path (dir_path + name) starts with the query.
     *
     * @param query      the prefix to search for
     * @param maxResults maximum number of results
     * @return the matching pages
     */
    List<Page> findByPathPrefix(String query, int maxResults);

    /**
     * Finds pages whose full path (dir_path + name) contains the query.
     *
     * @param query      the substring to search for
     * @param maxResults maximum number of results
     * @return the matching pages
     */
    List<Page> findByPathContains(String query, int maxResults);

    /**
     * Finds index pages whose last dir_path segment matches the query.
     * For example, query "dir2" matches dir_path "dir1/dir2/" with name "index".
     *
     * @param query      the segment name to match
     * @param maxResults maximum number of results
     * @return the matching index pages
     */
    List<Page> findIndexPagesByDirSegment(String query, int maxResults);

    /**
     * Finds pages whose parent_id matches the given page ID, sorted by name.
     *
     * @param parentId the parent page HMTID
     * @return the child pages sorted by name
     */
    List<Page> findByParentId(HMTID parentId);
}
