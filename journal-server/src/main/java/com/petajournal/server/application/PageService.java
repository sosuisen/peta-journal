package com.petajournal.server.application;

import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import com.petajournal.server.infrastructure.export.ExportQueueRepository;
import com.petajournal.server.infrastructure.export.MarkdownExportService;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import com.petajournal.server.domain.IdGenerator;
import com.petajournal.server.domain.FractionalIndexer;
import com.petajournal.server.domain.JournalFileNameMatcher;
import com.sosuisha.hmtid.HMTID;

import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

@ApplicationScoped
public class PageService {

    @Inject
    PageRepository pageRepository;

    @Inject
    ItemRepository itemRepository;

    @Inject
    SearchService searchService;

    @Inject
    MarkdownExportService exportService;

    @Inject
    ExportQueueRepository exportQueueRepository;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    IdGenerator idGenerator;

    /**
     * Retrieves a page by its ID.
     *
     * @param pageId the page HMTID
     * @return the page, or empty if not found
     */
    public Optional<Page> getPage(HMTID pageId) {
        return pageRepository.findById(pageId);
    }

    /**
     * Returns all items belonging to a page, ordered by index.
     *
     * @param pageId the page HMTID
     * @return the list of items
     */
    public List<Item> getPageItems(HMTID pageId) {
        return itemRepository.findByPageId(pageId);
    }

    /** Result of getting or creating a document page. */
    public record DocResult(Page page, List<Item> items, boolean created) {
    }

    /**
     * Gets an existing document page by title and directory, or creates a new one.
     *
     * @param title   the document title (used as file name)
     * @param dirPath the directory path (e.g. {@code "projects/"})
     * @return the page with its items and whether it was newly created
     * @throws IllegalArgumentException if the title is invalid or uses a
     *                                  journal-type name
     */
    public DocResult getOrCreateDoc(String title, String dirPath) {
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

        // Check for duplicate using (type, dir_path, name) index
        var existing = pageRepository.findByTypeDirPathFileName("doc", dir, fileName);
        if (existing.isPresent()) {
            Page page = existing.get();
            List<Item> items = itemRepository.findByPageId(page.id());
            return new DocResult(page, items, false);
        }

        // If no exact match, check if an index page exists at dir+fileName/
        // e.g. dirPath="" fileName="Quarkus" → look for dirPath="Quarkus/" fileName="index"
        String indexDir = dir + fileName + "/";
        var indexPage = pageRepository.findByTypeDirPathFileName("doc", indexDir, "index");
        if (indexPage.isPresent()) {
            Page page = indexPage.get();
            List<Item> items = itemRepository.findByPageId(page.id());
            return new DocResult(page, items, false);
        }

        HMTID id = idGenerator.generatePageId();
        Instant now = Instant.now();

        Page page = new Page(id, null, dir, fileName, "doc", now, now);
        page = pageRepository.save(page);
        searchService.indexPage(page);

        // Create initial empty item
        HMTID itemId = idGenerator.generateItemId();
        Item item = new Item(itemId, id, null, page.dirPath(), page.name(), page.type(), now, now,
                FractionalIndexer.initial(), 1, "", "", false, List.of(), null);
        itemRepository.save(item);

        // Queue for export
        exportService.queueExport(id);

        return new DocResult(page, List.of(item), true);
    }

    /**
     * Updates a page's metadata. Only non-null parameters are applied.
     *
     * @param pageId         the page to update
     * @param title          new title, or {@code null} to keep current
     * @param dirPath        new directory path, or {@code null} to keep current
     * @return the updated page
     * @throws IllegalArgumentException if the page is not found or the title is
     *                                  invalid
     */
    public Page updatePage(HMTID pageId, String title, String dirPath) {
        Page existing = pageRepository.findById(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        String fileName = existing.name();
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

        // Prevent demoting an index page that still has children
        if ("index".equals(existing.name()) && !"index".equals(fileName)) {
            List<Page> children = pageRepository.findByParentId(pageId);
            if (!children.isEmpty()) {
                throw new IllegalArgumentException(
                        "Cannot rename index page with " + children.size() + " child page(s). "
                        + "Delete or move child pages first.");
            }
        }

        String newDirPath = existing.dirPath();
        if (dirPath != null) {
            newDirPath = dirPath;
            // Ensure ancestor index pages exist for the new dir_path
            if (!"index".equals(fileName)) {
                ensureAncestorPages(newDirPath);
            }
        }

        // Detect dir_path change for index pages to propagate to descendants
        String oldDirPath = existing.dirPath();
        boolean dirPathChanged = "index".equals(existing.name()) && !newDirPath.equals(oldDirPath);

        Page updated = new Page(
                existing.id(),
                null,
                newDirPath,
                fileName,
                existing.type(),
                existing.createdDate(),
                Instant.now());

        updated = pageRepository.save(updated);
        searchService.indexPage(updated);

        // Re-index all items with the new page title
        String newTitle = updated.dirPath() + updated.name();
        List<Item> items = itemRepository.findByPageId(pageId);
        for (Item item : items) {
            searchService.indexItem(item, newTitle);
        }

        // Propagate dir_path change to all descendant pages
        if (dirPathChanged) {
            propagateDirPathChange(pageId, oldDirPath, newDirPath);
        }

        // Queue for export
        exportService.queueExport(pageId);

        return updated;
    }

    /**
     * Deletes a page with all its items and their properties.
     *
     * @param pageId the page to delete
     */
    public void deletePage(HMTID pageId) {
        Page page = pageRepository.findById(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));

        // Root index is never deletable
        if ("index".equals(page.name()) && (page.dirPath() == null || page.dirPath().isEmpty())) {
            throw new IllegalArgumentException("Cannot delete the root index page.");
        }

        // Prevent deleting index pages that have children
        if ("index".equals(page.name())) {
            List<Page> children = pageRepository.findByParentId(pageId);
            if (!children.isEmpty()) {
                throw new IllegalArgumentException(
                        "Cannot delete index page with " + children.size() + " child page(s). "
                        + "Delete or move child pages first.");
            }
        }

        searchService.removePage(pageId.toString());
        List<String> itemIds = itemRepository.findByPageId(pageId).stream().map(i -> i.id().toString()).toList();
        propertyRepository.deleteAllForTargets("item", itemIds);
        itemRepository.deleteByPageId(pageId);
        pageRepository.delete(pageId);
        exportQueueRepository.remove(pageId);
        // Remove the markdown file too, otherwise the folder scan would bring the page back
        exportService.deletePageFile(page);
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
     * Finds all pages and items that link to the given page (backlinks).
     *
     * @param pageId the target page HMTID
     * @return the list of backlink entries, each containing the source page and
     *         linking item
     */
    public List<BacklinkEntry> getBacklinks(HMTID pageId) {
        List<BacklinkEntry> entries = new ArrayList<>();

        Page targetPage = pageRepository.findById(pageId).orElse(null);
        if (targetPage == null)
            return entries;

        // The items that link to this page (using fileName as title), grouped by source page
        List<Item> linkingItems = itemRepository.findByPageIdContainingLink(targetPage.name());
        for (Page source : pageRepository.findBacklinkSources(pageId)) {
            for (Item item : linkingItems) {
                if (item.pageId().equals(source.id())) {
                    entries.add(new BacklinkEntry(source, item));
                }
            }
        }
        return entries;
    }

    /**
     * Ensures that all ancestor directory pages exist for the given dirPath.
     * For example, dirPath "dir1/dir2/" creates pages ("dir1/", "index") and ("dir1/dir2/", "index").
     */
    public void ensureAncestorPages(String dirPath) {
        if (dirPath == null || dirPath.isEmpty()) return;

        // Split "dir1/dir2/" into ["dir1", "dir2"]
        String[] segments = dirPath.split("/");
        StringBuilder currentDir = new StringBuilder();

        for (String segment : segments) {
            if (segment.isEmpty()) continue;

            String parentDir = currentDir.toString(); // "" or "a/" or "a/b/"
            currentDir.append(segment).append("/");
            String indexDir = currentDir.toString();   // "a/" or "a/b/" or "a/b/c/"

            var existing = pageRepository.findByTypeDirPathFileName("doc", indexDir, "index");
            if (existing.isEmpty()) {
                // Check if a named page should be promoted to index
                var namedPage = pageRepository.findByTypeDirPathFileName("doc", parentDir, segment);
                if (namedPage.isPresent()) {
                    // Rename {parentDir}{segment} → {indexDir}{index}
                    Page page = namedPage.get();
                    Page renamed = new Page(page.id(), null, indexDir, "index", page.type(),
                            page.createdDate(), Instant.now());
                    pageRepository.save(renamed);
                    searchService.indexPage(renamed);
                } else {
                    // Create new index page
                    HMTID id = idGenerator.generatePageId();
                    Instant now = Instant.now();
                    Page page = new Page(id, null, indexDir, "index", "doc", now, now);
                    pageRepository.save(page);
                    searchService.indexPage(page);

                    HMTID itemId = idGenerator.generateItemId();
                    Item item = new Item(itemId, id, null, indexDir, "index", "doc", now, now,
                            FractionalIndexer.initial(), 1, "", "", false, List.of(), null);
                    itemRepository.save(item);
                }
            }
        }
    }

    /**
     * Recursively propagates a dir_path change to all child pages.
     * For non-index children: replaces their dir_path from oldPrefix to newPrefix.
     * For index children: replaces their dir_path prefix and recurses into their children.
     */
    private void propagateDirPathChange(HMTID parentPageId, String oldPrefix, String newPrefix) {
        List<Page> children = pageRepository.findByParentId(parentPageId);
        for (Page child : children) {
            String childNewDirPath = newPrefix + child.dirPath().substring(oldPrefix.length());
            Page updatedChild = new Page(
                    child.id(), null, childNewDirPath, child.name(), child.type(),
                    child.createdDate(), child.modifiedDate());
            updatedChild = pageRepository.save(updatedChild);
            searchService.indexPage(updatedChild);

            // Re-index items
            String childTitle = updatedChild.dirPath() + updatedChild.name();
            for (Item item : itemRepository.findByPageId(child.id())) {
                searchService.indexItem(item, childTitle);
            }

            exportService.queueExport(child.id());

            // Recurse into index children
            if ("index".equals(child.name())) {
                propagateDirPathChange(child.id(), child.dirPath(), childNewDirPath);
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
