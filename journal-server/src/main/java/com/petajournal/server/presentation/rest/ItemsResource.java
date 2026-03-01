package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ItemService;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.util.Arrays;
import java.util.List;
import java.util.UUID;

@Path("/api/items")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class ItemsResource {

    @Inject
    ItemService itemService;

    @Inject
    SseBroadcaster sseBroadcaster;

    // --- Request/Response records ---

    record CreateItemRequest(String pageId, String afterItemId, int level, String body, String id) {}
    record CreateItemResponse(ItemDto item) {}
    record UpdateItemRequest(String body, Integer level, Boolean collapsed, String status) {}
    record UpdateItemResponse(ItemDto item) {}
    record UpdateItemStatusRequest(String status) {}
    record UpdateItemStatusResponse(ItemDto item) {}
    record MoveItemsRequest(List<String> itemIds, String targetPageId, String afterItemId, Integer levelDelta) {}
    record MoveItemsResponse(List<ItemDto> items) {}
    record GetItemsByStatusResponse(List<ItemDto> items) {}
    record ExportTreeMarkdownRequest(List<String> itemIds) {}
    record ExportTreeMarkdownResponse(String treeMarkdown, String html, String flatMarkdown) {}
    record ImportTreeMarkdownRequest(String treeMarkdown) {}
    record ImportTreeMarkdownResponse(String flatMarkdown) {}

    @POST
    public CreateItemResponse createItem(CreateItemRequest req) {
        UUID clientId = (req.id() != null && !req.id().isEmpty()) ? UUID.fromString(req.id()) : null;
        UUID afterItemId = (req.afterItemId() != null && !req.afterItemId().isEmpty())
                ? UUID.fromString(req.afterItemId()) : null;
        var item = itemService.createItem(
                UUID.fromString(req.pageId()),
                afterItemId,
                req.level(),
                req.body(),
                clientId);
        sseBroadcaster.broadcastPagesUpdated(req.pageId());
        return new CreateItemResponse(DtoMapper.toDto(item));
    }

    @PUT
    @Path("/{id}")
    public UpdateItemResponse updateItem(@PathParam("id") String id, UpdateItemRequest req) {
        var item = itemService.updateItem(
                UUID.fromString(id),
                req.body(),
                req.level(),
                req.collapsed(),
                req.status());
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        return new UpdateItemResponse(DtoMapper.toDto(item));
    }

    @DELETE
    @Path("/{id}")
    public void deleteItem(@PathParam("id") String id) {
        String pageId = itemService.getItemPageId(UUID.fromString(id));
        itemService.deleteItem(UUID.fromString(id));
        if (pageId != null) {
            sseBroadcaster.broadcastPagesUpdated(pageId);
        }
    }

    @PUT
    @Path("/{id}/status")
    public UpdateItemStatusResponse updateItemStatus(@PathParam("id") String id, UpdateItemStatusRequest req) {
        var item = itemService.updateItemStatus(UUID.fromString(id), req.status());
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        return new UpdateItemStatusResponse(DtoMapper.toDto(item));
    }

    @POST
    @Path("/move")
    public MoveItemsResponse moveItems(MoveItemsRequest req) {
        UUID afterItemId = (req.afterItemId() != null && !req.afterItemId().isEmpty())
                ? UUID.fromString(req.afterItemId()) : null;
        var items = itemService.moveItems(
                req.itemIds().stream().map(UUID::fromString).toList(),
                UUID.fromString(req.targetPageId()),
                afterItemId,
                req.levelDelta());
        sseBroadcaster.broadcastPagesUpdated(req.targetPageId());
        return new MoveItemsResponse(items.stream().map(DtoMapper::toDto).toList());
    }

    @GET
    public GetItemsByStatusResponse getItemsByStatus(@QueryParam("statuses") String statuses) {
        List<String> statusList = (statuses != null && !statuses.isEmpty())
                ? Arrays.asList(statuses.split(","))
                : List.of();
        var items = itemService.getItemsByStatuses(statusList);
        return new GetItemsByStatusResponse(items.stream().map(DtoMapper::toDto).toList());
    }

    @POST
    @Path("/export")
    public ExportTreeMarkdownResponse exportTreeMarkdown(ExportTreeMarkdownRequest req) {
        var result = itemService.exportTreeMarkdown(req.itemIds().stream().map(UUID::fromString).toList());
        return new ExportTreeMarkdownResponse(result.treeMarkdown(), result.html(), result.flatMarkdown());
    }

    @POST
    @Path("/import")
    public ImportTreeMarkdownResponse importTreeMarkdown(ImportTreeMarkdownRequest req) {
        String flatMd = itemService.importTreeMarkdown(req.treeMarkdown());
        return new ImportTreeMarkdownResponse(flatMd);
    }
}
