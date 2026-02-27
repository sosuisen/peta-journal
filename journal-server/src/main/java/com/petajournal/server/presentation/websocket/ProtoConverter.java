package com.petajournal.server.presentation.websocket;

import com.petajournal.proto.*;
import com.petajournal.server.domain.model.BacklinkEntry;

import java.util.List;
import java.util.UUID;

/**
 * Converts between domain models and protobuf messages.
 */
final class ProtoConverter {

    private ProtoConverter() {}

    static Page toProto(com.petajournal.server.domain.model.Page page) {
        Page.Builder b = Page.newBuilder()
                .setId(page.id().toString())
                .setDirPath(page.dirPath())
                .setFileName(page.fileName())
                .setType(page.type() != null ? page.type() : "doc")
                .setCreatedDate(Timestamp.newBuilder().setMillis(page.createdDate().toEpochMilli()).build())
                .setModifiedDate(Timestamp.newBuilder().setMillis(page.modifiedDate().toEpochMilli()).build());
        if (page.tags() != null) {
            b.addAllTags(page.tags());
        }
        return b.build();
    }

    static Block toProto(com.petajournal.server.domain.model.Block block) {
        Block.Builder b = Block.newBuilder()
                .setId(block.id().toString())
                .setPageId(block.pageId().toString())
                .setDirPath(block.dirPath())
                .setFileName(block.fileName())
                .setType(block.type() != null ? block.type() : "doc")
                .setCreatedDate(Timestamp.newBuilder().setMillis(block.createdDate().toEpochMilli()).build())
                .setModifiedDate(Timestamp.newBuilder().setMillis(block.modifiedDate().toEpochMilli()).build())
                .setIdx(block.idx())
                .setLevel(block.level())
                .setBody(block.body())
                .setHtml(block.html())
                .setCollapsed(block.collapsed())
                .setStatus(block.status() != null ? block.status() : "normal");
        if (block.types() != null) {
            b.addAllTypes(block.types());
        }
        return b.build();
    }

    static SearchResult toProto(com.petajournal.server.domain.model.SearchResult result) {
        return SearchResult.newBuilder()
                .setPageId(result.pageId().toString())
                .setPageTitle(result.pageTitle())
                .setBlockId(result.blockId() != null ? result.blockId() : "")
                .setSnippet(result.snippet() != null ? result.snippet() : "")
                .setScore(result.score())
                .build();
    }

    static com.petajournal.proto.BookmarkGroup toProto(com.petajournal.server.domain.model.BookmarkGroup group) {
        return com.petajournal.proto.BookmarkGroup.newBuilder()
                .setId(group.id().toString())
                .setName(group.name())
                .setSortOrder(group.sortOrder())
                .setCreatedAt(Timestamp.newBuilder().setMillis(group.createdAt().toEpochMilli()).build())
                .build();
    }

    static com.petajournal.proto.Bookmark toProto(com.petajournal.server.domain.model.Bookmark bookmark) {
        com.petajournal.proto.Bookmark.Builder b = com.petajournal.proto.Bookmark.newBuilder()
                .setId(bookmark.id().toString())
                .setGroupId(bookmark.groupId().toString())
                .setTargetType(bookmark.targetType())
                .setTargetId(bookmark.targetId())
                .setSortOrder(bookmark.sortOrder())
                .setCreatedAt(Timestamp.newBuilder().setMillis(bookmark.createdAt().toEpochMilli()).build());
        if (bookmark.targetPage() != null) {
            b.setTargetPage(toProto(bookmark.targetPage()));
        }
        return b.build();
    }

    static com.petajournal.proto.BacklinkEntry toProto(BacklinkEntry entry) {
        com.petajournal.proto.BacklinkEntry.Builder b = com.petajournal.proto.BacklinkEntry.newBuilder()
                .setSourcePage(toProto(entry.sourcePage()));
        if (entry.sourceBlock() != null) {
            b.setSourceBlock(toProto(entry.sourceBlock()));
        }
        return b.build();
    }
}
