package com.petajournal.server.domain.repository;

import com.petajournal.server.domain.model.Bookmark;
import com.petajournal.server.domain.model.BookmarkGroup;

import java.util.List;
import java.util.Optional;
import java.util.UUID;

public interface BookmarkRepository {

    List<BookmarkGroup> findAllGroups();

    Optional<BookmarkGroup> findGroupById(UUID id);

    List<Bookmark> findAllBookmarks();

    List<Bookmark> findBookmarksByGroupId(UUID groupId);

    Optional<Bookmark> findBookmarkByTarget(String targetType, String targetId);

    Optional<Bookmark> findBookmarkById(UUID id);

    BookmarkGroup saveGroup(BookmarkGroup group);

    Bookmark saveBookmark(Bookmark bookmark);

    void deleteBookmark(UUID id);

    void deleteGroup(UUID id);

    void updateBookmarkSortOrder(UUID id, String sortOrder);

    void updateBookmarkGroup(UUID id, UUID newGroupId, String sortOrder);

    void updateGroupSortOrder(UUID id, String sortOrder);

    void updateGroupName(UUID id, String name);
}
