package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.model.Item;
import com.petajournal.server.domain.repository.ItemRepository;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.infrastructure.persistence.PropertyRepository;
import com.petajournal.server.presentation.rest.dto.BacklinkEntryDto;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.PageDto;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import com.sosuisha.hmtid.HMTID;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.Set;

@Path("/api/pages")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class PagesResource {

    @Inject
    PageService pageService;

    @Inject
    PageRepository pageRepository;

    @Inject
    ItemRepository itemRepository;

    @Inject
    PropertyRepository propertyRepository;

    @Inject
    SseBroadcaster sseBroadcaster;

    // --- Response records ---

    record GetPageByIdResponse(PageDto page, List<ItemDto> items, Map<String, List<ItemDto>> transcludedTrees) {}
    record GetOrCreateDocRequest(String title, String dirPath) {}
    record GetOrCreateDocResponse(PageDto page, List<ItemDto> items, boolean created) {}
    record UpdatePageRequest(String title, String dirPath) {}
    record UpdatePageResponse(PageDto page) {}
    record GetRecentPagesResponse(List<PageDto> pages) {}
    record GetBacklinksResponse(List<BacklinkEntryDto> entries) {}
    record GetChildPagesResponse(List<PageDto> pages) {}

    private static final int MAX_TRANSCLUSION_DEPTH = 5;

    @GET
    @Path("/{id}")
    public GetPageByIdResponse getPageById(@PathParam("id") String id) {
        HMTID pageId = HMTID.of(id);
        var page = pageService.getPage(pageId)
                .orElseThrow(() -> new NotFoundException("Page not found: " + id));
        var items = pageService.getPageItems(pageId);

        // Build transcluded trees for items that have transclude_item_id
        Map<String, List<ItemDto>> transcludedTrees = new LinkedHashMap<>();
        Set<String> visited = new HashSet<>();
        for (Item item : items) {
            if (item.transcludeItemId() != null) {
                String sourceId = item.transcludeItemId().toString();
                if (!transcludedTrees.containsKey(sourceId)) {
                    loadTranscludedTree(sourceId, transcludedTrees, visited, 0);
                }
            }
        }

        return new GetPageByIdResponse(
                DtoMapper.toDto(page),
                loadItemDtos(items),
                transcludedTrees);
    }

    private void loadTranscludedTree(String sourceId, Map<String, List<ItemDto>> trees, Set<String> visited, int depth) {
        if (depth >= MAX_TRANSCLUSION_DEPTH || visited.contains(sourceId)) return;
        visited.add(sourceId);

        List<Item> descendants = itemRepository.findItemWithDescendants(HMTID.of(sourceId));
        if (descendants.isEmpty()) {
            trees.put(sourceId, List.of());
            return;
        }

        trees.put(sourceId, loadItemDtos(descendants));

        // Recursively load transcluded trees for nested transclusions
        for (Item desc : descendants) {
            if (desc.transcludeItemId() != null) {
                String nestedSourceId = desc.transcludeItemId().toString();
                if (!trees.containsKey(nestedSourceId)) {
                    loadTranscludedTree(nestedSourceId, trees, visited, depth + 1);
                }
            }
        }
    }

    @POST
    @Path("/doc")
    public GetOrCreateDocResponse getOrCreateDoc(GetOrCreateDocRequest req) {
        var result = pageService.getOrCreateDoc(req.title(), req.dirPath());
        return new GetOrCreateDocResponse(
                DtoMapper.toDto(result.page()),
                loadItemDtos(result.items()),
                result.created());
    }

    @PUT
    @Path("/{id}")
    public UpdatePageResponse updatePage(@PathParam("id") String id, UpdatePageRequest req) {
        var page = pageService.updatePage(
                HMTID.of(id),
                req.title(),
                req.dirPath());
        sseBroadcaster.broadcastPagesUpdated(id);
        return new UpdatePageResponse(DtoMapper.toDto(page));
    }

    @DELETE
    @Path("/{id}")
    public void deletePage(@PathParam("id") String id) {
        pageService.deletePage(HMTID.of(id));
        sseBroadcaster.broadcastPagesUpdated(id);
    }

    @GET
    @Path("/recent")
    public GetRecentPagesResponse getRecentPages(
            @QueryParam("maxResults") @DefaultValue("10") int maxResults,
            @QueryParam("type") String type) {
        String pageType = (type == null || type.isEmpty()) ? null : type;
        var pages = pageService.getRecentPages(maxResults, pageType);
        return new GetRecentPagesResponse(pages.stream().map(DtoMapper::toDto).toList());
    }

    @GET
    @Path("/{id}/backlinks")
    public GetBacklinksResponse getBacklinks(@PathParam("id") String id) {
        var entries = pageService.getBacklinks(HMTID.of(id));
        return new GetBacklinksResponse(entries.stream().map(DtoMapper::toDto).toList());
    }

    @GET
    @Path("/{id}/children")
    public GetChildPagesResponse getChildPages(@PathParam("id") String id) {
        var pages = pageRepository.findByParentId(HMTID.of(id));
        return new GetChildPagesResponse(pages.stream().map(DtoMapper::toDto).toList());
    }

    private List<ItemDto> loadItemDtos(List<Item> items) {
        List<String> ids = items.stream().map(i -> i.id().toString()).toList();
        var scalars = propertyRepository.findScalarByTargets("item", ids);
        var lists = propertyRepository.findListByTargets("item", ids);
        return DtoMapper.toDtos(items, scalars, lists);
    }
}
