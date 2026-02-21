package com.petajournal.server.application;

import com.petajournal.server.domain.FractionalIndexer;
import com.petajournal.server.domain.model.Bookmark;
import com.petajournal.server.domain.model.BookmarkGroup;
import com.petajournal.server.domain.repository.BookmarkRepository;
import jakarta.enterprise.context.ApplicationScoped;
import jakarta.inject.Inject;

import java.time.Instant;
import java.util.List;
import java.util.UUID;

@ApplicationScoped
public class BookmarkService {

    @Inject
    BookmarkRepository bookmarkRepository;

    public record GetBookmarksResult(List<BookmarkGroup> groups, List<Bookmark> bookmarks) {}

    public GetBookmarksResult getAllBookmarks() {
        List<BookmarkGroup> groups = bookmarkRepository.findAllGroups();
        // Auto-create default group if none exists
        if (groups.isEmpty()) {
            var defaultGroup = createGroup("");
            groups = List.of(defaultGroup);
        }
        List<Bookmark> bookmarks = bookmarkRepository.findAllBookmarks();
        return new GetBookmarksResult(groups, bookmarks);
    }

    public BookmarkGroup createGroup(String name) {
        List<BookmarkGroup> groups = bookmarkRepository.findAllGroups();
        String lastSortOrder = groups.isEmpty() ? null : groups.get(groups.size() - 1).sortOrder();
        String sortOrder = FractionalIndexer.between(lastSortOrder, null);

        var group = new BookmarkGroup(UUID.randomUUID(), name, sortOrder, Instant.now());
        return bookmarkRepository.saveGroup(group);
    }

    public Bookmark createBookmark(UUID groupId, String targetType, String targetId) {
        // Duplicate check
        var existing = bookmarkRepository.findBookmarkByTarget(targetType, targetId);
        if (existing.isPresent()) {
            return existing.get();
        }

        List<Bookmark> groupBookmarks = bookmarkRepository.findBookmarksByGroupId(groupId);
        String lastSortOrder = groupBookmarks.isEmpty() ? null : groupBookmarks.get(groupBookmarks.size() - 1).sortOrder();
        String sortOrder = FractionalIndexer.between(lastSortOrder, null);

        var bookmark = new Bookmark(UUID.randomUUID(), groupId, targetType, targetId, sortOrder, Instant.now(), null);
        bookmarkRepository.saveBookmark(bookmark);
        // Re-fetch to get resolved page data
        return bookmarkRepository.findBookmarkById(bookmark.id()).orElse(bookmark);
    }

    public void deleteBookmark(UUID id) {
        bookmarkRepository.deleteBookmark(id);
    }

    public void deleteGroup(UUID id) {
        bookmarkRepository.deleteGroup(id);
    }

    public Bookmark moveBookmark(UUID bookmarkId, UUID targetGroupId, String afterBookmarkId) {
        List<Bookmark> groupBookmarks = bookmarkRepository.findBookmarksByGroupId(targetGroupId);

        String beforeSort = null;
        String afterSort = null;

        if (afterBookmarkId == null || afterBookmarkId.isEmpty()) {
            // Insert at beginning
            if (!groupBookmarks.isEmpty()) {
                afterSort = groupBookmarks.get(0).sortOrder();
            }
        } else {
            UUID afterId = UUID.fromString(afterBookmarkId);
            for (int i = 0; i < groupBookmarks.size(); i++) {
                if (groupBookmarks.get(i).id().equals(afterId)) {
                    beforeSort = groupBookmarks.get(i).sortOrder();
                    if (i + 1 < groupBookmarks.size()) {
                        // Skip the bookmark being moved if it's the next one
                        var next = groupBookmarks.get(i + 1);
                        if (!next.id().equals(bookmarkId)) {
                            afterSort = next.sortOrder();
                        } else if (i + 2 < groupBookmarks.size()) {
                            afterSort = groupBookmarks.get(i + 2).sortOrder();
                        }
                    }
                    break;
                }
            }
        }

        String newSortOrder = FractionalIndexer.between(beforeSort, afterSort);
        bookmarkRepository.updateBookmarkGroup(bookmarkId, targetGroupId, newSortOrder);
        return bookmarkRepository.findBookmarkById(bookmarkId)
                .orElseThrow(() -> new IllegalArgumentException("Bookmark not found: " + bookmarkId));
    }

    public BookmarkGroup moveGroup(UUID groupId, String afterGroupId) {
        List<BookmarkGroup> groups = bookmarkRepository.findAllGroups();

        String beforeSort = null;
        String afterSort = null;

        if (afterGroupId == null || afterGroupId.isEmpty()) {
            // Insert at beginning
            if (!groups.isEmpty()) {
                afterSort = groups.get(0).sortOrder();
            }
        } else {
            UUID afterId = UUID.fromString(afterGroupId);
            for (int i = 0; i < groups.size(); i++) {
                if (groups.get(i).id().equals(afterId)) {
                    beforeSort = groups.get(i).sortOrder();
                    if (i + 1 < groups.size()) {
                        var next = groups.get(i + 1);
                        if (!next.id().equals(groupId)) {
                            afterSort = next.sortOrder();
                        } else if (i + 2 < groups.size()) {
                            afterSort = groups.get(i + 2).sortOrder();
                        }
                    }
                    break;
                }
            }
        }

        String newSortOrder = FractionalIndexer.between(beforeSort, afterSort);
        bookmarkRepository.updateGroupSortOrder(groupId, newSortOrder);
        return bookmarkRepository.findGroupById(groupId)
                .orElseThrow(() -> new IllegalArgumentException("Group not found: " + groupId));
    }

    public BookmarkGroup updateGroup(UUID groupId, String name) {
        bookmarkRepository.updateGroupName(groupId, name);
        return bookmarkRepository.findGroupById(groupId)
                .orElseThrow(() -> new IllegalArgumentException("Group not found: " + groupId));
    }
}
