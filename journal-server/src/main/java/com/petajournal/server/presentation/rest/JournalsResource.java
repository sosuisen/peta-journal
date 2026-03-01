package com.petajournal.server.presentation.rest;

import com.petajournal.server.application.JournalService;
import com.petajournal.server.presentation.rest.dto.ItemDto;
import com.petajournal.server.presentation.rest.dto.DtoMapper;
import com.petajournal.server.presentation.rest.dto.PageDto;
import jakarta.inject.Inject;
import jakarta.ws.rs.*;
import jakarta.ws.rs.core.MediaType;

import java.time.LocalDate;
import java.util.List;

@Path("/api/journals")
@Produces(MediaType.APPLICATION_JSON)
@Consumes(MediaType.APPLICATION_JSON)
public class JournalsResource {

    @Inject
    JournalService journalService;

    // --- Response records ---

    record GetOrCreateJournalResponse(PageDto page, List<ItemDto> items, boolean created) {}
    record GetMostRecentJournalBeforeResponse(boolean found, PageDto page, List<ItemDto> items) {}

    @POST
    @Path("/{name}")
    public GetOrCreateJournalResponse getOrCreateJournal(@PathParam("name") String name) {
        var result = journalService.getOrCreateJournalByName(name);
        return new GetOrCreateJournalResponse(
                DtoMapper.toDto(result.page()),
                result.items().stream().map(DtoMapper::toDto).toList(),
                result.created());
    }

    @GET
    @Path("/before")
    public GetMostRecentJournalBeforeResponse getMostRecentJournalBefore(@QueryParam("date") String date) {
        LocalDate beforeDate = LocalDate.parse(date);
        var result = journalService.getMostRecentJournalBefore(beforeDate);
        if (!result.found()) {
            return new GetMostRecentJournalBeforeResponse(false, null, List.of());
        }
        return new GetMostRecentJournalBeforeResponse(
                true,
                DtoMapper.toDto(result.page()),
                result.items().stream().map(DtoMapper::toDto).toList());
    }

    @GET
    @Path("/weekly/before")
    public GetMostRecentJournalBeforeResponse getMostRecentWeeklyJournalBefore(@QueryParam("week") String week) {
        var result = journalService.getMostRecentWeeklyJournalBefore(week);
        if (!result.found()) {
            return new GetMostRecentJournalBeforeResponse(false, null, List.of());
        }
        return new GetMostRecentJournalBeforeResponse(
                true,
                DtoMapper.toDto(result.page()),
                result.items().stream().map(DtoMapper::toDto).toList());
    }
}
