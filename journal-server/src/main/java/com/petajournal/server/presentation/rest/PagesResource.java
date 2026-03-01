package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.PageService;
import com.petajournal.server.domain.repository.PageRepository;
import com.petajournal.server.presentation.rest.dto.BacklinkEntryDto;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.PageDto;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.util.List;
import java.util.UUID;

@Path("/api/pages")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class PagesResource {

    @Inject
    PageService pageService;

    @Inject
    PageRepository pageRepository;

    @Inject
    SseBroadcaster sseBroadcaster;

    // --- Response records ---

    record GetPageByIdResponse(PageDto page, List<ItemDto> items) {}
    record GetOrCreateDocRequest(String title, String dirPath, List<String> tags) {}
    record GetOrCreateDocResponse(PageDto page, List<ItemDto> items, boolean created) {}
    record UpdatePageRequest(String title, List<String> tags, String dirPath) {}
    record UpdatePageResponse(PageDto page) {}
    record GetRecentPagesResponse(List<PageDto> pages) {}
    record GetBacklinksResponse(List<BacklinkEntryDto> entries) {}
    record GetChildPagesResponse(List<PageDto> pages) {}

    @GET
    @Path("/{id}")
    public GetPageByIdResponse getPageById(@PathParam("id") String id) {
        UUID pageId = UUID.fromString(id);
        var page = pageService.getPage(pageId)
                .orElseThrow(() -> new NotFoundException("Page not found: " + id));
        var items = pageService.getPageItems(pageId);
        return new GetPageByIdResponse(
                DtoMapper.toDto(page),
                items.stream().map(DtoMapper::toDto).toList());
    }

    @POST
    @Path("/doc")
    public GetOrCreateDocResponse getOrCreateDoc(GetOrCreateDocRequest req) {
        var result = pageService.getOrCreateDoc(req.title(), req.dirPath(), req.tags());
        return new GetOrCreateDocResponse(
                DtoMapper.toDto(result.page()),
                result.items().stream().map(DtoMapper::toDto).toList(),
                result.created());
    }

    @PUT
    @Path("/{id}")
    public UpdatePageResponse updatePage(@PathParam("id") String id, UpdatePageRequest req) {
        var page = pageService.updatePage(
                UUID.fromString(id),
                req.title(),
                req.tags(),
                req.dirPath());
        sseBroadcaster.broadcastPagesUpdated(id);
        return new UpdatePageResponse(DtoMapper.toDto(page));
    }

    @DELETE
    @Path("/{id}")
    public void deletePage(@PathParam("id") String id) {
        pageService.deletePage(UUID.fromString(id));
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
        var entries = pageService.getBacklinks(UUID.fromString(id));
        return new GetBacklinksResponse(entries.stream().map(DtoMapper::toDto).toList());
    }

    @GET
    @Path("/{id}/children")
    public GetChildPagesResponse getChildPages(@PathParam("id") String id) {
        var pages = pageRepository.findByParentId(UUID.fromString(id));
        return new GetChildPagesResponse(pages.stream().map(DtoMapper::toDto).toList());
    }
}
