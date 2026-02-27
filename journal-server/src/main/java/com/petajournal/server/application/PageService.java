package com.petajournal.server.application;

import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.model.Block;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.BlockRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import com.petajournal.server.domain.BlockIdGenerator;
import com.petajournal.server.domain.FractionalIndexer;
import com.petajournal.server.domain.JournalFileNameMatcher;

import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;
import java.util.UUID;

@ApplicationScoped
public class PageService {

    @Inject
    PageRepository pageRepository;

    @Inject
    BlockRepository blockRepository;

    @Inject
    SearchService searchService;

    @Inject
    com.petajournal.server.infrastructure.export.MarkdownExportService exportService;

    /**
     * Retrieves a page by its ID.
     *
     * @param pageId the page UUID
     * @return the page, or empty if not found
     */
    public Optional<Page> getPage(UUID pageId) {
        return pageRepository.findById(pageId);
    }

    /**
     * Returns all blocks belonging to a page, ordered by index.
     *
     * @param pageId the page UUID
     * @return the list of blocks
     */
    public List<Block> getPageBlocks(UUID pageId) {
        return blockRepository.findByPageId(pageId);
    }

    /** Result of getting or creating a document page. */
    public record DocResult(Page page, List<Block> blocks, boolean created) {
    }

    /**
     * Gets an existing document page by title and directory, or creates a new one.
     *
     * @param title   the document title (used as file name)
     * @param dirPath the directory path (e.g. {@code "projects/"})
     * @param tags    optional tags for the page
     * @return the page with its blocks and whether it was newly created
     * @throws IllegalArgumentException if the title is invalid or uses a
     *                                  journal-type name
     */
    public DocResult getOrCreateDoc(String title, String dirPath, List<String> tags) {
        // Validate: reject empty or whitespace-only titles
        if (title == null || title.trim().isEmpty()) {
            throw new IllegalArgumentException("Page title cannot be empty or whitespace-only");
        }

        // Validate: reject titles with leading or trailing whitespace
        if (!title.equals(title.trim())) {
            throw new IllegalArgumentException("Page title cannot have leading or trailing whitespace");
        }

        String dir = (dirPath == null) ? "" : dirPath;
        String fileName = title;

        validateFileName(fileName);

        // Reject journal-type names as doc titles only when dirPath is empty
        if (dir.isEmpty() && JournalFileNameMatcher.isJournalFileName(fileName)) {
            throw new IllegalArgumentException("Cannot create a doc with a journal-type name: " + title);
        }

        // Ensure all ancestor directory pages exist
        ensureAncestorPages(dir);

        // Check for duplicate using (type, dir_path, file_name) index
        var existing = pageRepository.findByTypeDirPathFileName("doc", dir, fileName);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Block> blocks = blockRepository.findByPageId(page.id());
            return new DocResult(page, blocks, false);
        }

        // If no exact match, check if an index page exists at dir+fileName/
        // e.g. dirPath="" fileName="Quarkus" → look for dirPath="Quarkus/" fileName="index"
        String indexDir = dir + fileName + "/";
        var indexPage = pageRepository.findByTypeDirPathFileName("doc", indexDir, "index");
        if (indexPage.isPresent()) {
            Page page = indexPage.get();
            List<Block> blocks = blockRepository.findByPageId(page.id());
            return new DocResult(page, blocks, false);
        }

        UUID id = UUID.randomUUID();
        Instant now = Instant.now();

        Page page = new Page(id, dir, fileName, "doc", now, now, tags != null ? tags : List.of());
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create initial empty block
        UUID blockId = BlockIdGenerator.generate();
        Block block = new Block(blockId, id, page.dirPath(), page.fileName(), page.type(), now, now,
                FractionalIndexer.initial(), 1, "", "", false, "normal", List.of());
        blockRepository.save(block);

        // Queue for export
        exportService.queueExport(id);

        return new DocResult(page, List.of(block), true);
    }

    /**
     * Updates a page's metadata. Only non-null parameters are applied.
     *
     * @param pageId         the page to update
     * @param title          new title, or {@code null} to keep current
     * @param tags           new tags, or {@code null} to keep current
     * @param dirPath        new directory path, or {@code null} to keep current
     * @return the updated page
     * @throws IllegalArgumentException if the page is not found or the title is
     *                                  invalid
     */
    public Page updatePage(UUID pageId, String title, List<String> tags, String dirPath) {
        Page existing = pageRepository.findById(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        String fileName = existing.fileName();
        if (title != null) {
            if (title.trim().isEmpty()) {
                throw new IllegalArgumentException("Page title cannot be empty or whitespace-only");
            }
            if (!title.equals(title.trim())) {
                throw new IllegalArgumentException("Page title cannot have leading or trailing whitespace");
            }
            validateFileName(title);
            fileName = title;
        }

        String newDirPath = existing.dirPath();
        if (dirPath != null) {
            newDirPath = dirPath;
        }

        Page updated = new Page(
                existing.id(),
                newDirPath,
                fileName,
                existing.type(),
                existing.createdDate(),
                Instant.now(),
                tags != null ? tags : existing.tags());

        updated = pageRepository.save(updated);
        searchService.indexPage(updated);

        // Queue for export
        exportService.queueExport(pageId);

        return updated;
    }

    /**
     * Deletes a page and all its blocks.
     *
     * @param pageId the page to delete
     */
    public void deletePage(UUID pageId) {
        searchService.removePage(pageId.toString());
        blockRepository.deleteByPageId(pageId);
        pageRepository.delete(pageId);
    }

    /**
     * Returns recently modified pages, ordered by modification date (newest first).
     *
     * @param maxResults maximum number of results
     * @param type       filter by page type ({@code "journal"}, {@code "doc"}), or
     *                   {@code null} for all
     * @return the list of recent pages
     */
    public List<Page> getRecentPages(int maxResults, String type) {
        return pageRepository.findRecent(maxResults, type);
    }

    /**
     * Finds all pages and blocks that link to the given page (backlinks).
     *
     * @param pageId the target page UUID
     * @return the list of backlink entries, each containing the source page and
     *         linking block
     */
    public List<BacklinkEntry> getBacklinks(UUID pageId) {
        List<Page> sources = pageRepository.findBacklinkSources(pageId);
        List<BacklinkEntry> entries = new ArrayList<>();

        Page targetPage = pageRepository.findById(pageId).orElse(null);
        if (targetPage == null)
            return entries;

        for (Page source : sources) {
            // Find the specific blocks that link to this page (using fileName as title)
            List<Block> linkingBlocks = blockRepository.findByPageIdContainingLink(targetPage.fileName());
            for (Block block : linkingBlocks) {
                if (block.pageId().equals(source.id())) {
                    entries.add(new BacklinkEntry(source, block));
                }
            }
        }
        return entries;
    }

    /**
     * Ensures that all ancestor directory pages exist for the given dirPath.
     * For example, dirPath "dir1/dir2/" creates pages ("dir1/", "index") and ("dir1/dir2/", "index").
     */
    private void ensureAncestorPages(String dirPath) {
        if (dirPath == null || dirPath.isEmpty()) return;

        // Split "dir1/dir2/" into ["dir1", "dir2"]
        String[] segments = dirPath.split("/");
        StringBuilder currentDir = new StringBuilder();

        for (String segment : segments) {
            if (segment.isEmpty()) continue;

            currentDir.append(segment).append("/");
            String indexDir = currentDir.toString();
            var existing = pageRepository.findByTypeDirPathFileName("doc", indexDir, "index");
            if (existing.isEmpty()) {
                UUID id = UUID.randomUUID();
                Instant now = Instant.now();
                Page page = new Page(id, indexDir, "index", "doc", now, now, List.of());
                pageRepository.save(page);
                searchService.indexPage(page);

                UUID blockId = BlockIdGenerator.generate();
                Block block = new Block(blockId, id, indexDir, "index", "doc", now, now,
                        FractionalIndexer.initial(), 1, "", "", false, "normal", List.of());
                blockRepository.save(block);
            }
        }
    }

    private static final java.util.regex.Pattern FORBIDDEN_CHARS = java.util.regex.Pattern.compile(
            "[<>:\"/\\\\|?*\\x00-\\x1F\\x7F]");
    private static final java.util.regex.Pattern RESERVED_NAMES = java.util.regex.Pattern.compile(
            "^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$", java.util.regex.Pattern.CASE_INSENSITIVE);

    private static void validateFileName(String title) {
        var matcher = FORBIDDEN_CHARS.matcher(title);
        if (matcher.find()) {
            String ch = matcher.group();
            if (ch.charAt(0) < 0x20 || ch.charAt(0) == 0x7F) {
                throw new IllegalArgumentException("Title contains a control character (not allowed in filenames)");
            }
            throw new IllegalArgumentException("Title contains '" + ch + "' (not allowed in filenames)");
        }
        if (title.endsWith(".") || title.endsWith(" ")) {
            throw new IllegalArgumentException("Title cannot end with a dot or space");
        }
        if (RESERVED_NAMES.matcher(title).matches()) {
            throw new IllegalArgumentException("Title '" + title + "' is a reserved name on Windows");
        }
    }
}
