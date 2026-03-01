package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.BookmarkService;
import com.petajournal.server.presentation.rest.dto.BookmarkDto;
import com.petajournal.server.presentation.rest.dto.BookmarkGroupDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.util.List;
import java.util.UUID;

@Path("/api/bookmarks")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class BookmarksResource {

    @Inject
    BookmarkService bookmarkService;

    // --- Request/Response records ---

    record GetBookmarksResponse(List<BookmarkGroupDto> groups, List<BookmarkDto> bookmarks) {}
    record CreateBookmarkRequest(String groupId, String targetType, String targetId) {}
    record CreateBookmarkResponse(BookmarkDto bookmark) {}
    record MoveBookmarkRequest(String bookmarkId, String targetGroupId, String afterBookmarkId) {}
    record MoveBookmarkResponse(BookmarkDto bookmark) {}
    record CreateBookmarkGroupRequest(String name) {}
    record CreateBookmarkGroupResponse(BookmarkGroupDto group) {}
    record UpdateBookmarkGroupRequest(String name) {}
    record UpdateBookmarkGroupResponse(BookmarkGroupDto group) {}
    record MoveBookmarkGroupRequest(String groupId, String afterGroupId) {}
    record MoveBookmarkGroupResponse(BookmarkGroupDto group) {}

    @GET
    public GetBookmarksResponse getBookmarks() {
        var result = bookmarkService.getAllBookmarks();
        return new GetBookmarksResponse(
                result.groups().stream().map(DtoMapper::toDto).toList(),
                result.bookmarks().stream().map(DtoMapper::toDto).toList());
    }

    @POST
    public CreateBookmarkResponse createBookmark(CreateBookmarkRequest req) {
        var bookmark = bookmarkService.createBookmark(
                UUID.fromString(req.groupId()),
                req.targetType(),
                req.targetId());
        return new CreateBookmarkResponse(DtoMapper.toDto(bookmark));
    }

    @DELETE
    @Path("/{id}")
    public void deleteBookmark(@PathParam("id") String id) {
        bookmarkService.deleteBookmark(UUID.fromString(id));
    }

    @POST
    @Path("/move")
    public MoveBookmarkResponse moveBookmark(MoveBookmarkRequest req) {
        var bookmark = bookmarkService.moveBookmark(
                UUID.fromString(req.bookmarkId()),
                UUID.fromString(req.targetGroupId()),
                (req.afterBookmarkId() != null && !req.afterBookmarkId().isEmpty())
                        ? req.afterBookmarkId() : null);
        return new MoveBookmarkResponse(DtoMapper.toDto(bookmark));
    }

    @POST
    @Path("/groups")
    public CreateBookmarkGroupResponse createBookmarkGroup(CreateBookmarkGroupRequest req) {
        var group = bookmarkService.createGroup(req.name());
        return new CreateBookmarkGroupResponse(DtoMapper.toDto(group));
    }

    @PUT
    @Path("/groups/{id}")
    public UpdateBookmarkGroupResponse updateBookmarkGroup(@PathParam("id") String id, UpdateBookmarkGroupRequest req) {
        var group = bookmarkService.updateGroup(UUID.fromString(id), req.name());
        return new UpdateBookmarkGroupResponse(DtoMapper.toDto(group));
    }

    @DELETE
    @Path("/groups/{id}")
    public void deleteBookmarkGroup(@PathParam("id") String id) {
        bookmarkService.deleteGroup(UUID.fromString(id));
    }

    @POST
    @Path("/groups/move")
    public MoveBookmarkGroupResponse moveBookmarkGroup(MoveBookmarkGroupRequest req) {
        var group = bookmarkService.moveGroup(
                UUID.fromString(req.groupId()),
                (req.afterGroupId() != null && !req.afterGroupId().isEmpty())
                        ? req.afterGroupId() : null);
        return new MoveBookmarkGroupResponse(DtoMapper.toDto(group));
    }
}
