package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.BlockService;
import com.petajournal.server.presentation.rest.dto.BlockDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.util.Arrays;
import java.util.List;
import java.util.UUID;

@Path("/api/blocks")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class BlocksResource {

    @Inject
    BlockService blockService;

    @Inject
    SseBroadcaster sseBroadcaster;

    // --- Request/Response records ---

    record CreateBlockRequest(String pageId, String afterBlockId, int level, String body, String id) {}
    record CreateBlockResponse(BlockDto block) {}
    record UpdateBlockRequest(String body, Integer level, Boolean collapsed, String status) {}
    record UpdateBlockResponse(BlockDto block) {}
    record UpdateBlockStatusRequest(String status) {}
    record UpdateBlockStatusResponse(BlockDto block) {}
    record MoveBlocksRequest(List<String> blockIds, String targetPageId, String afterBlockId, Integer levelDelta) {}
    record MoveBlocksResponse(List<BlockDto> blocks) {}
    record GetBlocksByStatusResponse(List<BlockDto> blocks) {}
    record ExportTreeMarkdownRequest(List<String> blockIds) {}
    record ExportTreeMarkdownResponse(String treeMarkdown, String html, String flatMarkdown) {}
    record ImportTreeMarkdownRequest(String treeMarkdown) {}
    record ImportTreeMarkdownResponse(String flatMarkdown) {}

    @POST
    public CreateBlockResponse createBlock(CreateBlockRequest req) {
        UUID clientId = (req.id() != null && !req.id().isEmpty()) ? UUID.fromString(req.id()) : null;
        UUID afterBlockId = (req.afterBlockId() != null && !req.afterBlockId().isEmpty())
                ? UUID.fromString(req.afterBlockId()) : null;
        var block = blockService.createBlock(
                UUID.fromString(req.pageId()),
                afterBlockId,
                req.level(),
                req.body(),
                clientId);
        sseBroadcaster.broadcastPagesUpdated(req.pageId());
        return new CreateBlockResponse(DtoMapper.toDto(block));
    }

    @PUT
    @Path("/{id}")
    public UpdateBlockResponse updateBlock(@PathParam("id") String id, UpdateBlockRequest req) {
        var block = blockService.updateBlock(
                UUID.fromString(id),
                req.body(),
                req.level(),
                req.collapsed(),
                req.status());
        sseBroadcaster.broadcastPagesUpdated(block.pageId().toString());
        return new UpdateBlockResponse(DtoMapper.toDto(block));
    }

    @DELETE
    @Path("/{id}")
    public void deleteBlock(@PathParam("id") String id) {
        String pageId = blockService.getBlockPageId(UUID.fromString(id));
        blockService.deleteBlock(UUID.fromString(id));
        if (pageId != null) {
            sseBroadcaster.broadcastPagesUpdated(pageId);
        }
    }

    @PUT
    @Path("/{id}/status")
    public UpdateBlockStatusResponse updateBlockStatus(@PathParam("id") String id, UpdateBlockStatusRequest req) {
        var block = blockService.updateBlockStatus(UUID.fromString(id), req.status());
        sseBroadcaster.broadcastPagesUpdated(block.pageId().toString());
        return new UpdateBlockStatusResponse(DtoMapper.toDto(block));
    }

    @POST
    @Path("/move")
    public MoveBlocksResponse moveBlocks(MoveBlocksRequest req) {
        UUID afterBlockId = (req.afterBlockId() != null && !req.afterBlockId().isEmpty())
                ? UUID.fromString(req.afterBlockId()) : null;
        var blocks = blockService.moveBlocks(
                req.blockIds().stream().map(UUID::fromString).toList(),
                UUID.fromString(req.targetPageId()),
                afterBlockId,
                req.levelDelta());
        sseBroadcaster.broadcastPagesUpdated(req.targetPageId());
        return new MoveBlocksResponse(blocks.stream().map(DtoMapper::toDto).toList());
    }

    @GET
    public GetBlocksByStatusResponse getBlocksByStatus(@QueryParam("statuses") String statuses) {
        List<String> statusList = (statuses != null && !statuses.isEmpty())
                ? Arrays.asList(statuses.split(","))
                : List.of();
        var blocks = blockService.getBlocksByStatuses(statusList);
        return new GetBlocksByStatusResponse(blocks.stream().map(DtoMapper::toDto).toList());
    }

    @POST
    @Path("/export")
    public ExportTreeMarkdownResponse exportTreeMarkdown(ExportTreeMarkdownRequest req) {
        var result = blockService.exportTreeMarkdown(req.blockIds().stream().map(UUID::fromString).toList());
        return new ExportTreeMarkdownResponse(result.treeMarkdown(), result.html(), result.flatMarkdown());
    }

    @POST
    @Path("/import")
    public ImportTreeMarkdownResponse importTreeMarkdown(ImportTreeMarkdownRequest req) {
        String flatMd = blockService.importTreeMarkdown(req.treeMarkdown());
        return new ImportTreeMarkdownResponse(flatMd);
    }
}
