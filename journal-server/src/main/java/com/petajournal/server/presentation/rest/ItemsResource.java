package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.FlatMarkdownSplitter;
import com.petajournal.server.application.ItemService;
import com.petajournal.server.domain.model.ItemPosition;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.FieldSelector;
import com.petajournal.server.presentation.rest.dto.ItemDtoAssembler;
import com.sosuisha.hmtid.HMTID;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import com.petajournal.server.domain.repository.ItemRepository;
import jakarta.ws.rs.core.Response;

import java.util.Arrays;
import java.util.List;
import java.util.Map;

@Path("/api/items")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class ItemsResource {

    private final ItemService itemService;
    private final FlatMarkdownSplitter flatMarkdownSplitter;
    private final ItemRepository itemRepository;
    private final PropertyRepository propertyRepository;
    private final ItemDtoAssembler dtos;
    private final SseBroadcaster sseBroadcaster;

    @Inject
    public ItemsResource(
            ItemService itemService,
            FlatMarkdownSplitter flatMarkdownSplitter,
            ItemRepository itemRepository,
            PropertyRepository propertyRepository,
            ItemDtoAssembler dtos,
            SseBroadcaster sseBroadcaster) {
        this.itemService = itemService;
        this.flatMarkdownSplitter = flatMarkdownSplitter;
        this.itemRepository = itemRepository;
        this.propertyRepository = propertyRepository;
        this.dtos = dtos;
        this.sseBroadcaster = sseBroadcaster;
    }

    // --- Request/Response records ---

    record CreateItemRequest(String pageId, String position, int level, String body, String id) {}
    record CreateItemResponse(ItemDto item) {}
    record UpdateItemRequest(String body, Integer level, Boolean collapsed) {}
    record UpdateItemResponse(ItemDto item) {}
    record MoveItemsRequest(List<String> itemIds, String targetPageId, String position, Integer levelDelta) {}
    record MoveItemsResponse(List<ItemDto> items) {}
    record ShiftLevelsRequest(List<String> itemIds, int delta) {}
    record ShiftLevelsResponse(List<ItemDto> items) {}
    record GetItemResponse(ItemDto item) {}
    record GetItemsByTypesResponse(List<ItemDto> items) {}
    record CycleTaskResponse(ItemDto item) {}
    record ExportTreeMarkdownRequest(List<String> itemIds) {}
    record ExportTreeMarkdownResponse(String treeMarkdown, String ast, String flatMarkdown) {}
    record ImportTreeMarkdownRequest(String treeMarkdown) {}
    record ImportTreeMarkdownResponse(String flatMarkdown) {}
    record ParseFlatMarkdownRequest(String markdown) {}
    record ParsedFlatItem(int level, String id, Map<String, String> properties, String body) {}
    record ParseFlatMarkdownResponse(List<ParsedFlatItem> items) {}
    record UpsertPropertyRequest(String valueType, String value) {}
    record UpsertPropertyResponse(ItemDto item) {}
    record SetTransclusionRequest(String sourceItemId) {}
    record SetTransclusionResponse(ItemDto item) {}

    @POST
    public CreateItemResponse createItem(CreateItemRequest req) {
        HMTID clientId = (req.id() != null && !req.id().isEmpty()) ? HMTID.of(req.id()) : null;
        ItemPosition position = ItemPosition.parse(req.position());
        var item = itemService.createItem(
                HMTID.of(req.pageId()),
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
        var dto = dtos.toDto(itemService.getItemById(HMTID.of(id)));
        if (fields != null && !fields.isEmpty()) {
            return Response.ok(FieldSelector.selectFields(dto, fields)).build();
        }
        return Response.ok(new GetItemResponse(dto)).build();
    }

    @PUT
    @Path("/{id}")
    public UpdateItemResponse updateItem(@PathParam("id") String id, UpdateItemRequest req) {
        var item = itemService.updateItem(
                HMTID.of(id),
                req.body(),
                req.level(),
                req.collapsed());
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        // Notify pages that transclude this item
        broadcastToTranscluders(HMTID.of(id), item.pageId().toString());
        return new UpdateItemResponse(DtoMapper.toDto(item));
    }

    @DELETE
    @Path("/{id}")
    public void deleteItem(@PathParam("id") String id) {
        // Find transcluders before deletion so we can notify their pages
        var transcluders = itemRepository.findTranscluders(HMTID.of(id));
        String pageId = itemService.getItemPageId(HMTID.of(id));
        itemService.deleteItem(HMTID.of(id));
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
            var item = itemService.setTransclusion(HMTID.of(id), HMTID.of(req.sourceItemId()));
            sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
            return Response.ok(new SetTransclusionResponse(dtos.toDto(item))).build();
        } catch (IllegalArgumentException e) {
            return Response.status(Response.Status.BAD_REQUEST)
                    .entity(Map.of("error", e.getMessage()))
                    .build();
        }
    }

    @DELETE
    @Path("/{id}/transclude")
    public Response removeTransclusion(@PathParam("id") String id) {
        var item = itemService.removeTransclusion(HMTID.of(id));
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        return Response.ok(new SetTransclusionResponse(dtos.toDto(item))).build();
    }

    @GET
    public GetItemsByTypesResponse getItemsByTypes(@QueryParam("types") String types) {
        List<String> typeList = (types != null && !types.isEmpty())
                ? Arrays.asList(types.split(","))
                : List.of();
        var items = itemService.getItemsByTypes(typeList);
        return new GetItemsByTypesResponse(dtos.toDtos(items));
    }

    @POST
    @Path("/{id}/cycle-task")
    public CycleTaskResponse cycleTask(@PathParam("id") String id) {
        var item = itemService.cycleTask(HMTID.of(id));
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        return new CycleTaskResponse(dtos.toDto(item));
    }

    @POST
    @Path("/move")
    public MoveItemsResponse moveItems(MoveItemsRequest req) {
        ItemPosition position = ItemPosition.parse(req.position());
        var items = itemService.moveItems(
                req.itemIds().stream().map(HMTID::of).toList(),
                HMTID.of(req.targetPageId()),
                position,
                req.levelDelta());
        sseBroadcaster.broadcastPagesUpdated(req.targetPageId());
        return new MoveItemsResponse(dtos.toDtos(items));
    }

    /**
     * Indents or outdents a group of items in one request.
     * The change is relative, so requests that arrive out of order still add up correctly.
     *
     * @param req the items and the amount to add to their levels
     * @return the items after the change
     * @throws IllegalArgumentException if itemIds is missing or empty, or an item is not found
     */
    @POST
    @Path("/shift-level")
    public ShiftLevelsResponse shiftLevels(ShiftLevelsRequest req) {
        if (req.itemIds() == null) {
            throw new IllegalArgumentException("itemIds is required");
        }
        List<HMTID> ids = req.itemIds().stream().map(HMTID::of).toList();
        var items = itemService.shiftLevels(ids, req.delta());
        items.stream().map(i -> i.pageId().toString()).distinct()
                .forEach(sseBroadcaster::broadcastPagesUpdated);
        for (var item : items) {
            broadcastToTranscluders(item.id(), item.pageId().toString());
        }
        return new ShiftLevelsResponse(dtos.toDtos(items));
    }

    @PUT
    @Path("/{id}/properties/{key}")
    public UpsertPropertyResponse upsertProperty(
            @PathParam("id") String id,
            @PathParam("key") String key,
            UpsertPropertyRequest req
    ) {
        var item = itemService.getItemById(HMTID.of(id));
        propertyRepository.upsertScalar("item", id, key, req.valueType(), req.value());
        sseBroadcaster.broadcastPagesUpdated(item.pageId().toString());
        // The DTO is built after the upsert, so it carries the new property
        return new UpsertPropertyResponse(dtos.toDto(item));
    }

    @POST
    @Path("/export")
    public ExportTreeMarkdownResponse exportTreeMarkdown(ExportTreeMarkdownRequest req) {
        return toResponse(itemService.exportTreeMarkdown(req.itemIds().stream().map(HMTID::of).toList()));
    }

    @POST
    @Path("/export-page")
    public ExportTreeMarkdownResponse exportPageMarkdown(@QueryParam("pageId") String pageId) {
        return toResponse(itemService.exportPageMarkdown(HMTID.of(pageId)));
    }

    @POST
    @Path("/export-subtree")
    public ExportTreeMarkdownResponse exportSubtreeMarkdown(@QueryParam("itemId") String itemId) {
        return toResponse(itemService.exportSubtreeMarkdown(HMTID.of(itemId)));
    }

    private static ExportTreeMarkdownResponse toResponse(ItemService.TreeMarkdownExport result) {
        return new ExportTreeMarkdownResponse(result.treeMarkdown(), result.ast(), result.flatMarkdown());
    }

    @POST
    @Path("/import")
    public ImportTreeMarkdownResponse importTreeMarkdown(ImportTreeMarkdownRequest req) {
        String flatMd = itemService.importTreeMarkdown(req.treeMarkdown());
        return new ImportTreeMarkdownResponse(flatMd);
    }

    /**
     * Splits Flat Markdown into items by the rules of the spec. Nothing is stored.
     * Bodies keep their GFM task prefix, so they can be passed to the create endpoint as-is.
     */
    @POST
    @Path("/parse")
    public ParseFlatMarkdownResponse parseFlatMarkdown(ParseFlatMarkdownRequest req) {
        List<ParsedFlatItem> items = flatMarkdownSplitter.split(req.markdown()).items().stream()
            .map(i -> new ParsedFlatItem(i.level(), i.id(), i.properties(), i.body()))
            .toList();
        return new ParseFlatMarkdownResponse(items);
    }

    private void broadcastToTranscluders(HMTID sourceItemId, String excludePageId) {
        for (var t : itemRepository.findTranscluders(sourceItemId)) {
            String tPageId = t.pageId().toString();
            if (!tPageId.equals(excludePageId)) {
                sseBroadcaster.broadcastPagesUpdated(tPageId);
            }
        }
    }
}
