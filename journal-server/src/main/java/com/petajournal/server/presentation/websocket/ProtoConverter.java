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
                .setCreatedDate(Timestamp.newBuilder().setMillis(page.createdDate().toEpochMilli()).build())
                .setModifiedDate(Timestamp.newBuilder().setMillis(page.modifiedDate().toEpochMilli()).build())
                .setPropertiesJson(page.propertiesJson() != null ? page.propertiesJson() : "{}");
        if (page.tags() != null) {
            b.addAllTags(page.tags());
        }
        return b.build();
    }

    static Block toProto(com.petajournal.server.domain.model.Block block) {
        Block.Builder b = Block.newBuilder()
                .setId(block.id())
                .setPageId(block.pageId().toString())
                .setDirPath(block.dirPath())
                .setFileName(block.fileName())
                .setCreatedDate(Timestamp.newBuilder().setMillis(block.createdDate().toEpochMilli()).build())
                .setModifiedDate(Timestamp.newBuilder().setMillis(block.modifiedDate().toEpochMilli()).build())
                .setIdx(block.idx())
                .setLevel(block.level())
                .setBody(block.body())
                .setHtml(block.html())
                .setCollapsed(block.collapsed())
                .setStatus(block.status() != null ? block.status() : "normal")
                .setPropertiesJson(block.propertiesJson() != null ? block.propertiesJson() : "{}");
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

    static com.petajournal.proto.BacklinkEntry toProto(BacklinkEntry entry) {
        com.petajournal.proto.BacklinkEntry.Builder b = com.petajournal.proto.BacklinkEntry.newBuilder()
                .setSourcePage(toProto(entry.sourcePage()));
        if (entry.sourceBlock() != null) {
            b.setSourceBlock(toProto(entry.sourceBlock()));
        }
        return b.build();
    }
}
