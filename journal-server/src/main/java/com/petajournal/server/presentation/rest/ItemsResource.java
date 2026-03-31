package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.ItemService;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.model.ItemPosition;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.FieldSelector;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import com.petajournal.server.domain.repository.ItemRepository;
import jakarta.ws.rs.core.Response;

import java.util.Arrays;
import java.util.List;
import java.util.Map;
import java.util.UUID;

@Path("/api/items")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class ItemsResource {

    @Inject
    ItemService itemService;

    @Inject
    ItemRepository itemRepository;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    SseBroadcaster sseBroadcaster;

    // --- Request/Response records ---

    record CreateItemRequest(String pageId, String position, int level, String body, String id) {}
    record CreateItemResponse(ItemDto item) {}
    record UpdateItemRequest(String body, Integer level, Boolean collapsed) {}
    record UpdateItemResponse(ItemDto item) {}
    record MoveItemsRequest(List<String> itemIds, String targetPageId, String position, Integer levelDelta) {}
    record MoveItemsResponse(List<ItemDto> items) {}
    record GetItemResponse(ItemDto item) {}
    record GetItemsByTypesResponse(List<ItemDto> items) {}
    record CycleTaskResponse(ItemDto item) {}
    record ExportTreeMarkdownRequest(List<String> itemIds) {}
    record ExportTreeMarkdownResponse(String treeMarkdown, String ast, String flatMarkdown) {}
    record ImportTreeMarkdownRequest(String treeMarkdown) {}
    record ImportTreeMarkdownResponse(String flatMarkdown) {}
    record UpsertPropertyRequest(String valueType, String value) {}
    record UpsertPropertyResponse(ItemDto item) {}
    record SetTransclusionRequest(String sourceItemId) {}
    record SetTransclusionResponse(ItemDto item) {}

    @POST
    public CreateItemResponse createItem(CreateItemRequest req) {
        UUID clientId = (req.id() != null && !req.id().isEmpty()) ? UUID.fromString(req.id()) : null;
        ItemPosition position = ItemPosition.parse(req.position());
        var item = itemService.createItem(
                UUID.fromString(req.pageId()),
                position,
                req.level(),
                req.body(),
                clientId);
        sseBroadcaster.broadcastPagesUpdated(req.pageId());
        return new CreateItemResponse(DtoMapper.toDto(item));
    }

    @GET
    @Path("/{id}")
    public Response getItem(@PathParam("id") String id, @QueryParam("fields") String fields) {
        var item = itemService.getItemById(UUID.fromString(id));
        if (fields != null && !fields.isEmpty()) {
            var dto = loadItemDto(item);
            return Response.ok(FieldSelector.selectFields(dto, fields)).build();
        }
        return Response.ok(new GetItemResponse(loadItemDto(item))).build();
    }

    @PUT
    @Path("/{id}")
    public UpdateItemResponse updateItem(@PathParam("id") String id, UpdateItemRequest req) {
        var item = itemService.updateItem(
                UUID.fromString(id),
                req.body(),
                req.level(),
                req.collapsed());
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        // Notify pages that transclude this item
        broadcastToTranscluders(UUID.fromString(id), item.pageId().toString());
        return new UpdateItemResponse(DtoMapper.toDto(item));
    }

    @DELETE
    @Path("/{id}")
    public void deleteItem(@PathParam("id") String id) {
        // Find transcluders before deletion so we can notify their pages
        var transcluders = itemRepository.findTranscluders(UUID.fromString(id));
        String pageId = itemService.getItemPageId(UUID.fromString(id));
        itemService.deleteItem(UUID.fromString(id));
        if (pageId != null) {
            sseBroadcaster.broadcastPagesUpdated(pageId);
        }
        for (var t : transcluders) {
            sseBroadcaster.broadcastPagesUpdated(t.pageId().toString());
        }
    }

    @POST
    @Path("/{id}/transclude")
    public Response setTransclusion(@PathParam("id") String id, SetTransclusionRequest req) {
        try {
            var item = itemService.setTransclusion(UUID.fromString(id), UUID.fromString(req.sourceItemId()));
            sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
            return Response.ok(new SetTransclusionResponse(loadItemDto(item))).build();
        } catch (IllegalArgumentException e) {
            return Response.status(Response.Status.BAD_REQUEST)
                    .entity(Map.of("error", e.getMessage()))
                    .build();
        }
    }

    @DELETE
    @Path("/{id}/transclude")
    public Response removeTransclusion(@PathParam("id") String id) {
        var item = itemService.removeTransclusion(UUID.fromString(id));
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        return Response.ok(new SetTransclusionResponse(loadItemDto(item))).build();
    }

    @GET
    public GetItemsByTypesResponse getItemsByTypes(@QueryParam("types") String types) {
        List<String> typeList = (types != null && !types.isEmpty())
                ? Arrays.asList(types.split(","))
                : List.of();
        var items = itemService.getItemsByTypes(typeList);
        return new GetItemsByTypesResponse(loadItemDtos(items));
    }

    @POST
    @Path("/{id}/cycle-task")
    public CycleTaskResponse cycleTask(@PathParam("id") String id) {
        var item = itemService.cycleTask(UUID.fromString(id));
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        return new CycleTaskResponse(loadItemDto(item));
    }

    @POST
    @Path("/move")
    public MoveItemsResponse moveItems(MoveItemsRequest req) {
        ItemPosition position = ItemPosition.parse(req.position());
        var items = itemService.moveItems(
                req.itemIds().stream().map(UUID::fromString).toList(),
                UUID.fromString(req.targetPageId()),
                position,
                req.levelDelta());
        sseBroadcaster.broadcastPagesUpdated(req.targetPageId());
        return new MoveItemsResponse(loadItemDtos(items));
    }

    @PUT
    @Path("/{id}/properties/{key}")
    public UpsertPropertyResponse upsertProperty(
            @PathParam("id") String id,
            @PathParam("key") String key,
            UpsertPropertyRequest req
    ) {
        var item = itemService.getItemById(UUID.fromString(id));
        propertyRepository.upsertScalar("item", id, key, req.valueType(), req.value());
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        return new UpsertPropertyResponse(loadItemDto(item));
    }

    @POST
    @Path("/export")
    public ExportTreeMarkdownResponse exportTreeMarkdown(ExportTreeMarkdownRequest req) {
        var result = itemService.exportTreeMarkdown(req.itemIds().stream().map(UUID::fromString).toList());
        return new ExportTreeMarkdownResponse(result.treeMarkdown(), result.ast(), result.flatMarkdown());
    }

    @POST
    @Path("/export-page")
    public ExportTreeMarkdownResponse exportPageMarkdown(@QueryParam("pageId") String pageId) {
        var result = itemService.exportPageMarkdown(UUID.fromString(pageId));
        return new ExportTreeMarkdownResponse(result.treeMarkdown(), result.ast(), result.flatMarkdown());
    }

    @POST
    @Path("/export-subtree")
    public ExportTreeMarkdownResponse exportSubtreeMarkdown(@QueryParam("itemId") String itemId) {
        var result = itemService.exportSubtreeMarkdown(UUID.fromString(itemId));
        return new ExportTreeMarkdownResponse(result.treeMarkdown(), result.ast(), result.flatMarkdown());
    }

    @POST
    @Path("/import")
    public ImportTreeMarkdownResponse importTreeMarkdown(ImportTreeMarkdownRequest req) {
        String flatMd = itemService.importTreeMarkdown(req.treeMarkdown());
        return new ImportTreeMarkdownResponse(flatMd);
    }

    private ItemDto loadItemDto(Item item) {
        String id = item.id().toString();
        var scalars = propertyRepository.findScalarByTargets("item", List.of(id)).getOrDefault(id, Map.of());
        var lists = propertyRepository.findListByTargets("item", List.of(id)).getOrDefault(id, Map.of());
        return DtoMapper.toDto(item, scalars, lists);
    }

    private void broadcastToTranscluders(UUID sourceItemId, String excludePageId) {
        for (var t : itemRepository.findTranscluders(sourceItemId)) {
            String tPageId = t.pageId().toString();
            if (!tPageId.equals(excludePageId)) {
                sseBroadcaster.broadcastPagesUpdated(tPageId);
            }
        }
    }

    private List<ItemDto> loadItemDtos(List<Item> items) {
        List<String> ids = items.stream().map(i -> i.id().toString()).toList();
        var scalars = propertyRepository.findScalarByTargets("item", ids);
        var lists = propertyRepository.findListByTargets("item", ids);
        return DtoMapper.toDtos(items, scalars, lists);
    }
}
