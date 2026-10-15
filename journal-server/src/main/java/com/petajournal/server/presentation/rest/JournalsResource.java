package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.JournalService;
import com.petajournal.server.application.JournalService.MostRecentJournalResult;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.ItemDtoAssembler;
import com.petajournal.server.presentation.rest.dto.PageDto;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.time.LocalDate;
import java.util.List;
import java.util.Map;

@Path("/api/journals")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class JournalsResource {

    private final JournalService journalService;
    private final ItemDtoAssembler dtos;

    @Inject
    public JournalsResource(JournalService journalService, ItemDtoAssembler dtos) {
        this.journalService = journalService;
        this.dtos = dtos;
    }

    // --- Response records ---

    record GetOrCreateJournalResponse(PageDto page, List<ItemDto> items, boolean created, Map<String, List<ItemDto>> transcludedTrees) {}
    record GetMostRecentJournalBeforeResponse(boolean found, PageDto page, List<ItemDto> items, Map<String, List<ItemDto>> transcludedTrees) {}

    @POST
    @Path("/{name}")
    public GetOrCreateJournalResponse getOrCreateJournal(@PathParam("name") String name) {
        var result = journalService.getOrCreateJournalByName(name);
        var items = result.items();
        return new GetOrCreateJournalResponse(
                DtoMapper.toDto(result.page()),
                dtos.toDtos(items),
                result.created(),
                dtos.transcludedTrees(items));
    }

    @GET
    @Path("/before")
    public GetMostRecentJournalBeforeResponse getMostRecentJournalBefore(@QueryParam("date") String date) {
        LocalDate beforeDate = LocalDate.parse(date);
        return toResponse(journalService.getMostRecentJournalBefore(beforeDate));
    }

    @GET
    @Path("/weekly/before")
    public GetMostRecentJournalBeforeResponse getMostRecentWeeklyJournalBefore(@QueryParam("week") String week) {
        return toResponse(journalService.getMostRecentWeeklyJournalBefore(week));
    }

    private GetMostRecentJournalBeforeResponse toResponse(MostRecentJournalResult result) {
        if (!result.found()) {
            return new GetMostRecentJournalBeforeResponse(false, null, List.of(), Map.of());
        }
        var items = result.items();
        return new GetMostRecentJournalBeforeResponse(
                true,
                DtoMapper.toDto(result.page()),
                dtos.toDtos(items),
                dtos.transcludedTrees(items));
    }
}
