package com.petajournal.server.presentation.websocket;

import com.petajournal.proto.*;
import com.petajournal.server.application.BlockService;
import com.petajournal.server.application.BookmarkService;
import com.petajournal.server.application.ConfigService;
import com.petajournal.server.application.JournalService;
import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.model.BacklinkEntry;
import com.petajournal.server.domain.model.Page;
import com.petajournal.server.domain.model.SearchResult;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.domain.repository.SearchService;
import io.quarkus.websockets.next.OnBinaryMessage;
import io.quarkus.websockets.next.OnOpen;
import io.quarkus.websockets.next.WebSocket;
import io.quarkus.websockets.next.WebSocketConnection;
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
    PageRepository pageRepository;

    @Inject
    BookmarkService bookmarkService;

    @Inject
    ConfigService configService;

    @Inject
    WebSocketConnection connection;

    @Inject
    EventBroadcastService eventBroadcastService;

    @OnOpen
    void onOpen() {
        LOG.debug("WebSocket client connected: " + connection.id());
        eventBroadcastService.sendConnectedEvent(connection);
    }

    @OnBinaryMessage
    Buffer onMessage(Buffer message) {
        try {
            byte[] bytes = message.getBytes();
            ApiRequest request = ApiRequest.parseFrom(bytes);

            // Pre-capture pageId for delete (block is gone after delete)
            String pageIdForBroadcast = extractPageIdForBroadcast(request);

            ApiResponse response = handleRequest(request);

            // Broadcast pages-updated for successful mutations
            if (response.getSuccess() && pageIdForBroadcast != null) {
                eventBroadcastService.broadcastPagesUpdated(pageIdForBroadcast, connection.id());
            }

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

    private String extractPageIdForBroadcast(ApiRequest request) {
        return switch (request.getPayloadCase()) {
            case CREATE_BLOCK -> request.getCreateBlock().getPageId();
            case UPDATE_BLOCK -> blockService.getBlockPageId(request.getUpdateBlock().getBlockId());
            case MOVE_BLOCKS -> request.getMoveBlocks().getTargetPageId();
            case DELETE_BLOCK -> blockService.getBlockPageId(request.getDeleteBlock().getBlockId());
            case UPDATE_BLOCK_STATUS -> blockService.getBlockPageId(request.getUpdateBlockStatus().getBlockId());
            case DELETE_PAGE -> request.getDeletePage().getPageId();
            case UPDATE_PAGE -> request.getUpdatePage().getPageId();
            default -> null;
        };
    }

    private ApiResponse handleRequest(ApiRequest request) {
        String requestId = request.getRequestId();

        try {
            ApiResponse.Builder response = ApiResponse.newBuilder()
                    .setRequestId(requestId)
                    .setSuccess(true);

            switch (request.getPayloadCase()) {
                case GET_PAGE_BY_ID -> response.setGetPageById(handleGetPageById(request.getGetPageById()));
                case GET_OR_CREATE_DOC -> response.setGetOrCreateDoc(handleGetOrCreateDoc(request.getGetOrCreateDoc()));
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
                case GET_OR_CREATE_JOURNAL -> response.setGetOrCreateJournal(handleGetOrCreateJournal(request.getGetOrCreateJournal()));
                case GET_BLOCKS_BY_STATUS -> response.setGetBlocksByStatus(handleGetBlocksByStatus(request.getGetBlocksByStatus()));
                case GET_CONFIG -> response.setGetConfig(handleGetConfig());
                case UPDATE_CONFIG -> response.setUpdateConfig(handleUpdateConfig(request.getUpdateConfig()));
                case EXPORT_TREE_MARKDOWN -> response.setExportTreeMarkdown(handleExportTreeMarkdown(request.getExportTreeMarkdown()));
                case IMPORT_TREE_MARKDOWN -> response.setImportTreeMarkdown(handleImportTreeMarkdown(request.getImportTreeMarkdown()));
                case GET_BOOKMARKS -> response.setGetBookmarks(handleGetBookmarks());
                case CREATE_BOOKMARK_GROUP -> response.setCreateBookmarkGroup(handleCreateBookmarkGroup(request.getCreateBookmarkGroup()));
                case CREATE_BOOKMARK -> response.setCreateBookmark(handleCreateBookmark(request.getCreateBookmark()));
                case DELETE_BOOKMARK -> response.setDeleteBookmark(handleDeleteBookmark(request.getDeleteBookmark()));
                case DELETE_BOOKMARK_GROUP -> response.setDeleteBookmarkGroup(handleDeleteBookmarkGroup(request.getDeleteBookmarkGroup()));
                case MOVE_BOOKMARK -> response.setMoveBookmark(handleMoveBookmark(request.getMoveBookmark()));
                case MOVE_BOOKMARK_GROUP -> response.setMoveBookmarkGroup(handleMoveBookmarkGroup(request.getMoveBookmarkGroup()));
                case UPDATE_BOOKMARK_GROUP -> response.setUpdateBookmarkGroup(handleUpdateBookmarkGroup(request.getUpdateBookmarkGroup()));
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

    private GetPageByIdResponse handleGetPageById(GetPageByIdRequest req) {
        UUID pageId = UUID.fromString(req.getPageId());
        var page = pageService.getPage(pageId)
                .orElseThrow(() -> new IllegalArgumentException("Page not found: " + pageId));
        var blocks = pageService.getPageBlocks(pageId);

        return GetPageByIdResponse.newBuilder()
                .setPage(ProtoConverter.toProto(page))
                .addAllBlocks(blocks.stream().map(ProtoConverter::toProto).toList())
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

    private GetOrCreateDocResponse handleGetOrCreateDoc(GetOrCreateDocRequest req) {
        var result = pageService.getOrCreateDoc(req.getTitle(), req.getDirPath(), req.getTagsList());
        return GetOrCreateDocResponse.newBuilder()
                .setPage(ProtoConverter.toProto(result.page()))
                .addAllBlocks(result.blocks().stream().map(ProtoConverter::toProto).toList())
                .setCreated(result.created())
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
                req.getPropertiesJson().isEmpty() ? null : req.getPropertiesJson(),
                req.hasDirPath() ? req.getDirPath() : null
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
        String query = req.getQuery();
        int maxResults = req.getMaxResults() > 0 ? req.getMaxResults() : 20;

        // Collect results, deduplicating by page ID
        java.util.Set<String> seenIds = new java.util.LinkedHashSet<>();
        java.util.List<com.petajournal.proto.SearchResult> allResults = new java.util.ArrayList<>();

        // 0th: Index pages whose last dir_path segment matches query (highest priority)
        for (Page p : pageRepository.findIndexPagesByDirSegment(query, maxResults)) {
            if (seenIds.add(p.id().toString())) {
                allResults.add(ProtoConverter.toProto(SearchResult.fromPage(p)));
            }
        }

        // 1st: SQLite prefix search on dir_path || file_name
        for (Page p : pageRepository.findByPathPrefix(query, maxResults)) {
            if (seenIds.add(p.id().toString())) {
                allResults.add(ProtoConverter.toProto(SearchResult.fromPage(p)));
            }
        }

        if (!allResults.isEmpty()) {
            return SearchPageNamesResponse.newBuilder()
                    .addAllResults(allResults.stream().limit(maxResults).toList())
                    .build();
        }

        // 2nd: Lucene fallback (fuzzy/token-based)
        var luceneResults = searchService.searchPageNames(query, maxResults);
        if (!luceneResults.isEmpty()) {
            return SearchPageNamesResponse.newBuilder()
                    .addAllResults(luceneResults.stream().map(ProtoConverter::toProto).toList())
                    .build();
        }

        // 3rd: SQLite contains search
        List<Page> containsPages = pageRepository.findByPathContains(query, maxResults);
        return SearchPageNamesResponse.newBuilder()
                .addAllResults(containsPages.stream().map(SearchResult::fromPage).map(ProtoConverter::toProto).toList())
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
        int max = req.getMaxResults() > 0 ? req.getMaxResults() : 10;
        String type = req.getType().isEmpty() ? null : req.getType();
        var pages = pageService.getRecentPages(max, type);
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

    private GetOrCreateJournalResponse handleGetOrCreateJournal(GetOrCreateJournalRequest req) {
        var result = journalService.getOrCreateJournalByName(req.getFileName());
        return GetOrCreateJournalResponse.newBuilder()
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

    private ExportTreeMarkdownResponse handleExportTreeMarkdown(ExportTreeMarkdownRequest req) {
        var result = blockService.exportTreeMarkdown(req.getBlockIdsList());
        return ExportTreeMarkdownResponse.newBuilder()
                .setTreeMarkdown(result.treeMarkdown())
                .setHtml(result.html())
                .setFlatMarkdown(result.flatMarkdown())
                .build();
    }

    private ImportTreeMarkdownResponse handleImportTreeMarkdown(ImportTreeMarkdownRequest req) {
        String flatMd = blockService.importTreeMarkdown(req.getTreeMarkdown());
        return ImportTreeMarkdownResponse.newBuilder()
                .setFlatMarkdown(flatMd)
                .build();
    }

    private GetBookmarksResponse handleGetBookmarks() {
        var result = bookmarkService.getAllBookmarks();
        return GetBookmarksResponse.newBuilder()
                .addAllGroups(result.groups().stream().map(ProtoConverter::toProto).toList())
                .addAllBookmarks(result.bookmarks().stream().map(ProtoConverter::toProto).toList())
                .build();
    }

    private CreateBookmarkGroupResponse handleCreateBookmarkGroup(CreateBookmarkGroupRequest req) {
        var group = bookmarkService.createGroup(req.getName());
        return CreateBookmarkGroupResponse.newBuilder()
                .setGroup(ProtoConverter.toProto(group))
                .build();
    }

    private CreateBookmarkResponse handleCreateBookmark(CreateBookmarkRequest req) {
        var bookmark = bookmarkService.createBookmark(
                UUID.fromString(req.getGroupId()),
                req.getTargetType(),
                req.getTargetId()
        );
        return CreateBookmarkResponse.newBuilder()
                .setBookmark(ProtoConverter.toProto(bookmark))
                .build();
    }

    private DeleteBookmarkResponse handleDeleteBookmark(DeleteBookmarkRequest req) {
        bookmarkService.deleteBookmark(UUID.fromString(req.getBookmarkId()));
        return DeleteBookmarkResponse.newBuilder().build();
    }

    private DeleteBookmarkGroupResponse handleDeleteBookmarkGroup(DeleteBookmarkGroupRequest req) {
        bookmarkService.deleteGroup(UUID.fromString(req.getGroupId()));
        return DeleteBookmarkGroupResponse.newBuilder().build();
    }

    private MoveBookmarkResponse handleMoveBookmark(MoveBookmarkRequest req) {
        var bookmark = bookmarkService.moveBookmark(
                UUID.fromString(req.getBookmarkId()),
                UUID.fromString(req.getTargetGroupId()),
                req.getAfterBookmarkId().isEmpty() ? null : req.getAfterBookmarkId()
        );
        return MoveBookmarkResponse.newBuilder()
                .setBookmark(ProtoConverter.toProto(bookmark))
                .build();
    }

    private MoveBookmarkGroupResponse handleMoveBookmarkGroup(MoveBookmarkGroupRequest req) {
        var group = bookmarkService.moveGroup(
                UUID.fromString(req.getGroupId()),
                req.getAfterGroupId().isEmpty() ? null : req.getAfterGroupId()
        );
        return MoveBookmarkGroupResponse.newBuilder()
                .setGroup(ProtoConverter.toProto(group))
                .build();
    }

    private UpdateBookmarkGroupResponse handleUpdateBookmarkGroup(UpdateBookmarkGroupRequest req) {
        var group = bookmarkService.updateGroup(UUID.fromString(req.getGroupId()), req.getName());
        return UpdateBookmarkGroupResponse.newBuilder()
                .setGroup(ProtoConverter.toProto(group))
                .build();
    }
}
