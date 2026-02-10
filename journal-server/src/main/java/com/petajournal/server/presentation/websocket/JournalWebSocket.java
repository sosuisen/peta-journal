package com.petajournal.server.presentation.websocket;

import com.petajournal.proto.*;
import com.petajournal.server.application.BlockService;
import com.petajournal.server.application.ConfigService;
import com.petajournal.server.application.JournalService;
import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.repository.SearchService;
import io.quarkus.websockets.next.OnBinaryMessage;
import io.quarkus.websockets.next.OnOpen;
import io.quarkus.websockets.next.WebSocket;
import jakarta.inject.Inject;
import org.jboss.logging.Logger;

import io.vertx.core.buffer.Buffer;
import java.time.LocalDate;
import java.util.List;
import java.util.UUID;

@WebSocket(path = "/ws")
public class JournalWebSocket {

    private static final Logger LOG = Logger.getLogger(JournalWebSocket.class);

    @Inject
    PageService pageService;

    @Inject
    BlockService blockService;

    @Inject
    JournalService journalService;

    @Inject
    SearchService searchService;

    @Inject
    ConfigService configService;

    @OnOpen
    void onOpen() {
        LOG.info("WebSocket client connected");
    }

    @OnBinaryMessage
    Buffer onMessage(Buffer message) {
        try {
            byte[] bytes = message.getBytes();
            ApiRequest request = ApiRequest.parseFrom(bytes);
            ApiResponse response = handleRequest(request);
            return Buffer.buffer(response.toByteArray());
        } catch (Exception e) {
            LOG.error("Error processing WebSocket message", e);
            ApiResponse errorResponse = ApiResponse.newBuilder()
                    .setSuccess(false)
                    .setErrorMessage(e.getMessage() != null ? e.getMessage() : "Internal error")
                    .build();
            return Buffer.buffer(errorResponse.toByteArray());
        }
    }

    private ApiResponse handleRequest(ApiRequest request) {
        String requestId = request.getRequestId();

        try {
            ApiResponse.Builder response = ApiResponse.newBuilder()
                    .setRequestId(requestId)
                    .setSuccess(true);

            switch (request.getPayloadCase()) {
                case GET_PAGE -> response.setGetPage(handleGetPage(request.getGetPage()));
                case GET_JOURNAL -> response.setGetJournal(handleGetJournal(request.getGetJournal()));
                case CREATE_PAGE -> response.setCreatePage(handleCreatePage(request.getCreatePage()));
                case DELETE_PAGE -> response.setDeletePage(handleDeletePage(request.getDeletePage()));
                case UPDATE_PAGE -> response.setUpdatePage(handleUpdatePage(request.getUpdatePage()));
                case CREATE_BLOCK -> response.setCreateBlock(handleCreateBlock(request.getCreateBlock()));
                case UPDATE_BLOCK -> response.setUpdateBlock(handleUpdateBlock(request.getUpdateBlock()));
                case MOVE_BLOCKS -> response.setMoveBlocks(handleMoveBlocks(request.getMoveBlocks()));
                case DELETE_BLOCK -> response.setDeleteBlock(handleDeleteBlock(request.getDeleteBlock()));
                case UPDATE_BLOCK_STATUS -> response.setUpdateBlockStatus(handleUpdateBlockStatus(request.getUpdateBlockStatus()));
                case SEARCH_PAGE_NAMES -> response.setSearchPageNames(handleSearchPageNames(request.getSearchPageNames()));
                case SEARCH_FULL_TEXT -> response.setSearchFullText(handleSearchFullText(request.getSearchFullText()));
                case GET_RECENT_PAGES -> response.setGetRecentPages(handleGetRecentPages(request.getGetRecentPages()));
                case GET_BACKLINKS -> response.setGetBacklinks(handleGetBacklinks(request.getGetBacklinks()));
                case GET_MOST_RECENT_JOURNAL_BEFORE -> response.setGetMostRecentJournalBefore(handleGetMostRecentJournalBefore(request.getGetMostRecentJournalBefore()));
                case GET_JOURNAL_BY_NAME -> response.setGetJournalByName(handleGetJournalByName(request.getGetJournalByName()));
                case GET_BLOCKS_BY_STATUS -> response.setGetBlocksByStatus(handleGetBlocksByStatus(request.getGetBlocksByStatus()));
                case GET_CONFIG -> response.setGetConfig(handleGetConfig());
                case UPDATE_CONFIG -> response.setUpdateConfig(handleUpdateConfig(request.getUpdateConfig()));
                default -> {
                    response.setSuccess(false);
                    response.setErrorMessage("Unknown request type: " + request.getPayloadCase());
                }
            }

            return response.build();
        } catch (Exception e) {
            LOG.errorv("Error handling request {0}: {1}", requestId, e.getMessage(), e);
            return ApiResponse.newBuilder()
                    .setRequestId(requestId)
                    .setSuccess(false)
                    .setErrorMessage(e.getMessage() != null ? e.getMessage() : "Internal error")
                    .build();
        }
    }

    private GetPageResponse handleGetPage(GetPageRequest req) {
        UUID pageId = UUID.fromString(req.getPageId());
        var page = pageService.getPage(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));
        var blocks = pageService.getPageBlocks(pageId);

        return GetPageResponse.newBuilder()
                .setPage(ProtoConverter.toProto(page))
                .addAllBlocks(blocks.stream().map(ProtoConverter::toProto).toList())
                .build();
    }

    private GetJournalResponse handleGetJournal(GetJournalRequest req) {
        LocalDate date = LocalDate.parse(req.getDate());
        var result = journalService.getOrCreateJournal(date);

        return GetJournalResponse.newBuilder()
                .setPage(ProtoConverter.toProto(result.page()))
                .addAllBlocks(result.blocks().stream().map(ProtoConverter::toProto).toList())
                .setCreated(result.created())
                .build();
    }

    private GetMostRecentJournalBeforeResponse handleGetMostRecentJournalBefore(GetMostRecentJournalBeforeRequest req) {
        LocalDate beforeDate = LocalDate.parse(req.getBeforeDate());
        var result = journalService.getMostRecentJournalBefore(beforeDate);

        GetMostRecentJournalBeforeResponse.Builder builder = GetMostRecentJournalBeforeResponse.newBuilder()
                .setFound(result.found());

        if (result.found()) {
            builder.setPage(ProtoConverter.toProto(result.page()))
                    .addAllBlocks(result.blocks().stream().map(ProtoConverter::toProto).toList());
        }

        return builder.build();
    }

    private CreatePageResponse handleCreatePage(CreatePageRequest req) {
        var page = pageService.createPage(req.getTitle(), req.getDirPath(), req.getTagsList());
        return CreatePageResponse.newBuilder()
                .setPage(ProtoConverter.toProto(page))
                .build();
    }

    private DeletePageResponse handleDeletePage(DeletePageRequest req) {
        pageService.deletePage(UUID.fromString(req.getPageId()));
        return DeletePageResponse.newBuilder().build();
    }

    private UpdatePageResponse handleUpdatePage(UpdatePageRequest req) {
        var page = pageService.updatePage(
                UUID.fromString(req.getPageId()),
                req.hasTitle() ? req.getTitle() : null,
                req.getTagsList().isEmpty() ? null : req.getTagsList(),
                req.getPropertiesJson().isEmpty() ? null : req.getPropertiesJson()
        );
        return UpdatePageResponse.newBuilder()
                .setPage(ProtoConverter.toProto(page))
                .build();
    }

    private CreateBlockResponse handleCreateBlock(CreateBlockRequest req) {
        var block = blockService.createBlock(
                UUID.fromString(req.getPageId()),
                req.getAfterBlockId(),
                req.getLevel(),
                req.getBody()
        );
        return CreateBlockResponse.newBuilder()
                .setBlock(ProtoConverter.toProto(block))
                .build();
    }

    private UpdateBlockResponse handleUpdateBlock(UpdateBlockRequest req) {
        var block = blockService.updateBlock(
                req.getBlockId(),
                req.hasBody() ? req.getBody() : null,
                req.hasLevel() ? req.getLevel() : null,
                req.hasCollapsed() ? req.getCollapsed() : null,
                req.hasStatus() ? req.getStatus() : null,
                req.hasPropertiesJson() ? req.getPropertiesJson() : null
        );
        return UpdateBlockResponse.newBuilder()
                .setBlock(ProtoConverter.toProto(block))
                .build();
    }

    private MoveBlocksResponse handleMoveBlocks(MoveBlocksRequest req) {
        var blocks = blockService.moveBlocks(
                req.getBlockIdsList(),
                UUID.fromString(req.getTargetPageId()),
                req.getAfterBlockId(),
                req.hasLevelDelta() ? req.getLevelDelta() : null
        );
        return MoveBlocksResponse.newBuilder()
                .addAllBlocks(blocks.stream().map(ProtoConverter::toProto).toList())
                .build();
    }

    private DeleteBlockResponse handleDeleteBlock(DeleteBlockRequest req) {
        blockService.deleteBlock(req.getBlockId());
        return DeleteBlockResponse.newBuilder().build();
    }

    private UpdateBlockStatusResponse handleUpdateBlockStatus(UpdateBlockStatusRequest req) {
        var block = blockService.updateBlockStatus(req.getBlockId(), req.getStatus());
        return UpdateBlockStatusResponse.newBuilder()
                .setBlock(ProtoConverter.toProto(block))
                .build();
    }

    private SearchPageNamesResponse handleSearchPageNames(SearchPageNamesRequest req) {
        var results = searchService.searchPageNames(req.getQuery(), req.getMaxResults());
        return SearchPageNamesResponse.newBuilder()
                .addAllResults(results.stream().map(ProtoConverter::toProto).toList())
                .build();
    }

    private SearchFullTextResponse handleSearchFullText(SearchFullTextRequest req) {
        var results = searchService.searchFullText(req.getQuery(), req.getMaxResults());
        return SearchFullTextResponse.newBuilder()
                .addAllResults(results.stream().map(ProtoConverter::toProto).toList())
                .setTotalHits(results.size())
                .build();
    }

    private GetRecentPagesResponse handleGetRecentPages(GetRecentPagesRequest req) {
        int max = req.getMaxResults() > 0 ? req.getMaxResults() : 20;
        var pages = pageService.getRecentPages(max);
        return GetRecentPagesResponse.newBuilder()
                .addAllPages(pages.stream().map(ProtoConverter::toProto).toList())
                .build();
    }

    private GetBlocksByStatusResponse handleGetBlocksByStatus(GetBlocksByStatusRequest req) {
        var blocks = blockService.getBlocksByStatuses(req.getStatusesList());
        return GetBlocksByStatusResponse.newBuilder()
                .addAllBlocks(blocks.stream().map(ProtoConverter::toProto).toList())
                .build();
    }

    private GetBacklinksResponse handleGetBacklinks(GetBacklinksRequest req) {
        var entries = pageService.getBacklinks(UUID.fromString(req.getPageId()));
        return GetBacklinksResponse.newBuilder()
                .addAllEntries(entries.stream().map(ProtoConverter::toProto).toList())
                .build();
    }

    private GetJournalByNameResponse handleGetJournalByName(GetJournalByNameRequest req) {
        var result = journalService.getOrCreateJournalByName(req.getFileName());
        return GetJournalByNameResponse.newBuilder()
                .setPage(ProtoConverter.toProto(result.page()))
                .addAllBlocks(result.blocks().stream().map(ProtoConverter::toProto).toList())
                .setCreated(result.created())
                .build();
    }

    private GetConfigResponse handleGetConfig() {
        return GetConfigResponse.newBuilder()
                .setWeekNumbering(configService.getWeekNumbering())
                .build();
    }

    private UpdateConfigResponse handleUpdateConfig(UpdateConfigRequest req) {
        configService.setWeekNumbering(req.getWeekNumbering());
        return UpdateConfigResponse.newBuilder()
                .setWeekNumbering(configService.getWeekNumbering())
                .build();
    }
}
